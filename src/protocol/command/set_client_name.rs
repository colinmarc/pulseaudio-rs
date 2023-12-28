use crate::protocol::{serde::*, ProtocolError};

use super::CommandReply;

/// The server reply to [`super::Command::SetClientName`].
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SetClientNameReply {
    /// The ID of the new client.
    pub client_id: u32,
}

impl CommandReply for SetClientNameReply {}

impl TagStructRead for SetClientNameReply {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let client_id = ts.read_u32()?;
        Ok(Self { client_id })
    }
}

impl TagStructWrite for SetClientNameReply {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.client_id)?;
        Ok(())
    }
}
