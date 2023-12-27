//! A "tagstruct" is PulseAudio's central IPC data structure.
//!
//! A tagstruct is a sequence of type-tagged `Value`s. This module provides parsers for the format
//! and writers to easily create tagstruct byte streams.

pub mod channel_map;
pub mod format_info;
pub mod props;
pub mod sample_spec;
pub mod volume;

pub use channel_map::{ChannelMap, ChannelPosition};
pub use format_info::*;
pub use props::{Prop, Props};
pub use sample_spec::{SampleFormat, SampleSpec};
pub use volume::{ChannelVolume, Volume};

use super::ProtocolError;

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use enum_primitive_derive::Primitive;
use num_traits::FromPrimitive;
use std::ffi::{CStr, CString};
use std::io::prelude::*;
use std::time;

#[allow(bad_style)]
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Primitive)]
pub enum Tag {
    String = b't',
    StringNull = b'N',
    U32 = b'L',
    U8 = b'B',
    U64 = b'R',
    S64 = b'r',
    SampleSpec = b'a',
    Arbitrary = b'x',
    BooleanTrue = b'1',
    BooleanFalse = b'0',
    TimeVal = b'T',
    Usec = b'U',
    ChannelMap = b'm',
    CVolume = b'v',
    PropList = b'P',
    Volume = b'V',
    FormatInfo = b'f',
}

/// Streaming zero-copy reader for untrusted data.
///
/// The data stream is parsed and validated on the fly.
pub struct TagStructReader<'a> {
    inner: &'a mut dyn BufRead,
    protocol_version: u16,
}

impl<'a> TagStructReader<'a> {
    /// Creates a tagstruct reader from a `BufRead` instance.
    pub fn new(inner: &'a mut dyn BufRead, protocol_version: u16) -> Self {
        TagStructReader {
            inner,
            protocol_version,
        }
    }

    /// Reads a tag from the stream.
    pub fn read_tag(&mut self) -> Result<Tag, ProtocolError> {
        let v = self.inner.read_u8()?;
        Tag::from_u8(v)
            .ok_or_else(|| ProtocolError::Invalid(format!("invalid tag 0x{:02X} in tagstruct", v)))
    }

    /// Reads a tag from the stream, and returns an error if it's not equal
    /// to the given tag.
    fn expect_tag(&mut self, next: Tag) -> Result<(), ProtocolError> {
        let t = self.read_tag()?;
        if t == next {
            Ok(())
        } else {
            Err(ProtocolError::Invalid(format!(
                "expected {:?}, got {:?}",
                next, t
            )))
        }
    }

    /// Reads a single byte.
    pub fn read_u8(&mut self) -> Result<u8, ProtocolError> {
        self.expect_tag(Tag::U8)?;
        Ok(self.inner.read_u8()?)
    }

    /// Reads an unsigned 32-bit integer.
    pub fn read_u32(&mut self) -> Result<u32, ProtocolError> {
        self.expect_tag(Tag::U32)?;
        Ok(self.inner.read_u32::<NetworkEndian>()?)
    }

    /// Reads an unsigned 64-bit integer.
    pub fn read_u64(&mut self) -> Result<u64, ProtocolError> {
        self.expect_tag(Tag::U64)?;
        Ok(self.inner.read_u64::<NetworkEndian>()?)
    }

    /// Reads a signed 64-bit integer.
    pub fn read_i64(&mut self) -> Result<i64, ProtocolError> {
        self.expect_tag(Tag::S64)?;
        Ok(self.inner.read_i64::<NetworkEndian>()?)
    }

    /// Reads a boolean value.
    pub fn read_bool(&mut self) -> Result<bool, ProtocolError> {
        let tag = self.read_tag()?;
        match tag {
            Tag::BooleanTrue => Ok(true),
            Tag::BooleanFalse => Ok(false),
            _ => Err(ProtocolError::Invalid(format!(
                "expected boolean, got {:?}",
                tag
            ))),
        }
    }

    /// Reads a "usec" value, which is a 64-bit unsigned integer representing
    /// a number of microseconds.
    pub fn read_usec(&mut self) -> Result<u64, ProtocolError> {
        self.expect_tag(Tag::Usec)?;
        Ok(self.inner.read_u64::<NetworkEndian>()?)
    }

    /// Reads a timestamp, or "timeval", with microsecond precision.
    pub fn read_timeval(&mut self) -> Result<time::SystemTime, ProtocolError> {
        self.expect_tag(Tag::TimeVal)?;

        let secs = self.inner.read_u32::<NetworkEndian>()?;
        let usecs = self.inner.read_u32::<NetworkEndian>()?;

        let duration = time::Duration::new(secs as u64, usecs * 1000);
        Ok(time::UNIX_EPOCH + duration)
    }

