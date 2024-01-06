use crate::protocol::{serde::*, ProtocolError};

use super::CommandReply;

/// A reply to the [`super::Command::Stat`] command.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct StatInfo {
    /// The number of currently allocated memory blocks.
    pub memblock_total: u32,
    /// The current total size of all allocated memory blocks.
    pub memblock_total_size: u32,
    /// The number of memblocks allocated over the lifetime of the daemon.
    pub memblock_allocated: u32,
    /// The total size of all memblocks allocated over the lifetime of the daemon.
    pub memblock_allocated_size: u32,
    /// The total size of all sample cache entries.
    pub sample_cache_size: u32,
}

impl CommandReply for StatInfo {}

impl TagStructRead for StatInfo {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            memblock_total: ts.read_u32()?,
            memblock_total_size: ts.read_u32()?,
            memblock_allocated: ts.read_u32()?,
            memblock_allocated_size: ts.read_u32()?,
            sample_cache_size: ts.read_u32()?,
        })
    }
}

impl TagStructWrite for StatInfo {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.memblock_total)?;
        w.write_u32(self.memblock_total_size)?;
        w.write_u32(self.memblock_allocated)?;
        w.write_u32(self.memblock_allocated_size)?;
        w.write_u32(self.sample_cache_size)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::test_util::test_serde;

    use super::*;

    #[test]
    fn stat_serde() -> anyhow::Result<()> {
        let info = StatInfo {
            memblock_total: 1,
            memblock_total_size: 2,
            memblock_allocated: 3,
            memblock_allocated_size: 4,
            sample_cache_size: 5,
        };

        test_serde(&info)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn stat() -> Result<(), Box<dyn std::error::Error>> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(sock.get_mut(), 0, Command::Stat, protocol_version)?;
        let (_, info) = read_reply_message::<StatInfo>(&mut sock, protocol_version)?;

        assert!(info.memblock_total > 0);
        assert!(info.memblock_total_size > 0);
        assert!(info.memblock_allocated > 0);
        assert!(info.memblock_allocated_size > 0);

        Ok(())
    }
}
