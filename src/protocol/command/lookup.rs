use crate::protocol::{serde::*, ProtocolError};

use super::CommandReply;

/// The server response to [`super::Command::LookupSource`] and [`super::Command::LookupSink`].
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct LookupReply(u32);

impl CommandReply for LookupReply {}

impl TagStructRead for LookupReply {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self(ts.read_u32()?))
    }
}

impl TagStructWrite for LookupReply {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.0)?;
        Ok(())
    }
}
