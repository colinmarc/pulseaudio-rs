//! Defines types that specify how samples are encoded.

use enum_primitive_derive::Primitive;

use num_traits::FromPrimitive;

use crate::protocol::ProtocolError;

use super::Props;

/// Describes how samples are encoded.
#[derive(Debug, Copy, Clone, Primitive, PartialEq, Eq)]
pub enum FormatEncoding {
    /// Any encoding is supported.
    Any = 0,
    /// Good old PCM.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatInfo {
    encoding: FormatEncoding,
    props: Props,
}

impl FormatInfo {
    /// Create a new `FormatInfo` from a sample encoding with an empty property list.
    pub fn new(encoding: FormatEncoding) -> Self {
        Self {
            encoding,
            props: Props::new(),
        }
    }

    /// Create a `FormatInfo` from raw data parsed from a tagstruct.
    ///
    /// # Parameters
    ///
    /// * `encoding`: Raw value for a `FormatEncoding`.
    /// * `props`: Property list to associate with the `FormatInfo`.
    pub fn from_raw(encoding: u8, props: Props) -> Result<Self, ProtocolError> {
        let encoding = FormatEncoding::from_u8(encoding).ok_or(ProtocolError::Invalid(format!(
            "invalid encoding: {}",
            encoding
        )))?;

        Ok(Self { encoding, props })
    }

    /// Get the actual sample encoding.
    pub fn encoding(&self) -> FormatEncoding {
        self.encoding
    }

    /// Get a reference to the property list for this `FormatInfo` object.
    pub fn props(&self) -> &Props {
        &self.props
    }

    /// Get a mutable reference to the property list for this `FormatInfo` object.
    pub fn props_mut(&mut self) -> &mut Props {
        &mut self.props
    }
}
