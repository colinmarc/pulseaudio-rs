use crate::protocol::stream::BufferAttr;

use super::*;

/// Parameters for [`super::Command::SetPlaybackStreamName`] and [`super::Command::SetRecordStreamName`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetStreamNameParams {
    /// The index of the stream to update.
    pub index: u32,

    /// The new name.
    pub name: CString,
}

impl TagStructRead for SetStreamNameParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let index = ts.read_u32()?;
        let name = ts.read_string_non_null()?;

        Ok(Self { index, name })
    }
}

impl TagStructWrite for SetStreamNameParams {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.index)?;
        w.write_string(Some(&self.name))?;

        Ok(())
    }
}

/// Parameters for [`super::Command::SetPlaybackStreamBufferAttr`].
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SetPlaybackStreamBufferAttrParams {
    /// The index of the stream to update.
    pub index: u32,

    /// The new buffer attributes. `fragment_size` is ignored.
    pub buffer_attr: BufferAttr,

    /// Sets the stream flag for adjusting latency. See [`super::stream::StreamFlags`].
    pub adjust_latency: bool,

    /// Sets the stream flag for early requests. See [`super::stream::StreamFlags`].
    pub early_requests: bool,
}

impl TagStructRead for SetPlaybackStreamBufferAttrParams {
    fn read(ts: &mut TagStructReader<'_>, protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts
                .read_index()?
                .ok_or(ProtocolError::Invalid("invalid index".into()))?,
            buffer_attr: BufferAttr {
                max_length: ts.read_u32()?,
                target_length: ts.read_u32()?,
                pre_buffering: ts.read_u32()?,
                minimum_request_length: ts.read_u32()?,
                fragment_size: 0,
            },
            adjust_latency: ts.read_bool()?,
            early_requests: if protocol_version >= 14 {
                ts.read_bool()?
            } else {
                false
            },
        })
    }
}

impl TagStructWrite for SetPlaybackStreamBufferAttrParams {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_index(Some(self.index))?;
        w.write_u32(self.buffer_attr.max_length)?;
        w.write_u32(self.buffer_attr.target_length)?;
        w.write_u32(self.buffer_attr.pre_buffering)?;
        w.write_u32(self.buffer_attr.minimum_request_length)?;
        w.write_bool(self.adjust_latency)?;
        if protocol_version >= 14 {
            w.write_bool(self.early_requests)?;
        }
        Ok(())
    }
}

/// The reply to [`super::Command::SetPlaybackStreamBufferAttr`].
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SetPlaybackStreamBufferAttrReply {
    /// The negotiated buffer attributes. `fragment_size` is always 0.
    pub buffer_attr: BufferAttr,

    /// The configured sink latency, in microseconds.
    pub configured_sink_latency: u64,
}

impl CommandReply for SetPlaybackStreamBufferAttrReply {}

impl TagStructRead for SetPlaybackStreamBufferAttrReply {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            buffer_attr: BufferAttr {
                max_length: ts.read_u32()?,
                target_length: ts.read_u32()?,
                pre_buffering: ts.read_u32()?,
                minimum_request_length: ts.read_u32()?,
                fragment_size: 0,
            },
            configured_sink_latency: ts.read_usec()?,
        })
    }
}

impl TagStructWrite for SetPlaybackStreamBufferAttrReply {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.buffer_attr.max_length)?;
        w.write_u32(self.buffer_attr.target_length)?;
        w.write_u32(self.buffer_attr.pre_buffering)?;
        w.write_u32(self.buffer_attr.minimum_request_length)?;
        w.write_usec(self.configured_sink_latency)?;
        Ok(())
    }
}

/// Parameters for [`super::Command::SetRecordStreamBufferAttr`].
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SetRecordStreamBufferAttrParams {
    /// The index of the stream to update.
    pub index: u32,

    /// The new buffer attributes. Only `max_length` and `fragment_size` are used.
    pub buffer_attr: BufferAttr,

    /// Sets the stream flag for adjusting latency. See [`super::stream::StreamFlags`].
    pub adjust_latency: bool,

    /// Sets the stream flag for early requests. See [`super::stream::StreamFlags`].
    pub early_requests: bool,
}

impl TagStructRead for SetRecordStreamBufferAttrParams {
    fn read(ts: &mut TagStructReader<'_>, protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts
                .read_index()?
                .ok_or(ProtocolError::Invalid("invalid index".into()))?,
            buffer_attr: BufferAttr {
                max_length: ts.read_u32()?,
                target_length: 0,
                pre_buffering: 0,
                minimum_request_length: 0,
                fragment_size: ts.read_u32()?,
            },
            adjust_latency: ts.read_bool()?,
            early_requests: if protocol_version >= 14 {
                ts.read_bool()?
            } else {
                false
            },
        })
    }
}

