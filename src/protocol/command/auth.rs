//! Authentication / Handshake command and reply.

use crate::protocol::{serde::*, ProtocolError};

use super::CommandReply;

const VERSION_MASK: u32 = 0x0000ffff;
pub const FLAG_SHM: u32 = 0x80000000;
pub const FLAG_MEMFD: u32 = 0x40000000;

/// Establish connection and authenticate client.
#[derive(Debug, Clone, PartialEq)]
pub struct Auth {
    pub version: u16,
    pub supports_shm: bool,
    pub supports_memfd: bool,
    pub cookie: Vec<u8>,
}

impl TagStructRead for Auth {
    fn read(ts: &mut TagStructReader, _version: u16) -> Result<Self, ProtocolError> {
        let (flags_and_version, cookie) = (ts.read_u32()?, ts.read_arbitrary()?);

        Ok(Self {
            version: (flags_and_version & VERSION_MASK) as u16,
            supports_shm: flags_and_version & FLAG_SHM != 0,
            supports_memfd: flags_and_version & FLAG_MEMFD != 0,
            cookie: cookie.to_owned(),
        })
    }
}

impl TagStructWrite for Auth {
    fn write(&self, w: &mut TagStructWriter, _version: u16) -> Result<(), ProtocolError> {
        let flags_and_version: u32 = (self.version as u32 & VERSION_MASK)
            | if self.supports_shm { FLAG_SHM } else { 0 }
            | if self.supports_memfd { FLAG_MEMFD } else { 0 };

        w.write_u32(flags_and_version)?;
        w.write_arbitrary(self.cookie.as_slice())?;
        Ok(())
    }
}

/// Server reply to `Auth` command.
#[derive(Debug)]
pub struct AuthReply {
    pub version: u16,
    pub use_memfd: bool,
    pub use_shm: bool,
    // TODO: What if both are true? Can that ever happen?
}

impl CommandReply for AuthReply {}

impl TagStructRead for AuthReply {
    fn read(ts: &mut TagStructReader, _version: u16) -> Result<Self, ProtocolError> {
        let reply = ts.read_u32()?;

        Ok(Self {
            version: (reply & VERSION_MASK) as u16,
            use_memfd: reply & FLAG_MEMFD != 0,
            use_shm: reply & FLAG_SHM != 0,
        })
    }
}

impl TagStructWrite for AuthReply {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        // Auth reply is a tagstruct with just a u32 that looks similar to the "version"
        // field in the auth request. It contains the server's protocol version and the
        // result of the shm and memfd negotiation.
        let reply: u32 = self.version as u32
            | if self.use_memfd { FLAG_MEMFD } else { 0 }
            | if self.use_shm { FLAG_SHM } else { 0 };
        w.write_u32(reply)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::serde::test_util::test_serde;

    use super::*;

    #[test]
    fn auth_serde() {
        let auth = Auth {
            version: 13,
            supports_shm: true,
            supports_memfd: false,
            cookie: vec![1, 2, 3, 4],
        };

        test_serde(&auth).expect("roundtrip auth");
    }
}
