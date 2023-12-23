use crate::protocol::{serde::*, ProtocolError};

// FIXME: Figure out exact semantics of this command
// FIXME: PA ignores this (doesn't attach the received memfd) at proto<31
#[derive(Debug)]
pub struct RegisterMemfdShmid {
    shmid: u32,
}

impl TagStructRead for RegisterMemfdShmid {
    fn read(ts: &mut TagStructReader, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            shmid: ts.read_u32()?,
        })
    }
}

impl TagStructWrite for RegisterMemfdShmid {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.write_u32(self.shmid)?;
        Ok(())
    }
}
