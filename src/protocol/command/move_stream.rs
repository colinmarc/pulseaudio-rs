use super::*;

/// Parameters for [`super::Command::MoveSinkInput`] and [`super::Command::MoveSourceOutput`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MoveStreamParams {
    /// The index of the sink input or source output.
    pub index: Option<u32>,

    /// The index of the destination sink or source.
    pub device_index: Option<u32>,

    /// The name of the destination sink or source.
    pub device_name: Option<CString>,
}

impl TagStructRead for MoveStreamParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts.read_index()?,
            device_index: ts.read_index()?,
            device_name: ts.read_string()?,
        })
    }
}

impl TagStructWrite for MoveStreamParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(self.index)?;
        ts.write_index(self.device_index)?;
        ts.write_string(self.device_name.as_ref())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn test_move_stream_params_serde() -> anyhow::Result<()> {
        let params = MoveStreamParams {
            index: Some(0),
            device_index: Some(1),
            device_name: Some(CString::new("device").unwrap()),
        };

        test_serde(&params)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use anyhow::Ok;
    use assert_matches::assert_matches;

    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn test_move_stream() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            Command::MoveSinkInput(MoveStreamParams {
                index: Some(999),
                device_index: Some(999),
                device_name: None,
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }
}
