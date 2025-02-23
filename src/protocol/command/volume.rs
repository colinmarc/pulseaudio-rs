use super::*;

/// Paramaters for [`super::Command::SetSinkVolume`] and [`super::Command::SetSourceVolume`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SetDeviceVolumeParams {
    /// The index of the sink or source.
    pub device_index: Option<u32>,

    /// The name of the sink or source.
    pub device_name: Option<CString>,

    /// The volume to set.
    pub volume: ChannelVolume,
}

impl TagStructRead for SetDeviceVolumeParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            device_index: ts.read_index()?,
            device_name: ts.read_string()?,
            volume: ts.read()?,
        })
    }
}

impl TagStructWrite for SetDeviceVolumeParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(self.device_index)?;
        ts.write_string(self.device_name.as_ref())?;
        ts.write(self.volume)?;
        Ok(())
    }
}

/// Parameters for [`super::Command::SetSinkInputVolume`] and [`super::Command::SetSourceOutputVolume`].
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SetStreamVolumeParams {
    /// The index of the sink input or source output.
    pub index: u32,

    /// The volume to set.
    pub volume: ChannelVolume,
}

impl TagStructRead for SetStreamVolumeParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts
                .read_index()?
                .ok_or_else(|| ProtocolError::Invalid("invalid index".into()))?,
            volume: ts.read()?,
        })
    }
}

impl TagStructWrite for SetStreamVolumeParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(Some(self.index))?;
        ts.write(self.volume)?;
        Ok(())
    }
}

/// Parameters for [`super::Command::SetSinkMute`] and [`super::Command::SetSourceMute`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SetDeviceMuteParams {
    /// The index of the sink or source to mute or unmute.
    pub device_index: Option<u32>,

    /// The name of the sink or source to mute or unmute.
    pub device_name: Option<CString>,

    /// Whether to mute or unmute.
    pub mute: bool,
}

impl TagStructRead for SetDeviceMuteParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            device_index: ts.read_index()?,
            device_name: ts.read_string()?,
            mute: ts.read_bool()?,
        })
    }
}

impl TagStructWrite for SetDeviceMuteParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(self.device_index)?;
        ts.write_string(self.device_name.as_ref())?;
        ts.write_bool(self.mute)?;
        Ok(())
    }
}

/// Parameters for [`super::Command::SetSinkInputMute`] and [`super::Command::SetSourceOutputMute`].
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SetStreamMuteParams {
    /// The index of the sink input or source output.
    pub index: u32,

    /// Whether to mute or unmute.
    pub mute: bool,
}

impl TagStructRead for SetStreamMuteParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts
                .read_index()?
                .ok_or_else(|| ProtocolError::Invalid("invalid index".into()))?,
            mute: ts.read_bool()?,
        })
    }
}

impl TagStructWrite for SetStreamMuteParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(Some(self.index))?;
        ts.write_bool(self.mute)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn test_set_device_volume_params_serde() -> anyhow::Result<()> {
        let params = SetDeviceVolumeParams {
            device_index: None,
            device_name: Some(CString::new("device").unwrap()),
            volume: ChannelVolume::default(),
        };

        test_serde(&params)
    }

    #[test]
    fn test_set_volume_params_serde() -> anyhow::Result<()> {
        let params = SetStreamVolumeParams {
            index: 1,
            volume: ChannelVolume::default(),
        };

        test_serde(&params)
    }

    #[test]
    fn test_set_device_mute_params_serde() -> anyhow::Result<()> {
        let params = SetDeviceMuteParams {
            device_index: None,
            device_name: Some(CString::new("device").unwrap()),
            mute: false,
        };

        test_serde(&params)
    }

    #[test]
    fn test_set_mute_params_serde() -> anyhow::Result<()> {
        let params = SetStreamMuteParams {
            index: 1,
            mute: false,
        };

        test_serde(&params)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use assert_matches::assert_matches;

    use anyhow::Ok;

    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn test_set_device_volume() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            &Command::SetSinkVolume(SetDeviceVolumeParams {
                device_index: Some(999),
                device_name: None,
                volume: ChannelVolume::default(),
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }

    #[test]
    fn test_set_stream_volume() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            &Command::SetSinkInputVolume(SetStreamVolumeParams {
                index: 999,
                volume: ChannelVolume::default(),
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }

    #[test]
    fn test_set_device_mute() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            &Command::SetSinkMute(SetDeviceMuteParams {
                device_index: Some(999),
                device_name: None,
                mute: false,
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }

    #[test]
    fn test_set_stream_mute() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            &Command::SetSinkInputMute(SetStreamMuteParams {
                index: 999,
                mute: false,
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }
}
