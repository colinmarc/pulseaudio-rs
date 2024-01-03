use crate::protocol::serde::*;
use crate::protocol::stream::BufferAttr;
use crate::protocol::ProtocolError;

/// Sent by the server to request a chunk from a playback stream.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    /// The channel ID.
    pub channel: u32,

    /// The number of bytes requested.
    pub length: u32,
}

impl TagStructRead for Request {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            channel: ts.read_u32()?,
            length: ts.read_u32()?,
        })
    }
}

impl TagStructWrite for Request {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.channel)?;
        w.write_u32(self.length)?;
        Ok(())
    }
}

/// Sent by the server to indicate an underflow in a playback stream.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Underflow {
    /// The channel ID.
    pub channel: u32,

    /// The offset where the underflow occurred.
    pub offset: i64,
}

impl TagStructRead for Underflow {
    fn read(ts: &mut TagStructReader<'_>, protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            channel: ts.read_u32()?,
            offset: if protocol_version >= 23 {
                ts.read_i64()?
            } else {
                0
            },
        })
    }
}

impl TagStructWrite for Underflow {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.channel)?;

        if protocol_version >= 23 {
            w.write_i64(self.offset)?;
        }

        Ok(())
    }
}

/// Sent by the server to indicate a change in buffer attributes for a playback stream.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackBufferAttrChanged {
    /// The index of the stream.
    pub stream_index: u32,

    /// The new buffer attributes.
    pub buffer_attr: BufferAttr,

    /// The new sink input latency, in microseconds.
    pub sink_input_latency: u64,
}

impl TagStructRead for PlaybackBufferAttrChanged {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            stream_index: ts.read_u32()?,
            buffer_attr: BufferAttr {
                max_length: ts.read_u32()?,
                target_length: ts.read_u32()?,
                pre_buffering: ts.read_u32()?,
                minimum_request_length: ts.read_u32()?,
                ..Default::default()
            },
            sink_input_latency: ts.read_usec()?,
        })
    }
}

impl TagStructWrite for PlaybackBufferAttrChanged {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.stream_index)?;
        w.write_u32(self.buffer_attr.max_length)?;
        w.write_u32(self.buffer_attr.target_length)?;
        w.write_u32(self.buffer_attr.pre_buffering)?;
        w.write_u32(self.buffer_attr.minimum_request_length)?;
        w.write_usec(self.sink_input_latency)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::MAX_VERSION;

    use super::test_util::{test_serde, test_serde_version};
    use super::*;

    #[test]
    fn request_serde() -> anyhow::Result<()> {
        let ev = Request {
            channel: 1,
            length: 2,
        };

        test_serde_version(&ev, MAX_VERSION)
    }

    #[test]
    fn underflow_serde() -> anyhow::Result<()> {
        let ev = Underflow {
            channel: 1,
            offset: 0,
        };

        test_serde(&ev)
    }

    #[test]
    fn playback_buffer_attr_changed_serde() -> anyhow::Result<()> {
        let ev = PlaybackBufferAttrChanged {
            stream_index: 1,
            buffer_attr: Default::default(),
            sink_input_latency: 2,
        };

        test_serde_version(&ev, MAX_VERSION)
    }
}
