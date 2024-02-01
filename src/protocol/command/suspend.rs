use std::ffi::CString;

use super::*;

/// Parameters for [`super::Command::SuspendSink`] and [`super::Command::SuspendSource`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SuspendParams {
    /// The index of the sink or source.
    pub device_index: Option<u32>,

    /// The name of the sink or source.
    pub device_name: Option<CString>,

    /// Whether to suspend or resume.
    pub suspend: bool,
}

impl TagStructRead for SuspendParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            device_index: ts.read_index()?,
            device_name: ts.read_string()?,
            suspend: ts.read_bool()?,
        })
    }
}

impl TagStructWrite for SuspendParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(self.device_index)?;
        ts.write_string(self.device_name.as_ref())?;
        ts.write_bool(self.suspend)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn test_suspend_params_serde() -> anyhow::Result<()> {
        let params = SuspendParams {
            device_index: None,
            device_name: Some(CString::new("device").unwrap()),
            suspend: true,
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
    fn test_suspend() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            Command::SuspendSink(SuspendParams {
                device_index: Some(999),
                device_name: None,
                suspend: true,
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }
}
