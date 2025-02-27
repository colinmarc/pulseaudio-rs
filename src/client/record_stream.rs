use std::{ffi::CString, sync::Arc, time};

use futures::{channel::oneshot, FutureExt as _};

use super::{reactor::ReactorHandle, ClientError, RecordSink, Result as ClientResult};
use crate::protocol;

/// A stream of audio data sent from the server to the client, originating from
/// a source.
///
/// The stream handle can be freely cloned and shared between threads.
#[derive(Clone)]
pub struct RecordStream(Arc<InnerRecordStream>);

struct InnerRecordStream {
    handle: ReactorHandle,
    info: protocol::CreateRecordStreamReply,
    start_notify: futures::future::Shared<oneshot::Receiver<()>>,
}

impl std::fmt::Debug for RecordStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RecordStream")
            .field(&self.0.info.channel)
            .finish()
    }
}

impl RecordStream {
    pub(super) async fn new(
        handle: ReactorHandle,
        params: protocol::RecordStreamParams,
        sink: impl RecordSink,
    ) -> Result<Self, ClientError> {
        let (start_tx, start_rx) = oneshot::channel();
        let info = handle
            .insert_record_stream(params, sink, Some(start_tx))
            .await?;

        Ok(Self(Arc::new(InnerRecordStream {
            handle,
            info,
            start_notify: start_rx.shared(),
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

    /// Sets the name of the record stream.
    pub async fn set_name(&self, name: CString) -> ClientResult<()> {
        self.0
            .handle
            .roundtrip_ack(protocol::Command::SetRecordStreamName(
                protocol::SetStreamNameParams {
                    index: self.0.info.stream_index,
                    name,
                },
            ))
            .await
    }

    /// Fetches record timing information for the record stream.
    pub async fn timing_info(&self) -> ClientResult<protocol::RecordLatency> {
        self.0
            .handle
            .roundtrip_reply(protocol::Command::GetRecordLatency(
                protocol::LatencyParams {
                    channel: self.0.info.channel,
                    now: time::SystemTime::now(),
                },
            ))
            .await
    }

    /// Corks the record stream (temporarily pausing recording).
    pub async fn cork(&self) -> ClientResult<()> {
        self.0
            .handle
            .roundtrip_ack(protocol::Command::CorkRecordStream(
                protocol::CorkStreamParams {
                    channel: self.0.info.channel,
                    cork: true,
                },
            ))
            .await
    }

    /// Uncorks the record stream.
    pub async fn uncork(&self) -> ClientResult<()> {
        self.0
            .handle
            .roundtrip_ack(protocol::Command::CorkRecordStream(
                protocol::CorkStreamParams {
                    channel: self.0.info.channel,
                    cork: false,
                },
            ))
            .await
    }

    /// Returns a future that resolves when the first bytes are written to
    /// the stream by the server.
    pub async fn started(&self) -> ClientResult<()> {
        self.0
            .start_notify
            .clone()
            .await
            .map_err(|_| ClientError::Disconnected)
    }

    /// Instructs the server to discard any buffered data.
    pub async fn flush(&self) -> super::Result<()> {
        self.0
            .handle
            .roundtrip_ack(protocol::Command::FlushRecordStream(self.0.info.channel))
            .await
    }

    /// Deletes the stream from the server.
    pub async fn delete(self) -> ClientResult<()> {
        self.0
            .handle
            .delete_record_stream(self.0.info.channel)
            .await
    }
}

impl Drop for InnerRecordStream {
    fn drop(&mut self) {
        // Sends the delete command to the server, but doesn't wait for the
        // response.
        let _ = self
            .handle
            .delete_record_stream(self.info.channel)
            .now_or_never();
    }
}