    /// Reads an "arbitrary", a byte blob. Allocates a vec.
    pub fn read_arbitrary(&mut self) -> Result<Vec<u8>, ProtocolError> {
        self.expect_tag(Tag::Arbitrary)?;

        let len = self.inner.read_u32::<NetworkEndian>()?;
        let mut buf = vec![0; len as usize];
        self.inner.read_exact(&mut buf)?;

        Ok(buf)
    }

    /// Reads an "arbitrary" without length prefix.
    pub fn read_arbitrary_unprefixed(&mut self, len: u32) -> Result<Vec<u8>, ProtocolError> {
        self.expect_tag(Tag::Arbitrary)?;

        let mut buf = vec![0; len as usize];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Reads a null-terminated string (which may be a special null string tag).
    ///
    /// Note that strings in a tagstruct aren't necessarily encoded in UTF-8.
    pub fn read_string(&mut self) -> Result<Option<CString>, ProtocolError> {
        match self.read_tag()? {
            Tag::String => {
                let mut buf = Vec::new();
                self.inner.read_until(0x00, &mut buf)?;
                Ok(Some(CString::from_vec_with_nul(buf).map_err(|e| {
                    ProtocolError::Invalid(format!("invalid string in tagstruct: {}", e))
                })?))
            }
            Tag::StringNull => Ok(None),
            tag => Err(ProtocolError::Invalid(format!(
                "expected string or null string, got {:?}",
                tag
            ))),
        }
    }

    /// Reads a u32, and checks it against PA_INVALID_INDEX (-1).
    pub fn read_index(&mut self) -> Result<Option<u32>, ProtocolError> {
        const INVALID_INDEX: u32 = u32::MAX;

        let v = self.read_u32()?;
        if v == INVALID_INDEX {
            Ok(None)
        } else {
            Ok(Some(v))
        }
    }

    /// Reads a u32 and checks that it is valid for the given enum.
    pub fn read_enum<T: FromPrimitive>(&mut self) -> Result<T, ProtocolError> {
        let v = self.read_u32()?;
        T::from_u32(v).ok_or_else(|| {
            ProtocolError::Invalid(format!(
                "invalid enum value {} for {}",
                v,
                std::any::type_name::<T>()
            ))
        })
    }

    pub fn read<T: TagStructRead>(&mut self) -> Result<T, ProtocolError> {
        T::read(self, self.protocol_version)
    }

    pub fn has_data_left(&mut self) -> Result<bool, ProtocolError> {
        Ok(self.inner.fill_buf().map(|b| !b.is_empty())?)
    }
}

pub struct TagStructWriter<'a> {
    inner: &'a mut dyn Write,
    protocol_version: u16,
}

impl<'a> TagStructWriter<'a> {
    /// Creates a tagstruct writer that writes to a reusable buffer.
    pub fn new(inner: &'a mut dyn Write, protocol_version: u16) -> Self {
        Self {
            inner,
            protocol_version,
        }
    }

    /// Writes a boolean value.
    pub fn write_bool(&mut self, b: bool) -> Result<(), ProtocolError> {
        self.inner.write_u8(if b {
            Tag::BooleanTrue as u8
        } else {
            Tag::BooleanFalse as u8
        })?;

        Ok(())
    }

