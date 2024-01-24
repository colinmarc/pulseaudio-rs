//! Sample specification data type.

use enum_primitive_derive::Primitive;

use super::*;
use crate::protocol::ProtocolError;

/// Maximum number of channels.
pub const CHANNELS_MAX: u8 = 32;

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

impl SampleFormat {
    /// Returns the number of bytes used to store a single sample.
    pub fn bytes_per_sample(&self) -> usize {
        match self {
            SampleFormat::U8 => 1,
            SampleFormat::Alaw => 1,
            SampleFormat::Ulaw => 1,
            SampleFormat::S16Le => 2,
            SampleFormat::S16Be => 2,
            SampleFormat::Float32Le => 4,
            SampleFormat::Float32Be => 4,
            SampleFormat::S32Le => 4,
            SampleFormat::S32Be => 4,
            SampleFormat::S24Le => 3,
            SampleFormat::S24Be => 3,
            SampleFormat::S24In32Le => 4,
            SampleFormat::S24In32Be => 4,
        }
    }
}

/// A sample specification that fully describes the format of a sample stream between 2 endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleSpec {
    /// Format / Encoding of individual samples.
    pub format: SampleFormat,
    /// Number of independent channels.
    pub channels: u8,
    /// Number of samples per second (and per channel).
    pub sample_rate: u32,
}

impl SampleSpec {
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

impl Default for SampleSpec {
    fn default() -> Self {
        Self {
            format: SampleFormat::default(),
            channels: 1,
            sample_rate: 44100,
        }
    }
}

impl TagStructRead for SampleSpec {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        ts.expect_tag(Tag::SampleSpec)?;
        let format = ts.inner.read_u8()?;
        let format = SampleFormat::from_u8(format)
            .ok_or_else(|| ProtocolError::Invalid(format!("invalid sample format {}", format)))?;
        let channels = ts.inner.read_u8()?;
        let sample_rate = ts.inner.read_u32::<NetworkEndian>()?;

        Ok(Self {
            format,
            channels,
            sample_rate,
        })
    }
}

impl TagStructWrite for SampleSpec {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.inner.write_u8(Tag::SampleSpec as u8)?;
        w.inner.write_u8(self.format as u8)?;
        w.inner.write_u8(self.channels)?;
        w.inner.write_u32::<NetworkEndian>(self.sample_rate)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::test_util::test_serde;

    use super::*;

    #[test]
    fn sample_spec_serde() -> anyhow::Result<()> {
        let spec = SampleSpec {
            format: SampleFormat::S16Le,
            channels: 2,
            sample_rate: 44100,
        };

        test_serde(&spec)
    }
}
