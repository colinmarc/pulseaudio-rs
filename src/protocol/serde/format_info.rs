//! Defines types that specify how samples are encoded.

use enum_primitive_derive::Primitive;

use num_traits::FromPrimitive;

use super::*;
use crate::protocol::ProtocolError;

/// Describes how samples are encoded.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Primitive)]
pub enum FormatEncoding {
    /// Any encoding is supported.
    Any = 0,
    /// Good old PCM.
    #[default]
    Pcm = 1,
    /// AC3 data encapsulated in IEC 61937 header/padding.
    Ac3Iec61937 = 2,
    /// EAC3 data encapsulated in IEC 61937 header/padding.
    Eac3Iec61937 = 3,
    /// MPEG-1 or MPEG-2 (Part 3, not AAC) data encapsulated in IEC 61937 header/padding.
    MpegIec61937 = 4,
    /// DTS data encapsulated in IEC 61937 header/padding.
    DtsIec61937 = 5,
    /// MPEG-2 AAC data encapsulated in IEC 61937 header/padding. \since 4.0
    Mpeg2Iec61937 = 6,
    // TODO extensible
}

/// Sample encoding info.
///
/// Associates a simple `FormatEncoding` with a list of arbitrary properties.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormatInfo {
    /// The sample encoding.
    pub encoding: FormatEncoding,

    /// Key-value properties for the format.
    pub props: Props,
}

impl FormatInfo {
    /// Create a new `FormatInfo` from a sample encoding with an empty property list.
    pub fn new(encoding: FormatEncoding) -> Self {
        Self {
            encoding,
            props: Props::new(),
        }
    }
}

impl TagStructRead for FormatInfo {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        ts.expect_tag(Tag::FormatInfo)?;

        let encoding = ts.read_u8()?;
        let encoding = FormatEncoding::from_u8(encoding).ok_or(ProtocolError::Invalid(format!(
            "invalid format encoding: 0x{:2x}",
            encoding
        )))?;
        let props = ts.read()?;
        Ok(Self { encoding, props })
    }
}

impl TagStructWrite for FormatInfo {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.inner.write_u8(Tag::FormatInfo as u8)?;
        w.write_u8(self.encoding as u8)?;
        w.write(&self.props)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn format_info_serde() -> anyhow::Result<()> {
        test_serde(&FormatInfo::new(FormatEncoding::Ac3Iec61937))
    }
}
