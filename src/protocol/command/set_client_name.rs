use crate::protocol::{serde::*, Props, ProtocolError};

use super::CommandReply;

/// Add or modify client properties.
#[derive(Debug)]
pub struct SetClientName {
    pub props: Props,
}

impl TagStructRead for SetClientName {
    fn read(ts: &mut TagStructReader, _protocol_version: u16) -> Result<Self, ProtocolError> {
        // before protocol version 13, *only* the client name was transferred (as a string)
        // proto>=13
        let props = ts.read()?;
        Ok(Self { props })
    }
}

impl TagStructWrite for SetClientName {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.write(&self.props)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SetClientNameReply {
    client_id: u32,
}

impl CommandReply for SetClientNameReply {}

impl TagStructRead for SetClientNameReply {
    fn read(ts: &mut TagStructReader, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let client_id = ts.read_u32()?;
        Ok(Self { client_id })
    }
}

impl TagStructWrite for SetClientNameReply {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.write_u32(self.client_id)?;
        Ok(())
    }
}
