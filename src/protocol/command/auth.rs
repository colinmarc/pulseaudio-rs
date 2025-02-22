//! Authentication / Handshake command and reply.

use crate::protocol::{serde::*, ProtocolError};

use super::CommandReply;

const VERSION_MASK: u32 = 0x0000ffff;
pub(crate) const FLAG_SHM: u32 = 0x80000000;
pub(crate) const FLAG_MEMFD: u32 = 0x40000000;

/// The auth command is the first message a client should send on connection.
#[derive(Default, Clone, Eq, PartialEq)]
pub struct AuthParams {
    /// The client's protocol version.
    pub version: u16,

    /// Whether the client supports shared memory memblocks.
    pub supports_shm: bool,

    /// Whether the client supports memfd memblocks.
    pub supports_memfd: bool,

    /// A password-like blob, usually created by the server at ~/.pulse-cookie.
    pub cookie: Vec<u8>,
}

// Avoid printing the cookie.
impl std::fmt::Debug for AuthParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthParams")
            .field("version", &self.version)
            .field("supports_shm", &self.supports_shm)
            .field("supports_memfd", &self.supports_memfd)
            .field("cookie", &"<redacted>")
            .finish()
    }
}

impl TagStructRead for AuthParams {
    fn read(ts: &mut TagStructReader<'_>, _version: u16) -> Result<Self, ProtocolError> {
        let (flags_and_version, cookie) = (ts.read_u32()?, ts.read_arbitrary()?);

        Ok(Self {
            version: (flags_and_version & VERSION_MASK) as u16,
            supports_shm: flags_and_version & FLAG_SHM != 0,
            supports_memfd: flags_and_version & FLAG_MEMFD != 0,
            cookie: cookie.to_owned(),
        })
    }
}

impl TagStructWrite for AuthParams {
    fn write(&self, w: &mut TagStructWriter<'_>, _version: u16) -> Result<(), ProtocolError> {
        let flags_and_version: u32 = (self.version as u32 & VERSION_MASK)
            | if self.supports_shm { FLAG_SHM } else { 0 }
            | if self.supports_memfd { FLAG_MEMFD } else { 0 };

        w.write_u32(flags_and_version)?;
        w.write_arbitrary(self.cookie.as_slice())?;
        Ok(())
    }
}

/// The server reply to [`super::Command::Auth`].
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthReply {
    /// The negotiated protocol version.
    pub version: u16,

    /// Whether the server supports memfd memblocks.
    pub use_memfd: bool,

    /// Whether the server supports shared memory memblocks.
    pub use_shm: bool,
}

impl CommandReply for AuthReply {}

impl TagStructRead for AuthReply {
    fn read(ts: &mut TagStructReader<'_>, _version: u16) -> Result<Self, ProtocolError> {
        let reply = ts.read_u32()?;

        Ok(Self {
            version: (reply & VERSION_MASK) as u16,
            use_memfd: reply & FLAG_MEMFD != 0,
            use_shm: reply & FLAG_SHM != 0,
        })
    }
}

impl TagStructWrite for AuthReply {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
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
        let auth = AuthParams {
            version: 13,
            supports_shm: true,
            supports_memfd: false,
            cookie: vec![1, 2, 3, 4],
        };

        test_serde(&auth).expect("roundtrip auth");
    }
}
