use crate::protocol::serde::stream::{BufferAttr, StreamFlags};
use crate::protocol::{serde::*, ProtocolError};
use crate::protocol::{ChannelMap, ChannelVolume, Props, SampleSpec};

use std::ffi::CString;

use super::CommandReply;

/// Parameters for [`super::Command::CreateRecordStream`].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct RecordStreamParams {
    /// Sample format for the stream.
    pub sample_spec: SampleSpec,

    /// Channel map for the stream.
    pub channel_map: ChannelMap,

    /// Index of the source to connect to.
    pub source_index: Option<u32>,

    /// Name of the source to connect to. Ignored if `source_index` is set.
    pub source_name: Option<CString>,

    /// Buffer attributes for the stream.
    pub buffer_attr: BufferAttr,

    /// Stream flags.
    pub flags: StreamFlags,

    // FIXME: I don't know what this is for.
    #[allow(missing_docs)]
    pub direct_on_input: bool,

    /// Volume of the stream.
    pub cvolume: Option<ChannelVolume>,

    /// Additional properties for the stream.
    pub props: Props,

    /// Formats the client offers.
    pub formats: Vec<FormatInfo>,
}

impl TagStructRead for RecordStreamParams {
    fn read(ts: &mut TagStructReader<'_>, protocol_version: u16) -> Result<Self, ProtocolError> {
        let sample_spec = ts.read()?;
        let channel_map = ts.read()?;
        let source_index = ts.read_index()?;
        let source_name = ts.read_string()?;

        let buffer_attr_max_length = ts.read_u32()?;

        let mut flags = StreamFlags {
            start_corked: ts.read_bool()?,
            ..Default::default()
        };

        let buffer_attr = BufferAttr {
            max_length: buffer_attr_max_length,
            fragment_size: ts.read_u32()?,
            ..Default::default()
        };

        flags.no_remap_channels = ts.read_bool()?;
        flags.no_remix_channels = ts.read_bool()?;
        flags.fix_format = ts.read_bool()?;
        flags.fix_rate = ts.read_bool()?;
        flags.fix_channels = ts.read_bool()?;
        flags.no_move = ts.read_bool()?;
        flags.variable_rate = ts.read_bool()?;

        flags.peak_detect = ts.read_bool()?;
        flags.adjust_latency = ts.read_bool()?;
        let props = ts.read()?;

        let direct_on_input = ts.read_bool()?;

        let mut params = Self {
            sample_spec,
            channel_map,
            source_index,
            source_name,
            buffer_attr,
            flags,
            props,
            direct_on_input,
            ..Default::default()
        };

        if protocol_version >= 14 {
            flags.early_requests = ts.read_bool()?;
        }

        if protocol_version >= 15 {
            flags.no_inhibit_auto_suspend = ts.read_bool()?;
            flags.fail_on_suspend = ts.read_bool()?;
        }

        if protocol_version >= 22 {
            for _ in 0..ts.read_u8()? {
                params.formats.push(ts.read()?);
            }

            let volume = ts.read()?;
            let start_muted = ts.read_bool()?;

            // Set if the client had a volume passed in. Otherwise, it just sent
            // a default cvolume.
            if ts.read_bool()? {
                params.cvolume = Some(volume);
            }

            // Sent by the client if (flags & START_MUTED | START_UNMUTED).
            if ts.read_bool()? {
                flags.start_muted = Some(start_muted);
            }

            flags.relative_volume = ts.read_bool()?;
            flags.passthrough = ts.read_bool()?;
        }

        Ok(params)
    }
}

