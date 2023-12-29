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
use pulseaudio::protocol::{self, LatencyParams};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <file>", args[0]);
        return Ok(());
    }

    let mut sock = connect_and_init().context("failed to initialize client")?;

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

    loop {
        let (_, msg) = protocol::read_command_message(&mut sock)?;
        match msg {
            // First, the server will indicate when the stream has started.
            protocol::Command::Started(_) => pb.reset_elapsed(),
            // The server will repeatedly send us a request for more bytes.
            protocol::Command::Request(protocol::Request { channel, length }) => {
                if channel != stream_info.channel {
                    bail!("unexpected channel: {}", channel);
                }

                let size = read_chunk(&mut wav_reader, &mut buf, length as u64)?;
                if size == 0 {
                    break;
                }

                protocol::write_memblock(sock.get_mut(), stream_info.channel, &buf[..size], 0)
                    .context("write_memblock failed")?;

                // Fetch the current timing information for the stream.
                let timing_info = get_timing_info(&mut sock, stream_info.channel)?;
                let latency =
                    time::Duration::from_micros(timing_info.sink_usec + timing_info.source_usec);
                pb.set_message(format!("{}ms latency", latency.as_millis()));
                pb.set_position(timing_info.read_offset as u64)
            }
            protocol::Command::Underflow(_) => bail!("buffer underrun!"),
            protocol::Command::Overflow(_) => bail!("buffer overrun!"),
            _ => (),
        }
    }

    // Tell the server we're done sending data.
    protocol::write_command_message(
        sock.get_mut(),
        101,
        protocol::Command::DrainPlaybackStream(stream_info.channel),
    )?;

    // Wait for the server to acknowledge that we're done.
    loop {
        let (seq, msg) = protocol::read_command_message(&mut sock)?;
        match msg {
            protocol::Command::Reply if seq == 101 => {
                assert_eq!(seq, 101);
                break;
            }
            _ => {
                let timing_info = get_timing_info(&mut sock, stream_info.channel)?;
                let latency =
                    time::Duration::from_micros(timing_info.sink_usec + timing_info.source_usec);
                pb.set_message(format!("{}ms latency", latency.as_millis()));
                pb.set_position(timing_info.read_offset as u64)
            }
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

fn connect_and_init() -> anyhow::Result<BufReader<UnixStream>> {
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR")?;
    let socket_path = Path::new(&xdg_runtime_dir).join("pulse/native");
    if !socket_path.exists() {
        bail!(
            "pulseaudio socket not found at {}",
            socket_path.to_string_lossy()
        );
    }

    let mut sock = std::io::BufReader::new(UnixStream::connect(&socket_path)?);

    let home = std::env::var("HOME")?;
    let cookie_path = Path::new(&home).join(".config/pulse/cookie");
    let auth = if cookie_path.exists() {
        let cookie = std::fs::read(&cookie_path)?;
        protocol::AuthParams {
            version: protocol::MAX_VERSION,
            supports_shm: false,
            supports_memfd: false,
            cookie,
        }
    } else {
        protocol::AuthParams {
            version: protocol::MAX_VERSION,
            supports_shm: false,
            supports_memfd: false,
            cookie: Vec::new(),
        }
    };

    protocol::write_command_message(sock.get_mut(), 0, protocol::Command::Auth(auth))?;
    let _ = protocol::read_reply_message::<protocol::AuthReply>(&mut sock)?;

    let mut props = protocol::Props::new();
    props.set(protocol::Prop::ApplicationName, "pulseaudio-rs-playback");
    protocol::write_command_message(sock.get_mut(), 1, protocol::Command::SetClientName(props))?;
    let _ = protocol::read_reply_message::<protocol::SetClientNameReply>(&mut sock)?;

    Ok(sock)
}

fn get_timing_info(
    sock: &mut BufReader<UnixStream>,
    channel: u32,
) -> anyhow::Result<protocol::PlaybackLatency> {
    protocol::write_command_message(
        sock.get_mut(),
        100,
        protocol::Command::GetPlaybackLatency(LatencyParams {
            channel,
            now: time::SystemTime::now(),
        }),
    )?;

    let (_, latency) = protocol::read_reply_message::<protocol::PlaybackLatency>(sock)?;
    Ok(latency)
}
