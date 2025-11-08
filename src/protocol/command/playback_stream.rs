use crate::protocol::serde::stream::{BufferAttr, StreamFlags};
use crate::protocol::{serde::*, ProtocolError};
use crate::protocol::{ChannelMap, ChannelVolume, Props, SampleSpec};

use std::ffi::CString;

use super::CommandReply;

/// Parameters for [`super::Command::CreatePlaybackStream`].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct PlaybackStreamParams {
    /// Sample format for the stream.
    pub sample_spec: SampleSpec,

    /// Channel map for the stream.
    ///
    /// Number of channels should match `sample_spec.channels`.
    pub channel_map: ChannelMap,

    /// Index of the sink to connect to.
    pub sink_index: Option<u32>,

    /// Name of the sink to connect to. Ignored if `sink_index` is set.
    pub sink_name: Option<CString>,

    /// Buffer attributes for the stream.
    pub buffer_attr: BufferAttr,

    /// Stream sync ID.
    pub sync_id: u32,

    /// Volume of the stream.
    ///
    /// Number of channels should match `sample_spec.channels`.
    pub cvolume: Option<ChannelVolume>,

    /// Additional properties for the stream.
    pub props: Props,

    /// Formats the client offers.
    pub formats: Vec<FormatInfo>,

    /// Stream flags.
    pub flags: StreamFlags,
}

impl TagStructRead for PlaybackStreamParams {
    fn read(ts: &mut TagStructReader<'_>, protocol_version: u16) -> Result<Self, ProtocolError> {
        let sample_spec = ts.read()?;
        let channel_map = ts.read()?;
        let sink_index = ts.read_index()?;
        let sink_name = ts.read_string()?;

        let buffer_attr_max_length = ts.read_u32()?;

        let mut flags = StreamFlags {
            start_corked: ts.read_bool()?,
            ..Default::default()
        };

        let buffer_attr = BufferAttr {
            max_length: buffer_attr_max_length,
            target_length: ts.read_u32()?,
            pre_buffering: ts.read_u32()?,
            minimum_request_length: ts.read_u32()?,
            ..Default::default()
        };

        let sync_id = ts.read_u32()?;
        let cvolume = Some(ts.read()?);

        flags.no_remap_channels = ts.read_bool()?;
        flags.no_remix_channels = ts.read_bool()?;
        flags.fix_format = ts.read_bool()?;
        flags.fix_rate = ts.read_bool()?;
        flags.fix_channels = ts.read_bool()?;
        flags.no_move = ts.read_bool()?;
        flags.variable_rate = ts.read_bool()?;

        flags.start_muted = Some(ts.read_bool()?);
        flags.adjust_latency = ts.read_bool()?;
        let props = ts.read()?;

        let mut params = Self {
            sample_spec,
            channel_map,
            sink_index,
            sink_name,
            buffer_attr,
            flags,
            sync_id,
            cvolume,
            props,
            ..Default::default()
        };

        if protocol_version >= 14 {
            // Set if the client had a volume passed in. Otherwise, it just sent
            // a default cvolume.
            if !ts.read_bool()? {
                params.cvolume = None;
            }

            flags.early_requests = ts.read_bool()?;
        }

        if protocol_version >= 15 {
            // Sent by the client if (flags & START_MUTED | START_UNMUTED).
            if !ts.read_bool()? {
                flags.start_muted = None
            }

            flags.no_inhibit_auto_suspend = ts.read_bool()?;
            flags.fail_on_suspend = ts.read_bool()?;
        }

        if protocol_version >= 17 {
            flags.relative_volume = ts.read_bool()?;
        }

        if protocol_version >= 18 {
            flags.passthrough = ts.read_bool()?;
        }

        if protocol_version >= 21 {
            for _ in 0..ts.read_u8()? {
                params.formats.push(ts.read()?);
            }
        }

        Ok(params)
    }
}

impl TagStructWrite for PlaybackStreamParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write(self.sample_spec)?;
        ts.write(self.channel_map)?;
        ts.write_index(self.sink_index)?;
        ts.write_string(self.sink_name.as_ref())?;
        ts.write_u32(self.buffer_attr.max_length)?;
        ts.write_bool(self.flags.start_corked)?;
        ts.write_u32(self.buffer_attr.target_length)?;
        ts.write_u32(self.buffer_attr.pre_buffering)?;
        ts.write_u32(self.buffer_attr.minimum_request_length)?;
        ts.write_u32(self.sync_id)?;
        ts.write(
            self.cvolume
                .unwrap_or_else(|| ChannelVolume::muted(self.sample_spec.channels)),
        )?;
        ts.write_bool(self.flags.no_remap_channels)?;
        ts.write_bool(self.flags.no_remix_channels)?;
        ts.write_bool(self.flags.fix_format)?;
        ts.write_bool(self.flags.fix_rate)?;
        ts.write_bool(self.flags.fix_channels)?;
        ts.write_bool(self.flags.no_move)?;
        ts.write_bool(self.flags.variable_rate)?;
        ts.write_bool(self.flags.start_muted.unwrap_or_default())?;
        ts.write_bool(self.flags.adjust_latency)?;
        ts.write(&self.props)?;

        if protocol_version >= 14 {
            ts.write_bool(self.cvolume.is_some())?;
            ts.write_bool(self.flags.early_requests)?;
        }

        if protocol_version >= 15 {
            ts.write_bool(self.flags.start_muted.is_some())?;
            ts.write_bool(self.flags.no_inhibit_auto_suspend)?;
            ts.write_bool(self.flags.fail_on_suspend)?;
        }

        if protocol_version >= 17 {
            ts.write_bool(self.flags.relative_volume)?;
        }

        if protocol_version >= 18 {
            ts.write_bool(self.flags.passthrough)?;
        }

        if protocol_version >= 21 {
            ts.write_u8(self.formats.len() as u8)?;
            for format in &self.formats {
                ts.write(format)?;
            }
        }

        Ok(())
    }
}

