//! Sample specification data type.

use enum_primitive_derive::Primitive;

use crate::protocol::ProtocolError;

/// Maximum number of channels.
pub const CHANNELS_MAX: u8 = 32;

const RATE_MAX: u32 = 48000 * 8;

/// Describes how individual samples are encoded.
#[derive(Debug, Copy, Clone, Primitive, PartialEq, Eq, Default)]
pub enum SampleFormat {
    /// Unsigned 8 Bit PCM
    U8 = 0,
    /// 8 Bit a-Law
    Alaw = 1,
    /// 8 Bit mu-Law
    Ulaw = 2,
    /// Signed 16 Bit PCM, little endian (PC)
    #[default]
    S16Le = 3,
    /// Signed 16 Bit PCM, big endian
    S16Be = 4,
    /// 32 Bit IEEE floating point, little endian (PC), range -1.0 to 1.0
    Float32Le = 5,
    /// 32 Bit IEEE floating point, big endian, range -1.0 to 1.0
    Float32Be = 6,
    /// Signed 32 Bit PCM, little endian (PC)
    S32Le = 7,
    /// Signed 32 Bit PCM, big endian
    S32Be = 8,
    /// Signed 24 Bit PCM packed, little endian (PC). \since 0.9.15
    S24Le = 9,
    /// Signed 24 Bit PCM packed, big endian. \since 0.9.15
    S24Be = 10,
    /// Signed 24 Bit PCM in LSB of 32 Bit words, little endian (PC). \since 0.9.15
    S24In32Le = 11,
    /// Signed 24 Bit PCM in LSB of 32 Bit words, big endian. \since 0.9.15
    S24In32Be = 12,
}

/// A sample specification that fully describes the format of a sample stream between 2 endpoints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SampleSpec {
    /// Format / Encoding of individual samples.
    pub format: SampleFormat,
    /// Number of independent channels. Must be at least 1.
    pub channels: u8,
    /// Number of samples per second (and per channel).
    pub sample_rate: u32,
}

impl SampleSpec {
    /// Creates a new sample specification.
    ///
    /// # Parameters
    ///
    /// * `format`: Format of individual samples.
    /// * `channels`: Number of independent channels (must be at least 1).
    /// * `sample_rate`: Samples per second and per channel.
    pub fn new(
        format: SampleFormat,
        channels: u8,
        sample_rate: u32,
    ) -> Result<Self, ProtocolError> {
        if channels == 0 || channels > CHANNELS_MAX {
            return Err(ProtocolError::Invalid(format!(
                "invalid channel count {} (must be between 1 and {})",
                channels, CHANNELS_MAX
            )));
        }

        if sample_rate == 0 || sample_rate > RATE_MAX * 101 / 100 {
            // PA says: "The extra 1% is due to module-loopback: it temporarily sets a
            // higher-than-nominal rate to get rid of excessive buffer latency"
            // TODO: We might get away without this workaround
            return Err(ProtocolError::Invalid(format!(
                "invalid sample rate {} (must be between 1 and {})",
                sample_rate, RATE_MAX
            )));
        }

        Ok(Self {
            format,
            channels,
            sample_rate,
        })
    }

    /// Modifies a `SampleSpec` to be compatible with a different `protocol_version` so that older
    /// clients can understand it.
    pub fn protocol_downgrade(self, protocol_version: u16) -> SampleSpec {
        use self::SampleFormat::*;

        let mut fixed = self;

        // proto>=12 (always holds)
        if protocol_version < 15 {
            // S24 samples were added in version 15, downgrade them so somthing vaguely similar
            match fixed.format {
                S24Le | S24In32Le => fixed.format = Float32Le,
                S24Be | S24In32Be => fixed.format = Float32Be,
                _ => {} // no fixup needed
            }
        }

        fixed
    }
}
