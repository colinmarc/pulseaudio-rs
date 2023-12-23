//! A "tagstruct" is PulseAudio's central IPC data structure.
//!
//! A tagstruct is a sequence of type-tagged `Value`s. This module provides parsers for the format
//! and writers to easily create tagstruct byte streams.

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use enum_primitive_derive::Primitive;
use num_traits::FromPrimitive;
use std::ffi::{CStr, CString};
use std::io::prelude::*;
use std::time;

use crate::protocol::sample_spec::CHANNELS_MAX;
use crate::protocol::{
    ChannelMap, ChannelPosition, FormatInfo, ProtocolError, SampleFormat, SampleSpec,
};

use super::{ChannelVolume, Props, Volume};

/// Max. size of a proplist value in Bytes.
const MAX_PROP_SIZE: u32 = 64 * 1024;

#[allow(bad_style)]
#[repr(u8)]
#[derive(Debug, Copy, Clone, Primitive)]
enum Tag {
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

/// Enum of the different values that can be stored in a tagstruct.
#[derive(Debug, Clone)]
pub enum Value {
    /// Zero-terminated string without prefix length. The zero is *not* included in the slice.
    ///
    /// Per construction, the string data cannot contain any nul bytes.
    String(CString),
    /// Encodes a string that's a null pointer.
    ///
    /// This is distinguishable from an empty string and perhaps analogous to `Option::None`.
    NullString,
    U32(u32),
    U8(u8),
    U64(u64),
    S64(i64),
    SampleSpec(SampleSpec),
    /// Byte Blob with prefix length.
    Arbitrary(Vec<u8>),
    Boolean(bool),
    Timeval(u32, u32),
    Usec(u64),
    ChannelMap(ChannelMap),
    ChannelVolume(ChannelVolume),
    PropList(Props),
    Volume(Volume),
    FormatInfo(FormatInfo),
}

impl From<u8> for Value {
    fn from(i: u8) -> Self {
        Value::U8(i)
    }
}

impl From<u32> for Value {
    fn from(i: u32) -> Self {
        Value::U32(i)
    }
}

impl From<u64> for Value {
    fn from(i: u64) -> Self {
        Value::U64(i)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::S64(i)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Boolean(b)
    }
}

impl From<time::Duration> for Value {
    fn from(d: time::Duration) -> Self {
        Value::Usec(d.as_micros() as u64)
    }
}

impl From<time::SystemTime> for Value {
    fn from(t: time::SystemTime) -> Self {
        let d = t.duration_since(std::time::UNIX_EPOCH).unwrap();
        Value::Timeval(d.as_secs() as u32, d.subsec_micros())
    }
}

impl From<SampleSpec> for Value {
    fn from(s: SampleSpec) -> Self {
        Value::SampleSpec(s)
    }
}

impl From<ChannelMap> for Value {
    fn from(m: ChannelMap) -> Self {
        Value::ChannelMap(m)
    }
}

impl From<ChannelVolume> for Value {
    fn from(v: ChannelVolume) -> Self {
        Value::ChannelVolume(v)
    }
}

impl From<Props> for Value {
    fn from(p: Props) -> Self {
        Value::PropList(p)
    }
}

macro_rules! read_typed {
    ($method:ident = Value::$variant:ident -> $t:ty) => {
        pub fn $method(&mut self) -> Result<$t, ProtocolError> {
            match self.try_read()? {
                Value::$variant(v) => Ok(v),
                v => Err(ProtocolError::Invalid(format!(
                    "expected {}({}), got {:?}",
                    stringify!($variant),
                    stringify!($t),
                    v,
                ))),
            }
        }
    };
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

    // /// Read a given number of bytes from the data stream.
    // ///
    // /// Returns an error if the stream ends prematurely.
    // fn read_n(&mut self, n: usize) -> Result<&'a [u8], ProtocolError> {
    //     let slice = self.inner.read_exact(buf)

    //     let pos = self.inner.position() as usize;
    //     let left = self.inner.get_ref()[pos..].len();
    //     if bytes > left {
    //         return Err(io::Error::new(
    //             io::ErrorKind::UnexpectedEof,
    //             "end of data reached when reading bytes from tagstruct",
    //         )
    //         .into());
    //     }

    //     self.inner.consume(bytes); // consume bytes we just read

    //     let slice = &(*self.inner.get_ref())[pos..pos + bytes];
    //     assert_eq!(slice.len(), bytes);
    //     Ok(slice)
    // }

    // /// Read bytes from the data stream until a specific termination byte is reached.
    // ///
    // /// The termination byte is contained in the returned slice.
    // fn read_until(&mut self, until: u8) -> Result<&'a [u8], ProtocolError> {
    //     // equivalent to `BufRead::read_until`
    //     // reading from a Cursor without allocation requires a small dance
    //     let pos = self.inner.position() as usize;

    //     let length = self.inner.get_ref()[pos..] // get data still left to read
    //         .iter()
    //         .position(|&byte| byte == until) // find delimiter or bail
    //         .ok_or_else(|| {
    //             ProtocolError::Invalid(format!(
    //                 "expected delimiter 0x{:02X} not found in stream",
    //                 until
    //             ))
    //         })?;

    //     let slice = self.read_n(length + 1)?; // with terminator
    //     assert_eq!(slice.last(), Some(&until), "expected terminator in data");

    //     Ok(&slice[..slice.len()])
    // }

    /// Reads the next value or returns `None` when at EOF.
    pub fn read_value(&mut self) -> Result<Option<Value>, ProtocolError> {
        let raw_tag = match self.inner.read_u8() {
            Ok(tag) => tag,
            // Return Ok(None) for well-behaved EOF
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let tag = Tag::from_u8(raw_tag).ok_or_else(|| {
            ProtocolError::Invalid(format!("invalid tag 0x{:02X} in tagstruct", raw_tag))
        })?;
        Ok(Some(match tag {
            Tag::String => {
                let mut buf = Vec::new();
                self.inner.read_until(0x00, &mut buf)?;
                Value::String(CString::from_vec_with_nul(buf).map_err(|e| {
                    ProtocolError::Invalid(format!("invalid string in tagstruct: {}", e))
                })?)
            }
            Tag::StringNull => Value::NullString,
            Tag::U32 => Value::U32(self.inner.read_u32::<NetworkEndian>()?),
            Tag::U8 => Value::U8(self.inner.read_u8()?),
            Tag::U64 => Value::U64(self.inner.read_u64::<NetworkEndian>()?),
            Tag::S64 => Value::S64(self.inner.read_i64::<NetworkEndian>()?),
            Tag::Arbitrary => {
                // prefix length u32
                let len = self.inner.read_u32::<NetworkEndian>()?;
                let mut buf = vec![0; len as usize];
                self.inner.read_exact(&mut buf)?;
                Value::Arbitrary(buf)
            }
            Tag::BooleanTrue => Value::Boolean(true),
            Tag::BooleanFalse => Value::Boolean(false),
            Tag::PropList => {
                // A proplist is a key-value map with string keys and blob values.
                // It's stored as a sequence of keys and values, terminated by a null string.
                let mut proplist = Props::new();
                while let Some(key) = self.read_string()? {
                    if key.to_bytes().is_empty() {
                        return Err(ProtocolError::Invalid("proplist key is empty".into()));
                    }

                    let key = key.to_str().map_err(|e| {
                        ProtocolError::Invalid(format!(
                            "proplist key contains invalid utf-8: {}",
                            e
                        ))
                    })?;
                    if !key.is_ascii() {
                        return Err(ProtocolError::Invalid(format!(
                            "proplist key contains non-ASCII characters: {:?}",
                            key
                        )));
                    }

                    let data_len = self.read_u32()?;
                    if data_len > MAX_PROP_SIZE {
                        return Err(ProtocolError::Invalid(format!(
                            "proplist value size {} exceeds hard limit of {} bytes",
                            data_len, MAX_PROP_SIZE
                        )));
                    }

                    let data = self.read_sized_arbitrary(data_len)?;
                    proplist.insert(key.to_owned(), data.into());
                }

                Value::PropList(proplist)
            }
            Tag::SampleSpec => {
                let (format, channels, rate) = (
                    self.inner.read_u8()?,
                    self.inner.read_u8()?,
                    self.inner.read_u32::<NetworkEndian>()?,
                );

                let format = SampleFormat::from_u8(format).ok_or_else(|| {
                    ProtocolError::Invalid(format!("invalid sample format 0x{:02X}", format))
                })?;

                Value::SampleSpec(
                    SampleSpec::new(format, channels, rate)
                        .map_err(|e| ProtocolError::Invalid(e.to_string()))?,
                )
            }
            Tag::ChannelMap => {
                let channels = self.inner.read_u8()?;
                if channels > CHANNELS_MAX {
                    return Err(ProtocolError::Invalid(format!(
                        "channel map too large (max is {} channels, got {})",
                        CHANNELS_MAX, channels
                    )));
                }

                let mut map = ChannelMap::new();
                for _ in 0..channels {
                    let raw = self.inner.read_u8()?;
                    map.push(ChannelPosition::from_u8(raw).ok_or_else(|| {
                        ProtocolError::Invalid(format!("invalid channel position {}", raw))
                    })?)
                    .expect("channel map full despite channels being in range");
                }

                Value::ChannelMap(map)
            }
            Tag::CVolume => {
                // Very similar to channel maps
                let n_channels = self.inner.read_u8()?;
                if n_channels == 0 || n_channels > CHANNELS_MAX {
                    return Err(ProtocolError::Invalid(format!(
                        "invalid cvolume channel count {}, must be between 1 and {}",
                        n_channels, CHANNELS_MAX
                    )));
                }

                let mut cvolume = ChannelVolume::empty();
                for _ in 0..n_channels {
                    let raw = self.inner.read_u32::<NetworkEndian>()?;
                    cvolume.push(Volume::from_u32_clamped(raw))
                }

                Value::ChannelVolume(cvolume)
            }
            Tag::Usec => Value::Usec(self.inner.read_u64::<NetworkEndian>()?),
            Tag::Volume => Value::Volume(Volume::from_u32_clamped(
                self.inner.read_u32::<NetworkEndian>()?,
            )),
            Tag::FormatInfo => {
                let encoding = self.read_u8()?;
                let props = self.read_proplist()?;
                Value::FormatInfo(FormatInfo::from_raw(encoding, props)?)
            }
            Tag::TimeVal => {
                let secs = self.inner.read_u32::<NetworkEndian>()?;
                let usecs = self.inner.read_u32::<NetworkEndian>()?;
                Value::Timeval(secs, usecs)
            }
        }))
    }

    /// Read the next `Value`, treating EOF as an error.
    pub fn try_read(&mut self) -> Result<Value, ProtocolError> {
        self.read_value()?
            .ok_or_else(|| ProtocolError::Io(std::io::ErrorKind::UnexpectedEof.into()))
    }

    // Helper methods that skip the `Value` enum:

    read_typed!(read_u32 = Value::U32 -> u32);
    read_typed!(read_u8 = Value::U8 -> u8);
    read_typed!(read_u64 = Value::U64 -> u64);
    read_typed!(read_i64 = Value::S64 -> i64);
    read_typed!(read_bool = Value::Boolean -> bool);
    read_typed!(read_arbitrary = Value::Arbitrary -> Vec<u8>);
    read_typed!(read_string_non_null = Value::String -> CString);
    read_typed!(read_proplist = Value::PropList -> Props);
    read_typed!(read_sample_spec = Value::SampleSpec -> SampleSpec);
    read_typed!(read_channel_map = Value::ChannelMap -> ChannelMap);
    read_typed!(read_cvolume = Value::ChannelVolume -> ChannelVolume);
    read_typed!(read_format_info = Value::FormatInfo -> FormatInfo);
    read_typed!(read_usec = Value::Usec -> u64);
    read_typed!(read_volume = Value::Volume -> Volume);

    pub fn read_timeval(&mut self) -> Result<time::SystemTime, ProtocolError> {
        match self.try_read()? {
            Value::Timeval(sec, usec) => {
                let d = time::Duration::new(sec as u64, usec * 1000);
                Ok(std::time::UNIX_EPOCH + d)
            }
            v => Err(ProtocolError::Invalid(format!(
                "expected {}({}), got {:?}",
                stringify!($variant),
                stringify!($t),
                v,
            ))),
        }
    }

    /// Reads a `Value::Arbitrary` with an expected size.
    ///
    /// If the next value is not a `Value::Arbitrary` or has the wrong length, or the tagstruct is
    /// at EOF, an error is returned.
    pub fn read_sized_arbitrary(&mut self, expected_length: u32) -> Result<Vec<u8>, ProtocolError> {
        let a = self.read_arbitrary()?;
        if a.len() != expected_length as usize {
            return Err(ProtocolError::Invalid(format!(
                "expected arbitrary of length {}, got length {}",
                expected_length,
                a.len()
            )));
        }

        Ok(a)
    }

    /// Reads the next value from the tagstruct and expects it to be a string or a null string.
    ///
    /// If the next value is not a `Value::String` or a `Value::NullString` (or the tagstruct is at
    /// EOF), an error is returned.
    ///
    /// If the next value is a `Value::String`, returns an `Ok(Some(<string>))`. If the next value
    /// is a `Value::NullString`, returns an `Ok(None)`.
    ///
    /// Note that strings in a tagstruct aren't necessarily encoded in UTF-8.
    pub fn read_string(&mut self) -> Result<Option<CString>, ProtocolError> {
        match self.try_read()? {
            Value::String(s) => Ok(Some(s)),
            Value::NullString => Ok(None),
            v => Err(ProtocolError::Invalid(format!(
                "expected string or null string, got {:?}",
                v
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

    fn write_value(&mut self, value: &Value) -> Result<(), ProtocolError> {
        use self::Value::*;

        // Forward to the `ToTagStruct` impls. Protocol version doesn't matter since these are
        // primitive data types making up the tagstruct format.
        match value {
            String(s) => Ok(self.write_string(Some(s))?),
            NullString => Ok(self.write_null_string()?),
            U32(n) => Ok(self.write_u32(*n)?),
            U8(n) => Ok(self.write_u8(*n)?),
            U64(n) => Ok(self.write_u64(*n)?),
            S64(n) => Ok(self.write_i64(*n)?),
            SampleSpec(spec) => self.write(spec),
            Arbitrary(bytes) => self.write_arbitrary(bytes),
            Boolean(b) => self.write_bool(*b),
            Timeval(secs, usecs) => self.write_timeval_raw(*secs, *usecs),
            Usec(n) => self.write_usec(*n),
            ChannelMap(map) => self.write(map),
            ChannelVolume(volumes) => self.write(volumes),
            PropList(proplist) => self.write(proplist),
            Volume(volume) => self.write(volume),
            FormatInfo(info) => self.write(info),
        }
    }

    pub fn write_bool(&mut self, b: bool) -> Result<(), ProtocolError> {
        self.inner.write_u8(if b {
            Tag::BooleanTrue as u8
        } else {
            Tag::BooleanFalse as u8
        })?;

        Ok(())
    }

    pub fn write_u8(&mut self, value: u8) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::U8 as u8)?;
        self.inner.write_u8(value)?;
        Ok(())
    }

    pub fn write_u32(&mut self, value: u32) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::U32 as u8)?;
        self.inner.write_u32::<NetworkEndian>(value)?;
        Ok(())
    }

    pub fn write_u64(&mut self, value: u64) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::U64 as u8)?;
        self.inner.write_u64::<NetworkEndian>(value)?;
        Ok(())
    }

    pub fn write_i64(&mut self, value: i64) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::S64 as u8)?;
        self.inner.write_i64::<NetworkEndian>(value)?;
        Ok(())
    }

    pub(crate) fn write_index(&mut self, index: Option<u32>) -> Result<(), ProtocolError> {
        match index {
            Some(index) => self.write_u32(index),
            None => self.write_u32(u32::MAX),
        }
    }

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

    pub fn write_null_string(&mut self) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::StringNull as u8)?;
        Ok(())
    }

    pub fn write_usec(&mut self, n: u64) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::Usec as u8)?;
        self.inner.write_u64::<NetworkEndian>(n)?;
        Ok(())
    }

    pub fn write_timeval(&mut self, t: time::SystemTime) -> Result<(), ProtocolError> {
        let d = t.duration_since(std::time::UNIX_EPOCH).unwrap();
        let (secs, usecs) = (d.as_secs() as u32, d.subsec_micros());

        self.write_timeval_raw(secs, usecs)
    }

    fn write_timeval_raw(&mut self, secs: u32, usecs: u32) -> Result<(), ProtocolError> {
        self.inner.write_u8(Tag::TimeVal as u8)?;
        self.inner.write_u32::<NetworkEndian>(secs)?;
        self.inner.write_u32::<NetworkEndian>(usecs)?;
        Ok(())
    }

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