impl TagStructWrite for RecordStreamParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write(self.sample_spec)?;
        ts.write(self.channel_map)?;
        ts.write_index(self.source_index)?;
        ts.write_string(self.source_name.as_ref())?;
        ts.write_u32(self.buffer_attr.max_length)?;
        ts.write_bool(self.flags.start_corked)?;
        ts.write_u32(self.buffer_attr.fragment_size)?;
        ts.write_bool(self.flags.no_remap_channels)?;
        ts.write_bool(self.flags.no_remix_channels)?;
        ts.write_bool(self.flags.fix_format)?;
        ts.write_bool(self.flags.fix_rate)?;
        ts.write_bool(self.flags.fix_channels)?;
        ts.write_bool(self.flags.no_move)?;
        ts.write_bool(self.flags.variable_rate)?;
        ts.write_bool(self.flags.peak_detect)?;
        ts.write_bool(self.flags.adjust_latency)?;
        ts.write(&self.props)?;
        ts.write_bool(self.direct_on_input)?;

        if protocol_version >= 14 {
            ts.write_bool(self.flags.early_requests)?;
        }

        if protocol_version >= 15 {
            ts.write_bool(self.flags.no_inhibit_auto_suspend)?;
            ts.write_bool(self.flags.fail_on_suspend)?;
        }

        if protocol_version >= 22 {
            ts.write_u8(self.formats.len() as u8)?;
            for format in &self.formats {
                ts.write(format)?;
            }

            ts.write(self.cvolume.unwrap_or_default())?;
            ts.write_bool(self.flags.start_muted.unwrap_or_default())?;
            ts.write_bool(self.cvolume.is_some())?;
            ts.write_bool(self.flags.start_muted.is_some())?;
            ts.write_bool(self.flags.relative_volume)?;
            ts.write_bool(self.flags.passthrough)?;
        }

        Ok(())
    }
}

/// The server reply to [`super::Command::CreateRecordStream`].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct CreateRecordStreamReply {
    /// Channel ID, which is used in other commands to refer to this stream.
    pub channel_index: u32,

    /// Server-internal stream ID.
    pub stream_index: u32,

    /// Attributes of the created buffer.
    pub buffer_attr: BufferAttr,

    /// The final sample format.
    pub sample_spec: SampleSpec,

    /// The finalized channel map.
    pub channel_map: ChannelMap,

    /// The latency of the stream, in microseconds.
    pub stream_latency: u64,

    /// The ID of the sink the stream is connected to.
    pub sink_index: u32,

    /// Name of the sink the stream is connected to.
    pub sink_name: Option<CString>,

    /// Whether the stream is suspended.
    pub suspended: bool,

    /// The finalized format of the stream.
    pub format: FormatInfo,
}

impl CommandReply for CreateRecordStreamReply {}

impl TagStructRead for CreateRecordStreamReply {
    fn read(ts: &mut TagStructReader<'_>, protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            channel_index: ts
                .read_index()?
                .ok_or_else(|| ProtocolError::Invalid("invalid channel_index".into()))?,
            stream_index: ts
                .read_index()?
                .ok_or_else(|| ProtocolError::Invalid("invalid stream_index".into()))?,
            buffer_attr: BufferAttr {
                max_length: ts.read_u32()?,
                fragment_size: ts.read_u32()?,
                ..Default::default()
            },
            sample_spec: ts.read()?,
            channel_map: ts.read()?,
            sink_index: ts
                .read_index()?
                .ok_or_else(|| ProtocolError::Invalid("invalid sink_index".into()))?,
            sink_name: ts.read_string()?,
            suspended: ts.read_bool()?,
            stream_latency: ts.read_usec()?,
            format: if protocol_version >= 21 {
                ts.read()?
            } else {
                FormatInfo::default()
            },
        })
    }
}

impl TagStructWrite for CreateRecordStreamReply {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.channel_index)?;
        w.write_u32(self.stream_index)?;
        w.write_u32(self.buffer_attr.max_length)?;
        w.write_u32(self.buffer_attr.fragment_size)?;

        w.write(self.sample_spec)?;
        w.write(self.channel_map)?;
        w.write_u32(self.sink_index)?;
        w.write_string(self.sink_name.as_ref())?;
        w.write_bool(self.suspended)?;
        w.write_usec(self.stream_latency)?;

        if protocol_version >= 21 {
            w.write(&self.format)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::test_util::test_serde;

    use super::*;

    #[test]
    fn params_serde() -> anyhow::Result<()> {
        let params = RecordStreamParams {
            sample_spec: SampleSpec {
                format: SampleFormat::S16Le,
                sample_rate: 44100,
                channels: 2,
            },
            channel_map: ChannelMap::stereo(),
            ..Default::default()
        };

        test_serde(&params)
    }

    #[test]
    fn reply_serde() -> anyhow::Result<()> {
        let reply = CreateRecordStreamReply {
            channel_index: 0,
            stream_index: 1,
            sink_index: 2,
            ..Default::default()
        };

        test_serde(&reply)
    }
}
