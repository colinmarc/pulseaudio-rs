use std::ffi::CString;

use super::*;

/// Parameters for [`super::Command::SetSinkPort`] and [`super::Command::SetSourcePort`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SetPortParams {
    /// The index of the sink or source.
    pub index: Option<u32>,

    /// The name of the sink or source.
    pub name: Option<CString>,

    /// The name of the port to set.
    pub port_name: CString,
}

impl TagStructRead for SetPortParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts.read_index()?,
            name: ts.read_string()?,
            port_name: ts
                .read_string()?
                .ok_or_else(|| ProtocolError::Invalid("invalid port name".into()))?,
        })
    }
}

impl TagStructWrite for SetPortParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(self.index)?;
        ts.write_string(self.name.as_ref())?;
        ts.write_string(Some(&self.port_name))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn test_set_port_params_serde() -> anyhow::Result<()> {
        let params = SetPortParams {
            index: None,
            name: Some(CString::new("device").unwrap()),
            port_name: CString::new("port").unwrap(),
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
    fn test_set_port() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            Command::SetSinkPort(SetPortParams {
                index: Some(999),
                name: None,
                port_name: CString::new("port").unwrap(),
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }
}
