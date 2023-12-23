//! Defines mappings from stream channels to speaker positions.

use std::fmt;

use crate::protocol::sample_spec::CHANNELS_MAX;

use enum_primitive_derive::Primitive;

/// Channel position labels.
#[derive(Debug, Copy, Clone, Primitive, PartialEq, Eq)]
pub enum ChannelPosition {
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
#[derive(Clone, PartialEq, Eq)]
pub struct ChannelMap {
    /// Number of channels in the map.
    channels: u8,
    /// Channel position map.
    map: [ChannelPosition; CHANNELS_MAX as usize],
}

impl Default for ChannelMap {
    fn default() -> Self {
        Self {
            channels: 0,
            map: [ChannelPosition::Mono; CHANNELS_MAX as usize],
        }
    }
}

// FIXME are empty channel maps accepted by PA?

impl ChannelMap {
    /// Creates an empty channel map.
    pub fn new() -> Self {
        Default::default()
    }

    /// Tries to append another `ChannelPosition` to the end of this map.
    ///
    /// If the map is already at max. capacity, returns a `MapFullError`.
    pub fn push(&mut self, position: ChannelPosition) -> Result<(), MapFullError> {
        *(self
            .map
            .get_mut(self.channels as usize)
            .ok_or(MapFullError {})?) = position;
        self.channels += 1;
        Ok(())
    }

    /// Returns the number of channel mappings stored in this `ChannelMap`.
    pub fn num_channels(&self) -> u8 {
        self.channels
    }
}

impl fmt::Debug for ChannelMap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

impl<'a> Iterator for Iter<'a> {
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

/// An error indicating that a channel map is already full and cannot be extended.
#[derive(Debug)]
pub struct MapFullError {}
