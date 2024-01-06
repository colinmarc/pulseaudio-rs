//! A simple example that records audio from the default input.
//!
//! Run with:
//!     cargo run --example record /tmp/recording.wav

use std::{
    ffi::CString,
    fs::File,
    io::{BufReader, BufWriter, Read},
    os::unix::net::UnixStream,
    path::Path,
};

use anyhow::{bail, Context};
use byteorder::ReadBytesExt;
use pulseaudio::protocol;

pub fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <file>", args[0]);
        return Ok(());
    }

    let (mut sock, protocol_version) = connect_and_init().context("failed to initialize client")?;

    // Figure out the default spec.
    protocol::write_command_message(
        sock.get_mut(),
        10,
        protocol::Command::GetSourceInfo(protocol::GetSourceInfo {
            name: Some(CString::new("@DEFAULT_SOURCE@")?),
            ..Default::default()
        }),
        protocol_version,
    )?;

    let (_, source_info) =
        protocol::read_reply_message::<protocol::SourceInfo>(&mut sock, protocol_version)?;
    eprintln!(
        "recording from source: {:?}...",
        source_info.description.unwrap_or(source_info.name)
    );

    // Create the recording stream on the server.
    protocol::write_command_message(
        sock.get_mut(),
        99,
        protocol::Command::CreateRecordStream(protocol::RecordStreamParams {
            source_index: Some(source_info.index),
            sample_spec: protocol::SampleSpec {
                format: source_info.sample_spec.format,
                channels: source_info.channel_map.num_channels(),
                sample_rate: source_info.sample_spec.sample_rate,
            },
            channel_map: source_info.channel_map,
            cvolume: Some(protocol::ChannelVolume::norm(2)),
            ..Default::default()
        }),
        protocol_version,
    )?;

    let (_, record_stream) = protocol::read_reply_message::<protocol::CreateRecordStreamReply>(
        &mut sock,
        protocol_version,
    )?;

    // Create the output file.
    let (bits_per_sample, sample_format) = match record_stream.sample_spec.format {
        protocol::SampleFormat::S16Le => (16, hound::SampleFormat::Int),
        protocol::SampleFormat::Float32Le => (32, hound::SampleFormat::Float),
        protocol::SampleFormat::S32Le => (32, hound::SampleFormat::Int),
        _ => bail!(
            "unsupported sample format: {:?}",
            record_stream.sample_spec.format
        ),
    };

    let spec = hound::WavSpec {
        channels: record_stream.channel_map.num_channels() as u16,
        sample_rate: record_stream.sample_spec.sample_rate,
        bits_per_sample,
        sample_format,
    };

    let mut file = BufWriter::new(File::create(Path::new(&args[1]))?);
    let mut wav_writer = hound::WavWriter::new(&mut file, spec)?;

    eprintln!("stream: {:#?}", record_stream);

    // A reusable buffer.
    let mut buf = vec![0; record_stream.buffer_attr.fragment_size as usize];

    // Read messages from the server in a loop. In real code it would be more
    // efficient to poll the socket using `mio` or similar.
    loop {
        let desc = protocol::read_descriptor(&mut sock)?;

        // A channel of -1 is a command message. Everything else is data.
        if desc.channel == u32::MAX {
            let (_, msg) = protocol::Command::read_tag_prefixed(&mut sock, protocol_version)?;
            eprintln!("received command from server: {:#?}", msg);
        } else {
            eprintln!("got {} bytes of data", desc.length);

            buf.resize(desc.length as usize, 0);
            sock.read_exact(&mut buf)?;

            let mut cursor = std::io::Cursor::new(buf.as_slice());
            while cursor.position() < cursor.get_ref().len() as u64 {
                match record_stream.sample_spec.format {
                    protocol::SampleFormat::S16Le => {
                        wav_writer.write_sample(cursor.read_i16::<byteorder::LittleEndian>()?)?
                    }
                    protocol::SampleFormat::Float32Le => {
                        wav_writer.write_sample(cursor.read_f32::<byteorder::LittleEndian>()?)?
                    }
                    protocol::SampleFormat::S32Le => {
                        wav_writer.write_sample(cursor.read_i32::<byteorder::LittleEndian>()?)?
                    }
                    _ => unreachable!(),
                };
            }

            wav_writer.flush()?;
        }
    }
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

    let (_, auth_reply) =
        protocol::read_reply_message::<protocol::AuthReply>(&mut sock, protocol::MAX_VERSION)?;
    let protocol_version = std::cmp::min(protocol::MAX_VERSION, auth_reply.version);

    let mut props = protocol::Props::new();
    props.set(protocol::Prop::ApplicationName, "pulseaudio-rs-playback");
    protocol::write_command_message(
        sock.get_mut(),
        1,
        protocol::Command::SetClientName(props),
        protocol_version,
    )?;

    let _ =
        protocol::read_reply_message::<protocol::SetClientNameReply>(&mut sock, protocol_version)?;
    Ok((sock, protocol_version))
}