    /// Writes a single byte.
    pub fn write_u8(&mut self, value: u8) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::U8 as u8)?;
        self.inner.write_u8(value)?;
        Ok(())
    }

    /// Writes an unsigned 32-bit integer.
    pub fn write_u32(&mut self, value: u32) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::U32 as u8)?;
        self.inner.write_u32::<NetworkEndian>(value)?;
        Ok(())
    }

    /// Writes an unsigned 64-bit integer.
    pub fn write_u64(&mut self, value: u64) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::U64 as u8)?;
        self.inner.write_u64::<NetworkEndian>(value)?;
        Ok(())
    }

    /// Writes a signed 64-bit integer.
    pub fn write_i64(&mut self, value: i64) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::S64 as u8)?;
        self.inner.write_i64::<NetworkEndian>(value)?;
        Ok(())
    }

    /// Writes an index as a u32, or PA_INVALID_INDEX (-1) in the case of None.
    pub fn write_index(&mut self, index: Option<u32>) -> Result<(), ProtocolError> {
        match index {
            Some(index) => self.write_u32(index),
            None => self.write_u32(u32::MAX),
        }
    }

    /// Writes a string, or a special "null string" tag.
    pub fn write_string<S: AsRef<CStr>>(&mut self, value: Option<S>) -> Result<(), ProtocolError> {
        match value {
            Some(value) => {
                self.inner.write_u8(Tag::String as u8)?;
                self.inner.write_all(value.as_ref().to_bytes_with_nul())?;
            }
            None => self.write_null_string()?,
        }

        Ok(())
    }

    /// Writes a special "null string" tag.
    pub fn write_null_string(&mut self) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::StringNull as u8)?;
        Ok(())
    }

    /// Writes a "usec" value, which is a 64-bit unsigned integer representing
    /// a number of microseconds.
    pub fn write_usec(&mut self, n: u64) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::Usec as u8)?;
        self.inner.write_u64::<NetworkEndian>(n)?;
        Ok(())
    }

    /// Writes a timestamp as a "timeval" with microsecond precision.
    pub fn write_timeval(&mut self, t: time::SystemTime) -> Result<(), ProtocolError> {
        let d = t.duration_since(std::time::UNIX_EPOCH).unwrap();
        let (secs, usecs) = (d.as_secs() as u32, d.subsec_micros());

        self.write_timeval_raw(secs, usecs)
    }

    /// Writes a "timeval" with microsecond precision.
    pub fn write_timeval_raw(&mut self, secs: u32, usecs: u32) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::TimeVal as u8)?;
        self.inner.write_u32::<NetworkEndian>(secs)?;
        self.inner.write_u32::<NetworkEndian>(usecs)?;
        Ok(())
    }

    ///  Write an "arbitrary", a byte blob, with prefix length.
    pub fn write_arbitrary<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), ProtocolError> {
        let bytes = bytes.as_ref();

        assert!(bytes.len() < u32::MAX as usize);
        self.inner.write_u8(Tag::Arbitrary as u8)?;
        self.inner.write_u32::<NetworkEndian>(bytes.len() as u32)?;
        self.inner.write_all(bytes)?;
        Ok(())
    }

    /// Appends a single value to the tagstruct.
    ///
    /// To append multiple values at once, use the `Extend` implementation.
    pub fn write<T: TagStructWrite>(&mut self, value: T) -> Result<(), ProtocolError> {
        // this cannot fail when we're writing into a `Vec<u8>`
        value.write(self, self.protocol_version)
    }
}

/// Trait implemented by types that can be serialized into a tagstruct.
pub trait TagStructWrite {
    /// Write `self` into a tagstruct.
    fn write(&self, w: &mut TagStructWriter, protocol_version: u16) -> Result<(), ProtocolError>;
}

/// Implemented by types that can be deserialized from a tagstruct.
pub trait TagStructRead: Sized {
    /// Read an instance of `Self` from a tagstruct.
    ///
    /// # Parameters
    ///
    /// * `ts`: The tagstruct to read from.
    /// * `protocol_version`: PulseAudio protocol version, used to decide on the precise data
    ///   format. For old versions, default values might be used for parts of `Self`.
    fn read(ts: &mut TagStructReader, protocol_version: u16) -> Result<Self, ProtocolError>;
}

impl<'a, T: ?Sized> TagStructWrite for &'a T
where
    T: TagStructWrite,
{
    fn write(&self, w: &mut TagStructWriter, protocol_version: u16) -> Result<(), ProtocolError> {
        (*self).write(w, protocol_version)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn usec_serde() {
        let usec = 1234567890;
        let mut buf = Vec::new();
        {
            let mut w = TagStructWriter::new(&mut buf, 0);
            w.write_usec(usec).unwrap();
        }

        let mut cursor = Cursor::new(&buf);
        let mut r = TagStructReader::new(&mut cursor, 0);
        let usec2 = r.read_usec().unwrap();

        assert_eq!(usec, usec2);
    }

    #[test]
    fn timeval_serde() {
        // Has to be truncated to micros.
        let t1 = time::UNIX_EPOCH + time::Duration::new(1234567890, 123000);

        let mut buf = Vec::new();
        {
            let mut w = TagStructWriter::new(&mut buf, 0);
            w.write_timeval(t1).unwrap();
        }

        let mut cursor = Cursor::new(&buf);
        let mut r = TagStructReader::new(&mut cursor, 0);
        let t2 = r.read_timeval().unwrap();

        assert_eq!(t1, t2);
    }
}

#[cfg(test)]
pub mod test_util {
    use super::*;
    use crate::protocol::{PROTOCOL_MIN_VERSION, PROTOCOL_VERSION};

    use anyhow::Context as _;
    use pretty_assertions::assert_eq;
    use std::io::Cursor;

    pub fn test_serde<T>(v: &T) -> anyhow::Result<()>
    where
        T: TagStructRead + TagStructWrite + PartialEq + std::fmt::Debug,
        for<'a> &'a T: PartialEq,
    {
        for version in PROTOCOL_MIN_VERSION..PROTOCOL_VERSION {
            test_serde_version(v, version)
                .context(format!("roundtrip failed for protocol version {}", version))?;
        }

        Ok(())
    }

    pub fn test_serde_version<T>(v: &T, version: u16) -> anyhow::Result<()>
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

        assert_eq!(v, &v2, "roundtrip failed for protocol version {}", version);

        Ok(())
    }
}
