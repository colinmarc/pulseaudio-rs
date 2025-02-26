//! Sample specification data type.

use enum_primitive_derive::Primitive;
use num_traits::Euclid;

use super::*;
use crate::protocol::ProtocolError;

/// PA_RATE_MAX from the Pulse source. This is the maximum sample rate, in Hz.
pub const MAX_RATE: u32 = 48000 * 16;

/// PA_CHANNEL_MAX from the Pulse source. This is the maximum number of channels
/// supported for streams.
pub const MAX_CHANNELS: u8 = 32;

/// Describes how individual samples are encoded.
#[derive(Debug, Copy, Clone, Primitive, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SampleFormat {
    /// Invalid or unspecified.
    #[default]
    Invalid = u8::MAX,
    /// Unsigned 8 Bit PCM
    U8 = 0,
    /// 8 Bit a-Law
    Alaw = 1,
    /// 8 Bit mu-Law
    Ulaw = 2,
    /// Signed 16 Bit PCM, little endian (PC)
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
            SampleFormat::Invalid => 0,
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
    /// For a given byte length, calculates how many samples it contains,
    /// divided by the sample rate.
    pub fn bytes_to_duration(&self, len: usize) -> time::Duration {
        let frames = len / self.format.bytes_per_sample() / self.channels as usize;
        let (secs, rem) = (frames as u32).div_rem_euclid(&self.sample_rate);

        const NANOS_PER_SECOND: u64 = 1_000_000_000;
        let nanos = (rem as u64 * NANOS_PER_SECOND) / self.sample_rate as u64;

        time::Duration::new(secs as u64, nanos as u32)
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
    fn bytes_to_duration() {
        // 2 bytes per sample, 4 bytes per frame, 48 frames per millisecond.
        let spec = SampleSpec {
            format: SampleFormat::S16Le,
            channels: 2,
            sample_rate: 48000,
        };

        assert_eq!(spec.bytes_to_duration(48000 * 4).as_millis(), 1000);
        assert_eq!(spec.bytes_to_duration(1920).as_millis(), 10);

        // Attempt to trigger an overflow.
        assert_eq!(spec.bytes_to_duration(usize::MAX).as_millis(), 89478485);
        assert_eq!(spec.bytes_to_duration((48000 * 400) - 1).as_millis(), 99999);
    }

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
