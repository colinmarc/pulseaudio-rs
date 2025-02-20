//! Defines volume specification data types.

use std::fmt;
use std::slice;

use byteorder::NetworkEndian;

use crate::protocol::ProtocolError;

use super::sample_spec::CHANNELS_MAX;
use super::*;

const VOLUME_NORM: u32 = 0x10000;
const VOLUME_MUTED: u32 = 0;
const VOLUME_MAX: u32 = u32::MAX / 2;

/// Volume specification for a single channel.
// TODO: Document linearity and conversion to/from that and dB (apparently conversion to dB is only valid if the sink says so!)
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct Volume(u32);

impl Volume {
    /// The normal volume (100%, 0 dB, no attenuation, no amplification).
    pub const NORM: Self = Volume(VOLUME_NORM);

    /// The muted volume (0%, -Inf dB).
    pub const MUTED: Self = Volume(VOLUME_MUTED);

    /// Gets the raw volume value as a `u32`.
    ///
    /// This is not useful for user presentation.
    pub fn as_u32(&self) -> u32 {
        self.0
    }

    /// Creates a volume specification from a raw `u32` sent over the wire (or obtained via
    /// `Volume::as_u32`).
    ///
    /// If the raw value is out of the valid range, it will be clamped.
    pub fn from_u32_clamped(raw: u32) -> Self {
        Volume(raw.min(VOLUME_MAX))
    }

    /// Gets the amplification/attenuation in decibel (dB) corresponding to this volume.
    pub fn to_db(&self) -> f32 {
        self.to_linear().log10() * 20.0
    }

    /// Convert the volume to a linear volume.
    ///
    /// The range of the returned number goes from 0.0 (mute) over 1.0 (0 dB, 100%) and can go
    /// beyond 1.0 to indicate that the signal should be amplified.
    pub fn to_linear(&self) -> f32 {
        // Like PulseAudio, we use a cubic scale.
        // Also see: http://www.robotplanet.dk/audio/audio_gui_design/
        let f = self.0 as f32 / VOLUME_NORM as f32;
        f * f * f
    }

    /// Convert from a linear volume.
    ///
    /// Volumes outside the valid range will be clamped.
    pub fn from_linear(linear: f32) -> Self {
        let raw = (linear.cbrt() * VOLUME_NORM as f32) as u32;
        Volume(match raw {
            _ if raw > VOLUME_MAX => VOLUME_MAX,
            _ => raw,
        })
    }
}

impl fmt::Display for Volume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} dB", self.to_db())
    }
}

impl fmt::Debug for Volume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Volume")
            .field(&format!(
                "raw={:.1}, linear={:.1}, {:.1} dB",
                self.0 as f32 / VOLUME_NORM as f32,
                self.to_linear(),
                self.to_db()
            ))
            .finish()
    }
}

impl TagStructRead for Volume {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        ts.expect_tag(Tag::Volume)?;
        Ok(Volume::from_u32_clamped(
            ts.inner.read_u32::<NetworkEndian>()?,
        ))
    }
}

impl TagStructWrite for Volume {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.inner.write_u8(Tag::Volume as u8)?;
        w.inner.write_u32::<NetworkEndian>(self.as_u32())?;
        Ok(())
    }
}

/// Per-channel volume setting.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ChannelVolume {
    channels: u8,
    volumes: [Volume; CHANNELS_MAX as usize],
}

impl Default for ChannelVolume {
    fn default() -> Self {
        Self {
            channels: 1,
            volumes: [Volume::MUTED; CHANNELS_MAX as usize],
        }
    }
}

// TODO: empty cvolumes are invalid!
impl ChannelVolume {
    /// Creates an empty `ChannelVolume` specifying no volumes for any channel.
    pub fn empty() -> Self {
        Self {
            channels: 0,
            volumes: [Volume::MUTED; CHANNELS_MAX as usize],
        }
    }

    /// Create a `ChannelVolume` with N channels, all muted.
    pub fn muted(channels: usize) -> ChannelVolume {
        Self {
            channels: channels as u8,
            volumes: [Volume::MUTED; CHANNELS_MAX as usize],
        }
    }

    /// Create a `ChannelVolume` with N channels, all at full volume.
    pub fn norm(channels: usize) -> ChannelVolume {
        Self {
            channels: channels as u8,
            volumes: [Volume::NORM; CHANNELS_MAX as usize],
        }
    }

    /// Append a new volume to the list.
    ///
    /// Returns the channel for which the volume was added.
    pub fn push(&mut self, volume: Volume) {
        if self.channels < CHANNELS_MAX {
            self.volumes[self.channels as usize] = volume;
            self.channels += 1;
        }
    }

    /// Returns the number of channel volumes stored in `self`.
    pub fn channels(&self) -> &[Volume] {
        &self.volumes[..self.channels as usize]
    }
}

impl fmt::Debug for ChannelVolume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Only print the occupied part of the backing storage
        self.volumes[..self.channels.into()].fmt(f)
    }
}

/// Iterator over volumes stored in a `CVolume`.
#[derive(Debug)]
pub struct Iter<'a> {
    inner: slice::Iter<'a, Volume>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Volume;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        self.inner.next()
    }
}

impl TagStructRead for ChannelVolume {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        ts.expect_tag(Tag::CVolume)?;
        let n_channels = ts.inner.read_u8()?;
        if n_channels == 0 || n_channels > CHANNELS_MAX {
            return Err(ProtocolError::Invalid(format!(
                "invalid cvolume channel count {}, must be between 1 and {}",
                n_channels, CHANNELS_MAX
            )));
        }

        let mut cvolume = ChannelVolume::empty();
        for _ in 0..n_channels {
            let raw = ts.inner.read_u32::<NetworkEndian>()?;
            cvolume.push(Volume::from_u32_clamped(raw))
        }

        Ok(cvolume)
    }
}

impl TagStructWrite for ChannelVolume {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
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
    use crate::protocol::{test_util::test_serde_version, MAX_VERSION};

    use super::*;

    use std::f32;

    #[test]
    fn volume_serde() -> anyhow::Result<()> {
        let v = Volume::from_linear(0.5);
        test_serde_version(&v, MAX_VERSION)?;
        Ok(())
    }

    #[test]
    fn cvolume_serde() -> anyhow::Result<()> {
        let mut cv = ChannelVolume::default();
        cv.push(Volume::from_linear(0.5));
        cv.push(Volume::from_linear(0.5));
        test_serde_version(&cv, MAX_VERSION)?;
        Ok(())
    }

    #[test]
    fn volume_conversions() {
        assert_eq!(Volume::NORM.to_linear(), 1.0);
        assert_eq!(Volume::MUTED.to_linear(), 0.0);
        assert_eq!(Volume::from_linear(-43.0).to_linear(), 0.0);
        assert_eq!(Volume::NORM.to_db(), 0.0);
        assert_eq!(Volume::MUTED.to_db(), -f32::INFINITY);
    }

    // TODO quickcheck to_linear from_linear round trip
}
