use std::ffi::CString;

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

/// Sent by the server to indicate a change in buffer attributes for a record
/// stream. N.B. this is currently not implemented or used by the C server.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordBufferAttrChanged {
    /// The index of the stream.
    pub stream_index: u32,

    /// The new buffer attributes.
    pub buffer_attr: BufferAttr,

    /// The new source output latency, in microseconds.
    pub source_output_latency: u64,
}

impl TagStructRead for RecordBufferAttrChanged {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            stream_index: ts.read_u32()?,
            buffer_attr: BufferAttr {
                max_length: ts.read_u32()?,
                fragment_size: ts.read_u32()?,
                ..Default::default()
            },
            source_output_latency: ts.read_usec()?,
        })
    }
}

impl TagStructWrite for RecordBufferAttrChanged {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.stream_index)?;
        w.write_u32(self.buffer_attr.max_length)?;
        w.write_u32(self.buffer_attr.fragment_size)?;
        w.write_usec(self.source_output_latency)?;
        Ok(())
    }
}

/// Sent by the server to indicate a playback stream has been moved to a different device.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlaybackStreamMovedParams {
    /// The index of the stream.
    pub stream_index: u32,

    /// The index of the new device.
    pub device_index: u32,

    /// The name of the new device.
    pub device_name: CString,

    /// Whether the the destination device is suspended.
    pub device_suspended: bool,

    /// The buffer attributes of the stream. `fragment_size` is ignored.
    pub buffer_attr: BufferAttr,

    /// The configured sink latency, in microseconds.
    pub configured_sink_latency: u64,
}

impl TagStructRead for PlaybackStreamMovedParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            stream_index: ts
                .read_index()?
                .ok_or(ProtocolError::Invalid("invalid stream index".to_string()))?,
            device_index: ts
                .read_index()?
                .ok_or(ProtocolError::Invalid("invalid device index".to_string()))?,
            device_name: ts.read_string_non_null()?,
            device_suspended: ts.read_bool()?,
            buffer_attr: BufferAttr {
                max_length: ts.read_u32()?,
                target_length: ts.read_u32()?,
                pre_buffering: ts.read_u32()?,
                minimum_request_length: ts.read_u32()?,
                ..Default::default()
            },
            configured_sink_latency: ts.read_usec()?,
        })
    }
}

impl TagStructWrite for PlaybackStreamMovedParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(Some(self.stream_index))?;
        ts.write_index(Some(self.device_index))?;
        ts.write_string(Some(&self.device_name))?;
        ts.write_bool(self.device_suspended)?;
        ts.write_u32(self.buffer_attr.max_length)?;
        ts.write_u32(self.buffer_attr.target_length)?;
        ts.write_u32(self.buffer_attr.pre_buffering)?;
        ts.write_u32(self.buffer_attr.minimum_request_length)?;
        ts.write_usec(self.configured_sink_latency)?;
        Ok(())
    }
}

/// Sent by the server to indicate a playback stream has been moved to a different device.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RecordStreamMovedParams {
    /// The index of the stream.
    pub stream_index: u32,

    /// The index of the new device.
    pub device_index: u32,

    /// The name of the new device.
    pub device_name: CString,

    /// Whether the the destination device is suspended.
    pub device_suspended: bool,

    /// The buffer attributes of the stream. Only `fragment_size` and `max_length` are used.
    pub buffer_attr: BufferAttr,

    /// The configured source latency, in microseconds.
    pub configured_source_latency: u64,
}

impl TagStructRead for RecordStreamMovedParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            stream_index: ts
                .read_index()?
                .ok_or(ProtocolError::Invalid("invalid stream index".to_string()))?,
            device_index: ts
                .read_index()?
                .ok_or(ProtocolError::Invalid("invalid device index".to_string()))?,
            device_name: ts.read_string_non_null()?,
            device_suspended: ts.read_bool()?,
            buffer_attr: BufferAttr {
                max_length: ts.read_u32()?,
                fragment_size: ts.read_u32()?,
                ..Default::default()
            },
            configured_source_latency: ts.read_usec()?,
        })
    }
}

impl TagStructWrite for RecordStreamMovedParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(Some(self.stream_index))?;
        ts.write_index(Some(self.device_index))?;
        ts.write_string(Some(&self.device_name))?;
        ts.write_bool(self.device_suspended)?;
        ts.write_u32(self.buffer_attr.max_length)?;
        ts.write_u32(self.buffer_attr.fragment_size)?;
        ts.write_usec(self.configured_source_latency)?;
        Ok(())
    }
}

/// Sent by the server to indicate a stream has been suspended or resumed.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct StreamSuspendedParams {
    /// The index of the stream.
    pub stream_index: u32,

    /// The suspended state of the stream.
    pub suspended: bool,
}

impl TagStructRead for StreamSuspendedParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            stream_index: ts
                .read_index()?
                .ok_or_else(|| ProtocolError::Invalid("invalid stream index".into()))?,
            suspended: ts.read_bool()?,
        })
    }
}

impl TagStructWrite for StreamSuspendedParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(Some(self.stream_index))?;
        ts.write_bool(self.suspended)?;
        Ok(())
    }
}

/// A stream event with a string and proplist attached.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GenericStreamEvent {
    /// The index of the stream.
    pub stream_index: u32,

    /// The event name.
    pub event_name: CString,

    /// The event properties.
    pub event_properties: Props,
}

impl TagStructRead for GenericStreamEvent {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            stream_index: ts
                .read_index()?
                .ok_or_else(|| ProtocolError::Invalid("invalid stream index".into()))?,
            event_name: ts.read_string_non_null()?,
            event_properties: ts.read()?,
        })
    }
}

impl TagStructWrite for GenericStreamEvent {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(Some(self.stream_index))?;
        ts.write_string(Some(&self.event_name))?;
        ts.write(&self.event_properties)?;
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

    #[test]
    fn record_buffer_attr_changed_serde() -> anyhow::Result<()> {
        let ev = RecordBufferAttrChanged {
            stream_index: 1,
            buffer_attr: Default::default(),
            source_output_latency: 2,
        };

        test_serde_version(&ev, MAX_VERSION)
    }

    #[test]
    fn playback_stream_moved_params_serde() -> anyhow::Result<()> {
        let ev = PlaybackStreamMovedParams {
            stream_index: 1,
            device_index: 2,
            device_name: CString::new("foo").unwrap(),
            device_suspended: false,
            buffer_attr: Default::default(),
            configured_sink_latency: 3000,
        };

        test_serde_version(&ev, MAX_VERSION)
    }

    #[test]
    fn record_stream_moved_params_serde() -> anyhow::Result<()> {
        let ev = RecordStreamMovedParams {
            stream_index: 1,
            device_index: 2,
            device_name: CString::new("foo").unwrap(),
            device_suspended: false,
            buffer_attr: Default::default(),
            configured_source_latency: 3000,
        };

        test_serde_version(&ev, MAX_VERSION)
    }

    #[test]
    fn test_stream_suspended_params_serde() -> anyhow::Result<()> {
        let params = StreamSuspendedParams {
            stream_index: 0,
            suspended: true,
        };

        test_serde(&params)
    }

    #[test]
    fn test_generic_stream_event_serde() -> anyhow::Result<()> {
        let params = GenericStreamEvent {
            stream_index: 0,
            event_name: CString::new("event").unwrap(),
            event_properties: Props::new(),
        };

        test_serde(&params)
    }
}