impl TagStructWrite for SetRecordStreamBufferAttrParams {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_index(Some(self.index))?;
        w.write_u32(self.buffer_attr.max_length)?;
        w.write_u32(self.buffer_attr.fragment_size)?;
        w.write_bool(self.adjust_latency)?;
        if protocol_version >= 14 {
            w.write_bool(self.early_requests)?;
        }

        Ok(())
    }
}

/// The reply to [`super::Command::SetRecordStreamBufferAttr`].
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SetRecordStreamBufferAttrReply {
    /// The negotiated buffer attributes. Only `max_length` and `fragment_size` are used.
    pub buffer_attr: BufferAttr,

    /// The configured source latency, in microseconds.
    pub configured_source_latency: u64,
}

impl CommandReply for SetRecordStreamBufferAttrReply {}

impl TagStructRead for SetRecordStreamBufferAttrReply {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            buffer_attr: BufferAttr {
                max_length: ts.read_u32()?,
                target_length: 0,
                pre_buffering: 0,
                minimum_request_length: 0,
                fragment_size: ts.read_u32()?,
            },
            configured_source_latency: ts.read_usec()?,
        })
    }
}

impl TagStructWrite for SetRecordStreamBufferAttrReply {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.buffer_attr.max_length)?;
        w.write_u32(self.buffer_attr.fragment_size)?;
        w.write_usec(self.configured_source_latency)?;
        Ok(())
    }
}

/// Parameters for [`super::Command::UpdatePlaybackStreamProplist`] and [`super::Command::UpdateRecordStreamProplist`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePropsParams {
    /// The index of the object to update.
    pub index: u32,

    /// The type of update being performed.
    pub mode: props::PropsUpdateMode,

    /// The new props.
    pub props: Props,
}

impl TagStructRead for UpdatePropsParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let index = ts.read_u32()?;
        let mode = ts.read_enum()?;
        let props = ts.read()?;

        Ok(Self { index, mode, props })
    }
}

impl TagStructWrite for UpdatePropsParams {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.index)?;
        w.write_u32(self.mode as u32)?;
        w.write(&self.props)?;

        Ok(())
    }
}

/// Parameters for [`super::Command::UpdatePlaybackStreamSampleRate`] and [`super::Command::UpdateRecordStreamSampleRate`].
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct UpdateSampleRateParams {
    /// The index of the stream to update.
    pub index: u32,

    /// The new sample rate.
    pub sample_rate: u32,
}

impl TagStructRead for UpdateSampleRateParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts
                .read_index()?
                .ok_or(ProtocolError::Invalid("invalid index".into()))?,
            sample_rate: ts.read_u32()?,
        })
    }
}

impl TagStructWrite for UpdateSampleRateParams {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_index(Some(self.index))?;
        w.write_u32(self.sample_rate)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn test_set_stream_name_params_serde() -> anyhow::Result<()> {
        let params = SetStreamNameParams {
            index: 0,
            name: CString::new("name").unwrap(),
        };

        test_serde(&params)
    }

    #[test]
    fn test_update_props_params_serde() -> anyhow::Result<()> {
        let params = UpdatePropsParams {
            index: 0,
            mode: props::PropsUpdateMode::Set,
            props: Props::new(),
        };

        test_serde(&params)
    }

    #[test]
    fn test_update_sample_rate_params_serde() -> anyhow::Result<()> {
        let params = UpdateSampleRateParams {
            index: 0,
            sample_rate: 1234,
        };

        test_serde(&params)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use assert_matches::assert_matches;
    use std::ffi::CString;

    use anyhow::Ok;

    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn test_set_stream_name() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            &Command::SetPlaybackStreamName(SetStreamNameParams {
                index: 999,
                name: CString::new("stream").unwrap(),
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }

    #[test]
    fn test_update_stream_props() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            &Command::UpdatePlaybackStreamProplist(UpdatePropsParams {
                index: 999,
                mode: props::PropsUpdateMode::Set,
                props: Props::new(),
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }

    #[test]
    fn test_update_stream_sample_rate() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            &Command::UpdatePlaybackStreamSampleRate(UpdateSampleRateParams {
                index: 999,
                sample_rate: 44100,
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }
}
