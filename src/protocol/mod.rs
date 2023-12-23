pub mod command;
pub mod paths;
pub mod serde;
pub mod sink;
pub mod stream;

mod error;
mod types;

use std::io::{BufRead, Cursor, Read, Seek, SeekFrom, Write};

use bitflags::bitflags;
use byteorder::NetworkEndian;
pub use command::*;
pub use error::*;
pub use types::*;

/// Minimum protocol version understood by the library.
pub const PROTOCOL_MIN_VERSION: u16 = 13;

/// PulseAudio protocol version implemented by this library.
///
/// This library can still work with clients and servers down to `PROTOCOL_MIN_VERSION` and up to
/// any higher version, but features added by versions higher than this are not supported.
pub const PROTOCOL_VERSION: u16 = 32;

const DESCRIPTOR_SIZE: usize = 5 * 4;

bitflags! {
    /// Special message types.
    #[derive(Debug, Default, Clone)]
    pub struct DescriptorFlags: u32 {
        const FLAG_SHMRELEASE = 0x40000000; // 0b0100
        const FLAG_SHMREVOKE = 0xC0000000; // 0b1100 FIXME 2 bits set?
    }
}

/// Packet descriptor / header.
#[derive(Debug, Clone)]
pub struct Descriptor {
    /// Payload length in Bytes.
    length: u32,
    /// The channel this packet belongs to, or -1 for a control packet.
    channel: u32,
    /// Offset into the memblock, in Bytes.
    offset: u64,
    /// SHMRELEASE or SHMREVOKE to mark packet as such, or:
    ///
    /// For memblock packets:
    /// * Lowest byte: Seek mode
    flags: DescriptorFlags,
}

/// Read a message header from an input stream.
pub fn read_descriptor<R: Read>(r: &mut R) -> Result<Descriptor, ProtocolError> {
    use byteorder::ReadBytesExt;

    let length = r.read_u32::<NetworkEndian>()?;
    let channel = r.read_u32::<NetworkEndian>()?;
    let offset = r.read_u64::<NetworkEndian>()?;
    let flags = r.read_u32::<NetworkEndian>()?;

    Ok(Descriptor {
        length,
        channel,
        offset,
        flags: DescriptorFlags::from_bits_truncate(flags),
    })
}

pub fn read_command_message<R: BufRead>(r: &mut R) -> Result<(u32, Command), ProtocolError> {
    let desc = read_descriptor(r)?;
    Command::read_tag_prefixed(&mut r.take(desc.length as u64), PROTOCOL_VERSION)
}

/// Write a message header to an output stream.
pub fn write_descriptor<W: Write>(w: &mut W, desc: Descriptor) -> Result<(), ProtocolError> {
    use byteorder::WriteBytesExt;

    w.write_u32::<NetworkEndian>(desc.length)?;
    w.write_u32::<NetworkEndian>(desc.channel)?;
    w.write_u64::<NetworkEndian>(desc.offset)?;
    w.write_u32::<NetworkEndian>(desc.flags.bits())?;

    Ok(())
}

pub fn read_reply_message<T: command::CommandReply>(
    r: &mut impl BufRead,
) -> Result<(u32, T), ProtocolError> {
    let desc = read_descriptor(r)?;

    let mut r = r.take(desc.length as u64);
    let mut ts = serde::TagStructReader::new(&mut r, PROTOCOL_VERSION);
    let (cmd, seq) = (ts.read_enum()?, ts.read_u32()?);

    match cmd {
        command::CommandTag::Error => {
            let error = ts.read_enum()?;
            Err(ProtocolError::ServerError(error))
        }
        command::CommandTag::Reply => Ok((seq, T::read(&mut ts, PROTOCOL_VERSION)?)),
        _ => Err(ProtocolError::Invalid(format!(
            "expected error, got {:?}",
            cmd
        ))),
    }
}

/// Writes a command message to a buffer, and returns the number of bytes
/// written.
pub fn encode_command_message<T: AsRef<[u8]>>(
    command: Command,
    seq: u32,
    buf: T,
) -> Result<usize, ProtocolError>
where
    Cursor<T>: Seek + Write,
{
    let mut cursor = Cursor::new(buf);
    cursor.seek(SeekFrom::Start(DESCRIPTOR_SIZE as u64))?;

    command.write_tag_prefixed(seq, &mut cursor)?;
    let length = (cursor.position() - DESCRIPTOR_SIZE as u64)
        .try_into()
        .map_err(|_| ProtocolError::Invalid("message payload greater than 4gb".to_string()))?;

    let desc = Descriptor {
        length,
        channel: u32::MAX,
        offset: 0,
        flags: DescriptorFlags::empty(),
    };

    cursor.set_position(0);
    write_descriptor(&mut cursor, desc)?;

    Ok(cursor.position() as usize)
}

/// Write a command message to an output stream. This will allocate a temporary
/// buffer to encode the command. To avoid the extra copy, use
/// [`encode_command_message`].
pub fn write_command_message<W: Write>(
    w: &mut W,
    seq: u32,
    command: Command,
) -> Result<(), ProtocolError> {
    let mut buf = Cursor::new(Vec::new());
    command.write_tag_prefixed(seq, &mut buf)?;

    let length = buf
        .position()
        .try_into()
        .map_err(|_| ProtocolError::Invalid("message payload greater than 4gb".to_string()))?;

    let desc = Descriptor {
        length,
        channel: u32::MAX,
        offset: 0,
        flags: DescriptorFlags::empty(),
    };

    write_descriptor(w, desc)?;
    w.write_all(buf.into_inner().as_slice())?;

    Ok(())
}

#[cfg(test)]
pub mod test_util {
    use super::serde::*;
    use super::*;
    use pretty_assertions::assert_eq;

    pub fn test_roundtrip<T>(v: &T) -> Result<(), ProtocolError>
    where
        T: TagStructRead + TagStructWrite + PartialEq + std::fmt::Debug,
        for<'a> &'a T: PartialEq,
    {
        for version in PROTOCOL_MIN_VERSION..PROTOCOL_VERSION {
            test_roundtrip_version(v, version)?;
        }

        Ok(())
    }

    pub fn test_roundtrip_version<T>(v: &T, version: u16) -> Result<(), ProtocolError>
    where
        T: TagStructRead + TagStructWrite + std::fmt::Debug,
        for<'a> &'a T: PartialEq,
    {
        let mut buf = Vec::new();

        {
            let mut ts = TagStructWriter::new(&mut buf, version);
            ts.write(v)?;
        }

        let mut cursor = Cursor::new(buf);
        let mut ts = TagStructReader::new(&mut cursor, version);
        let v2 = T::read(&mut ts, version)?;

        assert_eq!(v, &v2);

        Ok(())
    }
}
