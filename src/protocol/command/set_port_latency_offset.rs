use std::ffi::CString;

use super::*;

/// Parameters for [`super::Command::SetPortLatencyOffset`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SetPortLatencyOffsetParams {
    /// The index of the card.
    pub index: Option<u32>,

    /// The name of the card.
    pub name: Option<CString>,

    /// The name of the port.
    pub port_name: CString,

    /// The offset to set.
    pub offset: i64,
}

impl TagStructRead for SetPortLatencyOffsetParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts.read_index()?,
            name: ts.read_string()?,
            port_name: ts.read_string_non_null()?,
            offset: ts.read_i64()?,
        })
    }
}

impl TagStructWrite for SetPortLatencyOffsetParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(self.index)?;
        ts.write_string(self.name.as_ref())?;
        ts.write_string(Some(&self.port_name))?;
        ts.write_i64(self.offset)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn test_set_port_latency_offset_params_serde() -> anyhow::Result<()> {
        let params = SetPortLatencyOffsetParams {
            index: None,
            name: Some(CString::new("name").unwrap()),
            port_name: CString::new("port").unwrap(),
            offset: 0,
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
    fn test_set_port_latency() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            Command::SetPortLatencyOffset(SetPortLatencyOffsetParams {
                index: None,
                name: Some(CString::new("name").unwrap()),
                port_name: CString::new("port").unwrap(),
                offset: 0,
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }
}
