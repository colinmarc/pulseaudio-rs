use std::ffi::CString;

use crate::protocol::{serde::*, ProtocolError};

use super::CommandReply;

/// Server state for a client, in response to [`super::Command::GetClientInfo`].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct ClientInfo {
    /// ID of the client.
    pub index: u32,

    /// The name of the client.
    pub name: CString,

    /// The index of the owning module.
    pub owner_module_index: Option<u32>,

    /// The driver name.
    pub driver: Option<CString>,

    /// The client properties.
    pub props: Props,
}

impl CommandReply for ClientInfo {}

impl TagStructRead for ClientInfo {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts.read_u32()?,
            name: ts.read_string_non_null()?,
            owner_module_index: ts.read_index()?,
            driver: ts.read_string()?,
            props: ts.read()?,
        })
    }
}

impl TagStructWrite for ClientInfo {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.index)?;
        w.write_string(Some(&self.name))?;
        w.write_index(self.owner_module_index)?;
        w.write_string(self.driver.as_ref())?;
        w.write(&self.props)?;

        Ok(())
    }
}

/// The server reply to [`super::Command::GetClientInfoList`].
pub type ClientInfoList = Vec<ClientInfo>;

impl CommandReply for ClientInfoList {}

impl TagStructRead for ClientInfoList {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut clients = Vec::new();
        while ts.has_data_left()? {
            clients.push(ts.read()?);
        }

        Ok(clients)
    }
}

impl TagStructWrite for ClientInfoList {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        for client in self {
            w.write(client)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_info_serde() -> anyhow::Result<()> {
        let client_info = ClientInfo {
            index: 0,
            name: CString::new("test").unwrap(),
            ..Default::default()
        };

        test_util::test_serde(&client_info)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn list_clients() -> Result<(), Box<dyn std::error::Error>> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            &Command::GetClientInfoList,
            protocol_version,
        )?;
        let (seq, info_list) = read_reply_message::<ClientInfoList>(&mut sock, protocol_version)?;
        assert_eq!(seq, 0);
        assert!(!info_list.is_empty());

        Ok(())
    }
}
