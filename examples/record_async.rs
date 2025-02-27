//! An example using the higher-level [pulseaudio::Client] API with an async
//! runtime to record audio.
//!
//! Run with:
//!     cargo run --example record_async /tmp/recording.wav

use std::{
    fs::File,
    io::{self, BufWriter, Read},
    path::Path,
    time,
};

use anyhow::{bail, Context as _};
use futures::StreamExt as _;
use pulseaudio::{protocol, Client};
use tokio::sync::oneshot;
use tokio_util::{compat::FuturesAsyncReadCompatExt as _, io::ReaderStream};

// We're using tokio as a runtime here, but tokio is not a dependency of the
// crate, and it should be compatible with any executor.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <file>", args[0]);
        return Ok(());
    }

    // First, establish a connection to the PulseAudio server.
    let client = Client::from_env(c"test-record-rs").context("Failed to create client")?;

    // Determine the default stream format.
    let source_info = client
        .source_info_by_name(protocol::DEFAULT_SOURCE.to_owned())
        .await?;

    // Create a record stream on the server. This will negotiate the actual
    // format.
    let params = protocol::RecordStreamParams {
        source_index: Some(source_info.index),
        sample_spec: protocol::SampleSpec {
            format: source_info.sample_spec.format,
            channels: source_info.channel_map.num_channels(),
            sample_rate: source_info.sample_spec.sample_rate,
        },
        channel_map: source_info.channel_map,
        cvolume: Some(protocol::ChannelVolume::norm(2)),
        ..Default::default()
    };

    // Create a buffer that implements AsyncRead.
    let buffer = pulseaudio::RecordBuffer::new(1024 * 1024 * 1024);
    let stream = client
        .create_record_stream(params, buffer.as_record_sink())
        .await?;

    // Create the output file.
    let sample_spec = stream.sample_spec().clone();
    let (bits_per_sample, sample_format) = match sample_spec.format {
        protocol::SampleFormat::S16Le => (16, hound::SampleFormat::Int),
        protocol::SampleFormat::Float32Le => (32, hound::SampleFormat::Float),
        protocol::SampleFormat::S32Le => (32, hound::SampleFormat::Int),
        _ => bail!("unsupported sample format: {:?}", sample_spec.format),
    };

    let spec = hound::WavSpec {
        channels: stream.channel_map().num_channels() as u16,
        sample_rate: sample_spec.sample_rate,
        bits_per_sample,
        sample_format,
    };

    let file = BufWriter::new(File::create(Path::new(&args[1]))?);
    let mut wav_writer = hound::WavWriter::new(file, spec)?;

    let mut bytes = ReaderStream::new(buffer.compat());
    tokio::spawn(async move {
        while let Some(chunk) = bytes.next().await {
            write_chunk(&mut wav_writer, sample_spec.format, &chunk?)?;
        }

        Ok::<(), anyhow::Error>(())
    });

    // Wait for the stream to start.
    stream.started().await?;
    eprintln!("Recording... [press enter to finish]");

    // Wait for the user to press enter.
    read_stdin().await?;

    // If we quit now, we'll miss out on anything still in the server's buffer.
    // Instead, we can measure the stream latency and wait that long before
    // deleting the stream.
    //
    // To calculate the latency, we measure the difference between the
    // read/write offset on the buffer, and add the source's inherent latency.
    let timing_info = stream.timing_info().await?;
    let offset = (timing_info.write_offset as u64)
        .checked_sub(timing_info.read_offset as u64)
        .unwrap_or(0);
    let latency = time::Duration::from_micros(timing_info.source_usec)
        + sample_spec.bytes_to_duration(offset as usize);
    tokio::time::sleep(latency).await;

    stream.delete().await?;
    eprintln!("Saved recording to {}", args[1]);

    Ok(())
}

async fn read_stdin() -> io::Result<()> {
    let (done_tx, done_rx) = oneshot::channel();
    std::thread::spawn(|| {
        let mut buf = [0; 1];
        let _ = done_tx.send(std::io::stdin().read(&mut buf).map(|_| ()));
    });

    done_rx.await.unwrap()
}

fn write_chunk(
    wav_writer: &mut hound::WavWriter<BufWriter<File>>,
    format: protocol::SampleFormat,
    chunk: &[u8],
) -> anyhow::Result<()> {
    use byteorder::ReadBytesExt as _;

    let mut cursor = io::Cursor::new(chunk);
    while cursor.position() < cursor.get_ref().len() as u64 {
        match format {
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

    Ok(())
}
