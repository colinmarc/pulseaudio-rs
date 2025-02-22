use std::ffi::CString;
use std::sync::Arc;
use std::time;

use futures::channel::oneshot;
use futures::FutureExt as _;

use super::reactor::ReactorHandle;
use super::{ClientError, PlaybackSource, Result as ClientResult};
use crate::protocol;

/// A stream of audio data sent from the client to the server for playback in
/// a sink.
///
/// The stream handle can be freely cloned and shared between threads.
#[derive(Clone)]
pub struct PlaybackStream(Arc<InnerPlaybackStream>);

struct InnerPlaybackStream {
    handle: ReactorHandle,
    info: protocol::CreatePlaybackStreamReply,
    eof_notify: futures::future::Shared<oneshot::Receiver<()>>,
}

impl std::fmt::Debug for PlaybackStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PlaybackStream")
            .field(&self.0.info.channel)
            .finish()
    }
}

impl PlaybackStream {
    pub(super) async fn new(
        handle: ReactorHandle,
        params: protocol::PlaybackStreamParams,
        source: impl PlaybackSource,
    ) -> Result<Self, ClientError> {
        let (eof_tx, eof_rx) = oneshot::channel();
        let info = handle
            .insert_playback_stream(params, source, Some(eof_tx))
            .await?;

        Ok(Self(Arc::new(InnerPlaybackStream {
            handle,
            info,
            eof_notify: eof_rx.shared(),
        })))
    }

    /// The ID of the stream.
    pub fn channel(&self) -> u32 {
        self.0.info.channel
    }

    /// The attributes of the server-side buffer.
    pub fn buffer_attr(&self) -> &protocol::stream::BufferAttr {
        &self.0.info.buffer_attr
    }

    /// The sample specification for the stream. Can differ from the client's
    /// requested sample spec.
    pub fn sample_spec(&self) -> &protocol::SampleSpec {
        &self.0.info.sample_spec
    }

    /// The channel map for the stream.
    pub fn channel_map(&self) -> &protocol::ChannelMap {
        &self.0.info.channel_map
    }

    /// The sink the stream is connected to.
    pub fn sink(&self) -> u32 {
        self.0.info.sink_index
    }

    /// Sets the name of the playback stream.
    pub async fn set_name(&self, name: CString) -> ClientResult<()> {
        self.0
            .handle
            .roundtrip_ack(protocol::Command::SetPlaybackStreamName(
                protocol::SetStreamNameParams {
                    index: self.0.info.stream_index,
                    name,
                },
            ))
            .await
    }

    /// Fetches playback timing information for the playback stream.
    pub async fn timing_info(&self) -> ClientResult<protocol::PlaybackLatency> {
        self.0
            .handle
            .roundtrip_reply(protocol::Command::GetPlaybackLatency(
                protocol::LatencyParams {
                    channel: self.0.info.channel,
                    now: time::SystemTime::now(),
                },
            ))
            .await
    }

    /// Corks the playback stream (temporarily pausing playback).
    pub async fn cork(&self) -> ClientResult<()> {
        self.0
            .handle
            .roundtrip_ack(protocol::Command::CorkPlaybackStream(
                protocol::CorkStreamParams {
                    channel: self.0.info.channel,
                    cork: true,
                },
            ))
            .await
    }

    /// Uncorks the playback stream.
    pub async fn uncork(&self) -> ClientResult<()> {
        self.0
            .handle
            .roundtrip_ack(protocol::Command::CorkPlaybackStream(
                protocol::CorkStreamParams {
                    channel: self.0.info.channel,
                    cork: false,
                },
            ))
            .await
    }

    /// Returns a future that resolves when the stream's [AudioSource] has reached the end.
    pub async fn source_eof(&self) -> ClientResult<()> {
        self.0
            .eof_notify
            .clone()
            .await
            .map_err(|_| ClientError::Disconnected)
    }

    /// Waits until the given [AudioSource] has reached the end (and returns 0 in [AudioSource::poll_read]),
    /// and then instructs the server to drain the buffer before ending the stream.
    pub async fn play_all(&self) -> ClientResult<()> {
        self.source_eof().await?;
        self.drain().await?;
        Ok(())
    }

    /// Instructs the server to play any remaining data in the buffer, then end
    /// the stream. This method returns once the stream has finished.
    pub async fn drain(&self) -> ClientResult<()> {
        self.0
            .handle
            .mark_playback_stream_draining(self.0.info.channel);
        self.0
            .handle
            .roundtrip_ack(protocol::Command::DrainPlaybackStream(self.0.info.channel))
            .await
    }

    /// Instructs the server to discard any buffered data.
    pub async fn flush(&self) -> super::Result<()> {
        self.0
            .handle
            .roundtrip_ack(protocol::Command::FlushPlaybackStream(self.0.info.channel))
            .await
    }

    /// Deletes the stream from the server.
    pub async fn delete(self) -> ClientResult<()> {
        self.0
            .handle
            .delete_playback_stream(self.0.info.channel)
            .await
    }
}

impl Drop for InnerPlaybackStream {
    fn drop(&mut self) {
        // Sends the delete command to the server, but doesn't wait for the
        // response.
        let _ = self
            .handle
            .delete_playback_stream(self.info.channel)
            .now_or_never();
    }
}
