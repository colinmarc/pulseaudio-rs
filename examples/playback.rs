// To run this example, run the following command:
//    cargo run --example playback -- testfiles/victory.wav

use std::{
    ffi::CString,
    fs::File,
    io::{BufReader, Read},
    os::unix::net::UnixStream,
    path::Path,
    time,
};

use anyhow::{bail, Context};
use pulseaudio::protocol;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <file>", args[0]);
        return Ok(());
    }

    let (mut sock, protocol_version) = connect_and_init().context("failed to initialize client")?;

    let mut file = File::open(Path::new(&args[1]))?;
    let mut wav_reader = hound::WavReader::new(&mut file)?;
    let spec = wav_reader.spec();

    let format = match (spec.bits_per_sample, spec.sample_format) {
        (16, hound::SampleFormat::Int) => protocol::SampleFormat::S16Le,
        _ => bail!(
            "unsupported sample format: {}bit {:?}",
            spec.bits_per_sample,
            spec.sample_format,
        ),
    };

    let channel_map = match spec.channels {
        1 => protocol::ChannelMap::mono(),
        2 => protocol::ChannelMap::stereo(),
        _ => bail!("unsupported channel count: {}", spec.channels),
    };

    // Set up a progress bar for displaying during playback.
    let file_duration =
        time::Duration::from_secs(wav_reader.duration() as u64 / spec.sample_rate as u64);
    let file_bytes =
        wav_reader.duration() as u64 * (spec.channels * spec.bits_per_sample / 8) as u64;
    let pb = indicatif::ProgressBar::new(file_bytes)
        .with_style(indicatif::ProgressStyle::with_template(&format!(
            "[{{elapsed_precise}} / {}] {{bar}} {{msg}}",
            indicatif::FormattedDuration(file_duration)
        ))?)
        .with_finish(indicatif::ProgressFinish::AndLeave);

    // Create the playback stream on the server.
    protocol::write_command_message(
        sock.get_mut(),
        99,
        protocol::Command::CreatePlaybackStream(protocol::PlaybackStreamParams {
            sample_spec: protocol::SampleSpec {
                format,
                channels: spec.channels as u8,
                sample_rate: spec.sample_rate,
            },
            channel_map,
            cvolume: Some(protocol::ChannelVolume::norm(2)),
            sink_name: Some(CString::new("@DEFAULT_SINK@")?),
            ..Default::default()
        }),
        protocol_version,
    )
    .context("failed to send create_playback_stream")?;

    let (seq, stream_info) =
        protocol::read_reply_message::<protocol::CreatePlaybackStreamReply>(&mut sock)
            .context("create_playback_stream failed")?;
    assert_eq!(seq, 99);

    // Create a buffer for sending data to the server.
    let mut buf = vec![0u8; stream_info.buffer_attr.minimum_request_length as usize];

    // The response has a field, requested_bytes, which tells us how many
    // bytes we should send right away.
    let size = read_chunk(
        &mut wav_reader,
        &mut buf,
        stream_info.requested_bytes as u64,
    )?;

    // Send initial bytes to the server.
    protocol::write_memblock(sock.get_mut(), stream_info.channel, &buf[..size], 0)
        .context("write_memblock failed")?;

    // PulseAudio uses tags to associate commands with replies. We can use a
    // token to know which kind of reply we're getting.
    const TIMING_INFO: u32 = 200;
    const DRAIN_COMPLETED: u32 = 201;

    // We'll read from the socket in a loop. Real code would probably use something like `mio`
    // to poll the socket.
    let mut draining = false;
    loop {
        let (seq, msg) = protocol::read_command_message(&mut sock, protocol_version)
            .context("reading from socket")?;

        match msg {
            // First, the server will indicate when the stream has started.
            protocol::Command::Started(_) => pb.reset_elapsed(),

            // PulseAudio streams are "clocked" by the server. That means we
            // should wait for the server to request bytes before sending more.
            protocol::Command::Request(protocol::Request { channel, length }) => {
                if channel != stream_info.channel {
                    bail!("unexpected channel: {}", channel);
                }

                if !draining {
                    let size = read_chunk(&mut wav_reader, &mut buf, length as u64)?;
                    if size > 0 {
                        protocol::write_memblock(
                            sock.get_mut(),
                            stream_info.channel,
                            &buf[..size],
                            0,
                        )
                        .context("write_memblock failed")?;
                    } else {
                        // Tell the server we're done sending data.
                        protocol::write_command_message(
                            sock.get_mut(),
                            DRAIN_COMPLETED,
                            protocol::Command::DrainPlaybackStream(stream_info.channel),
                            protocol_version,
                        )?;

                        draining = true;
                    }
                }

                // Fetch the current timing information for the stream.
                protocol::write_command_message(
                    sock.get_mut(),
                    TIMING_INFO,
                    protocol::Command::GetPlaybackLatency(protocol::LatencyParams {
                        channel,
                        now: time::SystemTime::now(),
                    }),
                    protocol_version,
                )?;
            }

            // This is a response to the timing info query we fired off just.
            // The format of the reply depends on the command that we sent,
            // so the library can't parse it for us -- so we parse it here.
            protocol::Command::Reply if seq == TIMING_INFO => {
                let mut ts =
                    protocol::serde::TagStructReader::new(&mut sock, protocol::MAX_VERSION);
                let timing_info = ts.read::<protocol::PlaybackLatency>()?;

                // The response includes information that allows us to estimate playback latency.
                let latency =
                    time::Duration::from_micros(timing_info.sink_usec + timing_info.source_usec);
                pb.set_message(format!("{}ms latency", latency.as_millis()));

                // The playback position is the server's offset into the buffer,
                // not the amount of data we've transmitted. We'll use that to
                // update the progress bar.
                pb.set_position(timing_info.read_offset as u64)
            }

            // This is a response to the DrainPlaybackStream command, which the
            // server waits to send until draining is finished. There's no
            // response payload.
            protocol::Command::Reply if seq == DRAIN_COMPLETED => break,

            // These are notifications that something went wrong.
            protocol::Command::Underflow(_) => bail!("buffer underrun!"),
            protocol::Command::Overflow(_) => bail!("buffer overrun!"),

            // We ignore all other messages.
            _ => (),
        }
    }

    Ok(())
}