impl TagStructWrite for Value {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.write_value(self)
    }
}

impl TagStructWrite for Props {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.inner.write_u8(Tag::PropList as u8)?;
        for (k, v) in self.iter() {
            let k = CString::new(k.as_bytes()).map_err(|_| {
                ProtocolError::Invalid(format!("proplist key contains nul byte: {:?}", k))
            })?;
            assert!(v.len() < u32::MAX as usize);
            w.write_string(Some(k))?;
            w.write_u32(v.len() as u32)?;
            w.write_arbitrary(v)?;
        }

        w.write_null_string()?;
        Ok(())
    }
}

impl TagStructWrite for Volume {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.inner.write_u8(Tag::Volume as u8)?;
        w.inner.write_u32::<NetworkEndian>(self.as_u32())?;
        Ok(())
    }
}

impl TagStructWrite for SampleSpec {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.inner.write_u8(Tag::SampleSpec as u8)?;
        w.inner.write_u8(self.format as u8)?;
        w.inner.write_u8(self.channels)?;
        w.inner.write_u32::<NetworkEndian>(self.sample_rate)?;
        Ok(())
    }
}

impl TagStructWrite for ChannelMap {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.inner.write_u8(Tag::ChannelMap as u8)?;
        w.inner.write_u8(self.num_channels())?;
        for channel_pos in self {
            w.inner.write_u8(channel_pos as u8)?;
        }
        Ok(())
    }
}

