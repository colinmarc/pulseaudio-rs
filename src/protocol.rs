//! An implementation of the PulseAudio IPC protocol.

pub mod command;
mod serde;

mod error;

use std::io::{BufRead, Cursor, Read, Seek, SeekFrom, Write};

use bitflags::bitflags;
use byteorder::NetworkEndian;
pub use command::*;
pub use error::*;
pub use serde::*;

/// Minimum protocol version understood by the library.
pub const MIN_VERSION: u16 = 13;

/// PulseAudio protocol version implemented by this library.
///
/// This library can still work with clients and servers down to `PROTOCOL_MIN_VERSION` and up to
/// any higher version, but features added by versions higher than this are not supported.
pub const MAX_VERSION: u16 = 35;

/// The size of a message header.
pub const DESCRIPTOR_SIZE: usize = 5 * 4;

bitflags! {
    /// Special message types.
    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
    pub struct DescriptorFlags: u32 {
        /// Indicates a SHMRELEASE message.
        const FLAG_SHMRELEASE = 0x40000000; // 0b0100

        /// Indicates a SHMREVOKE message.
        const FLAG_SHMREVOKE = 0xC0000000; // 0b1100 FIXME 2 bits set?
    }
}

/// Packet descriptor / header.
#[derive(Debug, Clone, Copy)]
pub struct Descriptor {
    /// Payload length in Bytes.
    pub length: u32,
    /// The channel this packet belongs to, or -1 for a control packet.
    pub channel: u32,
    /// Offset into the memblock, in Bytes.
    pub offset: u64,
    /// SHMRELEASE or SHMREVOKE to mark packet as such, or:
    ///
    /// For memblock packets:
    /// * Lowest byte: Seek mode
    pub flags: DescriptorFlags,
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

/// Writes a message header to an output stream.
pub fn write_descriptor<W: Write>(w: &mut W, desc: &Descriptor) -> Result<(), ProtocolError> {
    use byteorder::WriteBytesExt;

    w.write_u32::<NetworkEndian>(desc.length)?;
    w.write_u32::<NetworkEndian>(desc.channel)?;
    w.write_u64::<NetworkEndian>(desc.offset)?;
    w.write_u32::<NetworkEndian>(desc.flags.bits())?;

    Ok(())
}

/// Encodes a message header to a buffer.
pub fn encode_descriptor(buf: &mut [u8; DESCRIPTOR_SIZE], desc: &Descriptor) {
    buf[0..4].copy_from_slice(&desc.length.to_be_bytes());
    buf[4..8].copy_from_slice(&desc.channel.to_be_bytes());
    buf[8..16].copy_from_slice(&desc.offset.to_be_bytes());
    buf[16..20].copy_from_slice(&desc.flags.bits().to_be_bytes());
}

/// Reads a command message from an input stream. If the result is
/// [`Command::Reply`], then the payload is command-specific and must be read
/// immediately afterwards.
pub fn read_command_message<R: BufRead>(
    r: &mut R,
    protocol_version: u16,
) -> Result<(u32, Command), ProtocolError> {
    let desc = read_descriptor(r)?;
    Command::read_tag_prefixed(&mut r.take(desc.length as u64), protocol_version)
}

/// Writes a command message to a buffer, and returns the number of bytes
/// written.
pub fn encode_command_message<T: AsRef<[u8]>>(
    seq: u32,
    command: &Command,
    buf: T,
    protocol_version: u16,
) -> Result<usize, ProtocolError>
where
    Cursor<T>: Seek + Write,
{
    let mut cursor = Cursor::new(buf);
    cursor.seek(SeekFrom::Start(DESCRIPTOR_SIZE as u64))?;

    command.write_tag_prefixed(seq, &mut cursor, protocol_version)?;
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
    write_descriptor(&mut cursor, &desc)?;

    Ok(length as usize + DESCRIPTOR_SIZE)
}

/// Writes a command message to an output stream. This will allocate a temporary
/// buffer to encode the command. To avoid the extra copy, use
/// [`encode_command_message`].
pub fn write_command_message<W: Write>(
    w: &mut W,
    seq: u32,
    command: &Command,
    protocol_version: u16,
) -> Result<(), ProtocolError> {
    let mut buf = Cursor::new(Vec::new());
    command.write_tag_prefixed(seq, &mut buf, protocol_version)?;

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

    write_descriptor(w, &desc)?;
    w.write_all(buf.into_inner().as_slice())?;

    Ok(())
}

/// Reads reply data from the server.
pub fn read_reply_message<T: CommandReply>(
    r: &mut impl BufRead,
    protocol_version: u16,
) -> Result<(u32, T), ProtocolError> {
    let desc = read_descriptor(r)?;

    let mut r = r.take(desc.length as u64);
    let mut ts = TagStructReader::new(&mut r, protocol_version);
    let (cmd, seq) = (ts.read_enum()?, ts.read_u32()?);

    match cmd {
        CommandTag::Error => {
            let error = ts.read_enum()?;
            Err(ProtocolError::ServerError(error))
        }
        CommandTag::Reply => Ok((seq, T::read(&mut ts, protocol_version)?)),
        _ => Err(ProtocolError::UnexpectedCommand(cmd)),
    }
}

/// Writes reply data to a buffer, and returns the number of bytes written.
pub fn encode_reply_message<T: AsRef<[u8]>, R: CommandReply>(
    seq: u32,
    reply: &R,
    buf: T,
    protocol_version: u16,
) -> Result<usize, ProtocolError>
where
    Cursor<T>: Seek + Write,
{
    let mut cursor = Cursor::new(buf);
    cursor.seek(SeekFrom::Start(DESCRIPTOR_SIZE as u64))?;

    let mut ts = TagStructWriter::new(&mut cursor, protocol_version);
    ts.write_u32(CommandTag::Reply as u32)?;
    ts.write_u32(seq)?;
    ts.write(reply)?;

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
    write_descriptor(&mut cursor, &desc)?;

    Ok(length as usize + DESCRIPTOR_SIZE)
}

/// Writes reply data to a client. This will allocate a temporary buffer. To
/// avoid the extra copy, use [`encode_reply_message`].
pub fn write_reply_message<W: Write, R: CommandReply>(
    w: &mut W,
    seq: u32,
    reply: &R,
    protocol_version: u16,
) -> Result<(), ProtocolError> {
    let mut buf = Cursor::new(Vec::new());
    let mut ts = TagStructWriter::new(&mut buf, protocol_version);

    ts.write_u32(CommandTag::Reply as u32)?;
    ts.write_u32(seq)?;
    ts.write(reply)?;

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

    write_descriptor(w, &desc)?;
    w.write_all(buf.into_inner().as_slice())?;

    Ok(())
}

/// Reads an ack (an empty reply) from the server.
pub fn read_ack_message(r: &mut impl BufRead) -> Result<u32, ProtocolError> {
    let desc = read_descriptor(r)?;

    let mut r = r.take(desc.length as u64);

    // Protocol version doesn't matter for this.
    let mut ts = TagStructReader::new(&mut r, MAX_VERSION);
    let (cmd, seq) = (ts.read_enum()?, ts.read_u32()?);

    match cmd {
        CommandTag::Error => {
            let error = ts.read_enum()?;
            Err(ProtocolError::ServerError(error))
        }
        CommandTag::Reply => Ok(seq),
        _ => Err(ProtocolError::Invalid(format!(
            "expected reply, got {:?}",
            cmd
        ))),
    }
}

/// Writes an ack (an empty reply) to a buffer, and returns the number of bytes
/// written.
pub fn encode_ack_message<T: AsRef<[u8]>>(seq: u32, buf: T) -> Result<usize, ProtocolError>
where
    Cursor<T>: Seek + Write,
{
    let mut cursor = Cursor::new(buf);
    cursor.seek(SeekFrom::Start(DESCRIPTOR_SIZE as u64))?;

    // Protocol version doesn't matter for this.
    let mut ts = TagStructWriter::new(&mut cursor, MAX_VERSION);
    ts.write_u32(CommandTag::Reply as u32)?;
    ts.write_u32(seq)?;

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
    write_descriptor(&mut cursor, &desc)?;

    Ok(length as usize + DESCRIPTOR_SIZE)
}

/// Write an ack (an empty reply) to a client.
pub fn write_ack_message<W: Write>(w: &mut W, seq: u32) -> Result<(), ProtocolError> {
    let desc = Descriptor {
        length: 10, // Two tagged u32s.
        channel: u32::MAX,
        offset: 0,
        flags: DescriptorFlags::empty(),
    };

    write_descriptor(w, &desc)?;

    // Protocol version doesn't matter for this.
    let mut ts = TagStructWriter::new(w, MAX_VERSION);

    ts.write_u32(CommandTag::Reply as u32)?;
    ts.write_u32(seq)?;

    Ok(())
}

/// Write an error reply to a client. This is equivalent to [`write_command_message`] with [`Command::Error`].
pub fn write_error<W: Write>(w: &mut W, seq: u32, error: &PulseError) -> Result<(), ProtocolError> {
    let desc = Descriptor {
        length: 15, // Three tagged u32s.
        channel: u32::MAX,
        offset: 0,
        flags: DescriptorFlags::empty(),
    };

    write_descriptor(w, &desc)?;

    // Protocol version doesn't matter for this.
    let mut ts = TagStructWriter::new(w, MAX_VERSION);

    ts.write_u32(CommandTag::Error as u32)?;
    ts.write_u32(seq)?;
    ts.write_u32(*error as u32)?;

    Ok(())
}

/// Writes a stream chunk.
pub fn write_memblock<W: Write>(
    w: &mut W,
    channel: u32,
    chunk: &[u8],
    offset: u64,
) -> Result<(), ProtocolError> {
    let desc = Descriptor {
        length: chunk.len() as u32,
        channel,
        offset,
        flags: DescriptorFlags::empty(),
    };

    write_descriptor(w, desc)?;
    w.write_all(chunk)?;

    Ok(())
}