/// The server response to [`super::Command::CreatePlaybackStream`].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct CreatePlaybackStreamReply {
    /// Channel ID, which is used in other commands to refer to this stream.
    /// Unlike the stream index, it is scoped to the connection.
    pub channel: u32,

    /// Server-internal stream ID.
    pub stream_index: u32,

    /// The number of bytes that can be written to the playback buffer.
    pub requested_bytes: u32,

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

impl CommandReply for CreatePlaybackStreamReply {}

impl TagStructRead for CreatePlaybackStreamReply {
    fn read(ts: &mut TagStructReader<'_>, protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            channel: ts
                .read_index()?
                .ok_or_else(|| ProtocolError::Invalid("invalid channel_index".into()))?,
            stream_index: ts
                .read_index()?
                .ok_or_else(|| ProtocolError::Invalid("invalid stream_index".into()))?,
            requested_bytes: ts.read_u32()?,
            buffer_attr: BufferAttr {
                max_length: ts.read_u32()?,
                target_length: ts.read_u32()?,
                pre_buffering: ts.read_u32()?,
                minimum_request_length: ts.read_u32()?,
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

impl TagStructWrite for CreatePlaybackStreamReply {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.channel)?;
        w.write_u32(self.stream_index)?;
        w.write_u32(self.requested_bytes)?;
        w.write_u32(self.buffer_attr.max_length)?;
        w.write_u32(self.buffer_attr.target_length)?;
        w.write_u32(self.buffer_attr.pre_buffering)?;
        w.write_u32(self.buffer_attr.minimum_request_length)?;

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
        let params = PlaybackStreamParams {
            sample_spec: SampleSpec {
                format: SampleFormat::S16Le,
                sample_rate: 44100,
                channels: 2,
            },
            channel_map: ChannelMap::stereo(),
            cvolume: Some(ChannelVolume::default()),
            flags: StreamFlags {
                start_corked: true,
                start_muted: Some(true),
                ..Default::default()
            },
            ..Default::default()
        };

        test_serde(&params)
    }

    #[test]
    fn reply_serde() -> anyhow::Result<()> {
        let reply = CreatePlaybackStreamReply {
            channel: 0,
            stream_index: 1,
            sink_index: 2,
            ..Default::default()
        };

        test_serde(&reply)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use super::*;
    use crate::integration_test_util::*;
    use crate::protocol::*;

    #[test]
    fn create_playback_stream() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            &Command::CreatePlaybackStream(PlaybackStreamParams {
                sample_spec: SampleSpec {
                    format: SampleFormat::S16Le,
                    sample_rate: 44100,
                    channels: 2,
                },
                channel_map: ChannelMap::stereo(),
                cvolume: Some(ChannelVolume::norm(2)),
                flags: StreamFlags {
                    start_corked: true,
                    start_muted: Some(true),
                    ..Default::default()
                },
                sink_index: None,
                sink_name: Some(CString::new("@DEFAULT_SINK@")?),
                ..Default::default()
            }),
            protocol_version,
        )?;

        let _ = read_reply_message::<CreatePlaybackStreamReply>(&mut sock, protocol_version)?;

        Ok(())
    }

    /// Tests that PlaybackStreamParams maintains consistent
    /// channel counts across its implicitly set fields.
    #[test]
    fn create_playback_stream_channel_count_invariants() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        // Arbitrarily chosen number of channels that should be kept in sync
        // across fields (chosen to test beyond the usual 1 or 2).
        const CHANNEL_COUNT: u8 = 3;

        // Explicitly set case (for reference).
        {
            write_command_message(
                sock.get_mut(),
                0,
                &Command::CreatePlaybackStream(PlaybackStreamParams {
                    sample_spec: SampleSpec {
                        format: SampleFormat::S16Le,
                        channels: CHANNEL_COUNT,
                        ..Default::default()
                    },
                    sync_id: 0,
                    channel_map: ChannelMap::new([ChannelPosition::Mono; CHANNEL_COUNT as usize]),
                    cvolume: Some(ChannelVolume::norm(CHANNEL_COUNT)),
                    ..Default::default()
                }),
                protocol_version,
            )?;

            let _ = read_reply_message::<CreatePlaybackStreamReply>(&mut sock, protocol_version)?;
        }

        // Implicitly set case (on fields that allow it).
        {
            write_command_message(
                sock.get_mut(),
                1,
                &Command::CreatePlaybackStream(PlaybackStreamParams {
                    sample_spec: SampleSpec {
                        format: SampleFormat::S16Le,
                        channels: CHANNEL_COUNT,
                        ..Default::default()
                    },
                    sync_id: 1,

                    channel_map: ChannelMap::new([ChannelPosition::Mono; CHANNEL_COUNT as usize]),
                    cvolume: None,
                    ..Default::default()
                }),
                protocol_version,
            )?;

            let _ = read_reply_message::<CreatePlaybackStreamReply>(&mut sock, protocol_version)?;
        }

        Ok(())
    }
}