impl TagStructWrite for FormatInfo {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.inner.write_u8(Tag::FormatInfo as u8)?;
        w.write_u8(self.encoding() as u8)?;
        w.write(self.props())?;
        Ok(())
    }
}

impl TagStructWrite for ChannelVolume {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.inner.write_u8(Tag::CVolume as u8)?;

        w.inner.write_u8(self.channels().len() as u8)?;
        for volume in self.channels() {
            w.inner.write_u32::<NetworkEndian>(volume.as_u32())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::protocol::{FormatEncoding, Prop};

    use super::*;

    #[test]
    fn test_prop_list_roundtrip() {
        let mut proplist = Props::new();
        proplist.insert("foo".into(), vec![1, 2, 3].into());
        proplist.set(Prop::ApplicationName, "bar");

        let mut buf = Vec::new();
        {
            let mut w = TagStructWriter::new(&mut buf, 0);
            w.write(&proplist).unwrap();
        }

        let mut cursor = Cursor::new(&buf);
        let mut r = TagStructReader::new(&mut cursor, 0);
        let proplist2 = r.read_proplist().unwrap();

        assert_eq!(proplist, proplist2);
    }

    #[test]
    fn test_volume_roundtrip() {
        let volume = Volume::from_linear(0.5);
        let mut buf = Vec::new();
        {
            let mut w = TagStructWriter::new(&mut buf, 0);
            w.write(volume).unwrap();
        }

        let mut cursor = Cursor::new(&buf);
        let mut r = TagStructReader::new(&mut cursor, 0);
        let volume2 = r.read_volume().unwrap();

        assert_eq!(volume, volume2);
    }

    #[test]
    fn test_sample_spec_roundtrip() {
        let spec = SampleSpec::new(SampleFormat::S16Le, 2, 44100).unwrap();
        let mut buf = Vec::new();
        {
            let mut w = TagStructWriter::new(&mut buf, 0);
            w.write(spec).unwrap();
        }

        let mut cursor = Cursor::new(&buf);
        let mut r = TagStructReader::new(&mut cursor, 0);
        let spec2 = r.read_sample_spec().unwrap();

        assert_eq!(spec, spec2);
    }

    #[test]
    fn test_channel_map_roundtrip() {
        let map = ChannelMap::new();
        let mut buf = Vec::new();
        {
            let mut w = TagStructWriter::new(&mut buf, 0);
            w.write(&map).unwrap();
        }

        let mut cursor = Cursor::new(&buf);
        let mut r = TagStructReader::new(&mut cursor, 0);
        let map2 = r.read_channel_map().unwrap();

        assert_eq!(map, map2);
    }

    #[test]
    fn test_channel_volume_roundtrip() {
        let volume = ChannelVolume::default();
        let mut buf = Vec::new();
        {
            let mut w = TagStructWriter::new(&mut buf, 0);
            w.write(&volume).unwrap();
        }

        let mut cursor = Cursor::new(&buf);
        let mut r = TagStructReader::new(&mut cursor, 0);
        let volume2 = r.read_cvolume().unwrap();

        assert_eq!(volume, volume2);
    }

    #[test]
    fn test_format_info_roundtrip() {
        let info = FormatInfo::new(FormatEncoding::Eac3Iec61937);
        let mut buf = Vec::new();
        {
            let mut w = TagStructWriter::new(&mut buf, 0);
            w.write(&info).unwrap();
        }

        let mut cursor = Cursor::new(&buf);
        let mut r = TagStructReader::new(&mut cursor, 0);
        let info2 = r.read_format_info().unwrap();

        assert_eq!(info, info2);
    }

    #[test]
    fn test_usec_roundtrip() {
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
    fn test_timeval_roundtrip() {
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
