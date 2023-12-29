//! Defines mappings from stream channels to speaker positions.

use std::fmt;

use super::*;
use crate::protocol::sample_spec::CHANNELS_MAX;
use crate::protocol::ProtocolError;

use enum_primitive_derive::Primitive;

/// Channel position labels.
#[allow(missing_docs)]
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Primitive)]
pub enum ChannelPosition {
    /// No position.
    #[default]
    Mono = 0,
    /// Apple, Dolby call this 'Left'.
    FrontLeft = 1,
    /// Apple, Dolby call this 'Right'.
    FrontRight = 2,
    /// Apple, Dolby call this 'Center'.
    FrontCenter = 3,
    /// Microsoft calls this 'Back Center', Apple calls this 'Center Surround', Dolby calls this 'Surround Rear Center'.
    RearCenter = 4,
    /// Microsoft calls this 'Back Left', Apple calls this 'Left Surround' (!), Dolby calls this 'Surround Rear Left'.
    RearLeft = 5,
    /// Microsoft calls this 'Back Right', Apple calls this 'Right Surround' (!), Dolby calls this 'Surround Rear Right'.
    RearRight = 6,
    /// Microsoft calls this 'Low Frequency', Apple calls this 'LFEScreen'.
    Lfe = 7,
    /// Apple, Dolby call this 'Left Center'.
    FrontLeftOfCenter = 8,
    /// Apple, Dolby call this 'Right Center'.
    FrontRightOfCenter = 9,
    /// Apple calls this 'Left Surround Direct', Dolby calls this 'Surround Left' (!).
    SideLeft = 10,
    /// Apple calls this 'Right Surround Direct', Dolby calls this 'Surround Right' (!).
    SideRight = 11,
    Aux0 = 12,
    Aux1 = 13,
    Aux2 = 14,
    Aux3 = 15,
    Aux4 = 16,
    Aux5 = 17,
    Aux6 = 18,
    Aux7 = 19,
    Aux8 = 20,
    Aux9 = 21,
    Aux10 = 22,
    Aux11 = 23,
    Aux12 = 24,
    Aux13 = 25,
    Aux14 = 26,
    Aux15 = 27,
    Aux16 = 28,
    Aux17 = 29,
    Aux18 = 30,
    Aux19 = 31,
    Aux20 = 32,
    Aux21 = 33,
    Aux22 = 34,
    Aux23 = 35,
    Aux24 = 36,
    Aux25 = 37,
    Aux26 = 38,
    Aux27 = 39,
    Aux28 = 40,
    Aux29 = 41,
    Aux30 = 42,
    Aux31 = 43,
    /// Apple calls this 'Top Center Surround'.
    TopCenter = 44,
    /// Apple calls this 'Vertical Height Left'.
    TopFrontLeft = 45,
    /// Apple calls this 'Vertical Height Right'.
    TopFrontRight = 46,
    /// Apple calls this 'Vertical Height Center'.
    TopFrontCenter = 47,
    /// Microsoft and Apple call this 'Top Back Left'.
    TopRearLeft = 48,
    /// Microsoft and Apple call this 'Top Back Right'.
    TopRearRight = 49,
    /// Microsoft and Apple call this 'Top Back Center'.
    TopRearCenter = 50,
}

/// A map from stream channels to speaker positions.
///
/// These values are relevant for conversion and mixing of streams.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ChannelMap {
    /// Number of channels in the map.
    channels: u8,
    /// Channel position map.
    map: [ChannelPosition; CHANNELS_MAX as usize],
}

impl Default for ChannelMap {
    fn default() -> Self {
        Self::mono()
    }
}

// FIXME are empty channel maps accepted by PA?

impl ChannelMap {
    /// Creates an empty channel map.
    pub fn empty() -> Self {
        ChannelMap {
            channels: 0,
            map: [Default::default(); CHANNELS_MAX as usize],
        }
    }

    /// Creates a channel map with a single channel.
    pub fn mono() -> Self {
        Self {
            channels: 1,
            map: [Default::default(); CHANNELS_MAX as usize],
        }
    }

    /// Creates a channel map with two channels in the standard stereo positions.
    pub fn stereo() -> Self {
        let mut map = Self::empty();
        map.push(ChannelPosition::FrontLeft);
        map.push(ChannelPosition::FrontRight);
        map
    }

    /// Tries to append another `ChannelPosition` to the end of this map.
    ///
    /// Panics if the map already has CHANNEL_MAX channels.
    pub fn push(&mut self, position: ChannelPosition) {
        if self.channels < CHANNELS_MAX {
            self.map[self.channels as usize] = position;
            self.channels += 1;
        } else {
            panic!("channel map full");
        }
    }

    /// Returns the number of channel mappings stored in this `ChannelMap`.
    pub fn num_channels(&self) -> u8 {
        self.channels
    }
}

impl fmt::Debug for ChannelMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Only print the occupied part of the backing storage
        self.map[..self.channels.into()].fmt(f)
    }
}

impl<'a> IntoIterator for &'a ChannelMap {
    type Item = ChannelPosition;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        Iter { map: self, next: 0 }
    }
}

/// An iterator over `ChannelPosition`s stored in a `ChannelMap`.
#[derive(Debug)]
pub struct Iter<'a> {
    map: &'a ChannelMap,
    next: u8,
}

impl Iterator for Iter<'_> {
    type Item = ChannelPosition;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.next < self.map.num_channels() {
            self.next += 1;
            Some(self.map.map[self.next as usize - 1])
        } else {
            None
        }
    }
}

impl TagStructRead for ChannelMap {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        ts.expect_tag(Tag::ChannelMap)?;

        let channels = ts.inner.read_u8()?;
        if channels > CHANNELS_MAX {
            return Err(ProtocolError::Invalid(format!(
                "channel map too large (max is {} channels, got {})",
                CHANNELS_MAX, channels
            )));
        }

        let mut map = ChannelMap::empty();
        for _ in 0..channels {
            let raw = ts.inner.read_u8()?;
            map.push(ChannelPosition::from_u8(raw).ok_or_else(|| {
                ProtocolError::Invalid(format!("invalid channel position {}", raw))
            })?)
        }

        Ok(map)
    }
}

impl TagStructWrite for ChannelMap {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.inner.write_u8(Tag::ChannelMap as u8)?;
        w.inner.write_u8(self.num_channels())?;
        for channel_pos in self {
            w.inner.write_u8(channel_pos as u8)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::{test_util::test_serde_version, MAX_VERSION};

    use super::*;

    #[test]
    fn roundtrip() -> anyhow::Result<()> {
        let mut map = ChannelMap::empty();
        map.push(ChannelPosition::FrontLeft);
        map.push(ChannelPosition::FrontRight);
        map.push(ChannelPosition::RearLeft);
        map.push(ChannelPosition::RearRight);

        test_serde_version(&map, MAX_VERSION)
    }
}