fn read_chunk<T: Read>(
    wav_reader: &mut hound::WavReader<T>,
    buf: &mut Vec<u8>,
    target_length: u64,
) -> anyhow::Result<usize> {
    use byteorder::WriteBytesExt;

    if target_length > buf.len() as u64 {
        buf.resize(target_length as usize, 0);
    }

    let mut cursor = std::io::Cursor::new(buf);
    for sample in wav_reader.samples::<i16>() {
        cursor.write_i16::<byteorder::LittleEndian>(sample?)?;
        if cursor.position() >= target_length {
            break;
        }
    }

    Ok(cursor.position() as usize)
}

fn connect_and_init() -> anyhow::Result<(BufReader<UnixStream>, u16)> {
    let socket_path = pulseaudio::socket_path_from_env().context("PulseAudio not available")?;
    let mut sock = std::io::BufReader::new(UnixStream::connect(socket_path)?);

    let cookie = pulseaudio::cookie_path_from_env()
        .and_then(|path| std::fs::read(path).ok())
        .unwrap_or_default();
    let auth = protocol::AuthParams {
        version: protocol::MAX_VERSION,
        supports_shm: false,
        supports_memfd: false,
        cookie,
    };

    protocol::write_command_message(
        sock.get_mut(),
        0,
        protocol::Command::Auth(auth),
        protocol::MAX_VERSION,
    )?;

    let (_, auth_reply) = protocol::read_reply_message::<protocol::AuthReply>(&mut sock)?;
    let protocol_version = std::cmp::min(protocol::MAX_VERSION, auth_reply.version);

    let mut props = protocol::Props::new();
    props.set(protocol::Prop::ApplicationName, "pulseaudio-rs-playback");
    protocol::write_command_message(
        sock.get_mut(),
        1,
        protocol::Command::SetClientName(props),
        protocol_version,
    )?;

    let _ = protocol::read_reply_message::<protocol::SetClientNameReply>(&mut sock)?;
    Ok((sock, protocol_version))
}
