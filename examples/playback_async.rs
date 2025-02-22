//! An example of using the higher-level [pulseaudio::Client] API to play audio
//! with an async runtime.
//!
//! Run with:
//!    cargo run --example playback -- testfiles/victory.wav

use std::{fs::File, io, path::Path, time};

use anyhow::{bail, Context as _};
use pulseaudio::{protocol, AsPlaybackSource, Client, PlaybackStream};

// We're using tokio as a runtime here, but tokio is not a dependency of the
// crate, and it should be compatible with any executor.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <file>", args[0]);
        return Ok(());
    }

    // Load the audio file, and choose parameters for the playback stream based
    // on the format of the audio. We only support 16bit integer PCM in this
    // example.
    let file = File::open(Path::new(&args[1]))?;
    let mut wav_reader = hound::WavReader::new(file)?;
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

    let params = protocol::PlaybackStreamParams {
        sample_spec: protocol::SampleSpec {
            format,
            channels: spec.channels as u8,
            sample_rate: spec.sample_rate,
        },
        channel_map,
        cvolume: Some(protocol::ChannelVolume::norm(2)),
        sink_name: Some(protocol::DEFAULT_SINK.to_owned()),
        ..Default::default()
    };

    // First, establish a connection to the PulseAudio server.
    let client = Client::from_env(c"test-playback-rs").context("Failed to create client")?;

    // Create a callback function, which is called by the client to write data
    // to the stream.
    let callback = move |data: &mut [u8]| copy_chunk(&mut wav_reader, data);

    let stream = client
        .create_playback_stream(params, callback.as_playback_source())
        .await
        .context("Failed to create playback stream")?;

    // Update our progress bar in a loop while waiting for the stream to finish.
    tokio::select! {
        res = stream.play_all() => res.context("Failed to play stream")?,
        _ = async {
            loop {
                if let Err(err) = update_progress(stream.clone(), pb.clone()).await {
                    eprintln!("Failed to update progress: {}", err);
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        } => (),
    }

    Ok(())
}

async fn update_progress(
    stream: PlaybackStream,
    pb: indicatif::ProgressBar,
) -> Result<(), pulseaudio::ClientError> {
    let timing_info = stream.timing_info().await?;

    // Use the information from the server to display the current playback latency.
    let latency = time::Duration::from_micros(timing_info.sink_usec + timing_info.source_usec);

    pb.set_message(format!("{}ms latency", latency.as_millis()));

    // The playback position is the server's offset into the buffer.
    // We'll use that to update the progress bar.
    pb.set_position(timing_info.read_offset as u64);
    Ok(())
}

fn copy_chunk<T: io::Read>(wav_reader: &mut hound::WavReader<T>, buf: &mut [u8]) -> usize {
    use byteorder::WriteBytesExt;
    let len = buf.len();
    assert!(len % 2 == 0);

    let mut cursor = std::io::Cursor::new(buf);
    for sample in wav_reader.samples::<i16>().filter_map(Result::ok) {
        if cursor.write_i16::<byteorder::LittleEndian>(sample).is_err() {
            break;
        }

        if cursor.position() == len as u64 {
            break;
        }
    }

    cursor.position() as usize
}
