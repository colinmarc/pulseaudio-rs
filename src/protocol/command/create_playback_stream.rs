use crate::protocol::stream::{BufferAttr, StreamFlags};
use crate::protocol::{tagstruct::*, ProtocolError};
use crate::protocol::{ChannelMap, ChannelVolume, PropList, SampleSpec};

use std::ffi::{CStr, CString};
use std::u32;

const INVALID_INDEX: u32 = u32::MAX;

/// Specifies a sink to connect a playback stream to.
///
/// Said sink might not exist (in which case a `NoEntity` error should be returned).
#[derive(Debug)]
pub enum SinkSpec<'a> {
    /// Sink index.
    ///
    /// PA specifies `u32::MAX` as an invalid index, which does not occur here.
    Index(u32),
    /// Named sink.
    Name(&'a CStr),
}

#[derive(Debug)]
struct CreatePlaybackStreamParams<'a> {
    /// Stream properties to set (such as the media name).
    stream_props: PropList,
    sample_spec: SampleSpec,
    channel_map: ChannelMap,
    stream_flags: StreamFlags,
    sink_spec: Option<SinkSpec<'a>>,
    /// Whether to start the stream in muted or unmuted state.
    ///
    /// `None` means no preference and the server should decide.
    muted: Option<bool>,
    /// Set the channel volumes.
    volume: Option<ChannelVolume>,
    syncid: u32,
}

/// Parameters for `CreatePlaybackStream` command.
#[derive(Debug)]
pub struct CreatePlaybackStream<'a> {
    inner: Box<CreatePlaybackStreamParams<'a>>,
}

impl<'a> CreatePlaybackStream<'a> {
    pub fn stream_props(&self) -> &PropList {
        &self.inner.stream_props
    }

    pub fn stream_flags(&self) -> StreamFlags {
        self.inner.stream_flags
    }

    pub fn sample_spec(&self) -> &SampleSpec {
        &self.inner.sample_spec
    }

    pub fn channel_map(&self) -> &ChannelMap {
        &self.inner.channel_map
    }

    /// Get the sink specification.
    ///
    /// This tells the server which sink to connect the stream to. If `None`, the stream will be
    /// connected to the default sink.
    pub fn sink_spec(&self) -> Option<&SinkSpec> {
        self.inner.sink_spec.as_ref()
    }

    /// Get the stream mute preference.
    ///
    /// * `None`: No preference, let the server decide.
    /// * `Some(true)`: Create the stream in muted state.
    /// * `Some(false)`: Create the stream in unmuted state.
    pub fn muted(&self) -> Option<bool> {
        self.inner.muted
    }

    pub fn volume(&self) -> Option<&ChannelVolume> {
        self.inner.volume.as_ref()
    }

    /// Get the specified sync ID group.
    ///
    /// All streams with the same sync ID are synced with each other. They all have the same sink.
    pub fn sync_id(&self) -> u32 {
        self.inner.syncid
    }
}

impl<'a> TagStructRead<'a> for CreatePlaybackStream<'a> {
    fn read(ts: &mut TagStructReader<'a>, protocol_version: u16) -> Result<Self, ProtocolError> {
        // This one contains a *lot* of stuff, and many flags that were added over time.
        let (sample_spec, channel_map, sink_index, sink_name, syncid, cvolume, muted);
        let mut muted_set = false; // whether to set muted state (since proto 15)
        let mut volume_set = true;
        let mut stream_flags = StreamFlags::empty();
        let mut buf_attr = BufferAttr::default();
        let mut formats = Vec::new();

        // only valid for protocol>=13
        sample_spec = ts.read_sample_spec()?;
        channel_map = ts.read_channel_map()?;
        sink_index = ts.read_u32()?;
        sink_name = ts.read_string()?;
        buf_attr.maxlength = ts.read_u32()?;
        stream_flags.set(StreamFlags::START_CORKED, ts.read_bool()?);
        buf_attr.tlength = ts.read_u32()?;
        buf_attr.prebuf = ts.read_u32()?;
        buf_attr.minreq = ts.read_u32()?;
        syncid = ts.read_u32()?;
        cvolume = ts.read_cvolume()?;
        // (cvolume must contain at least 1 volume even if set_volume is false)

        // only valid for proto>=12
        stream_flags.set(StreamFlags::NO_REMAP_CHANNELS, ts.read_bool()?);
        stream_flags.set(StreamFlags::NO_REMIX_CHANNELS, ts.read_bool()?);
        stream_flags.set(StreamFlags::FIX_FORMAT, ts.read_bool()?);
        stream_flags.set(StreamFlags::FIX_RATE, ts.read_bool()?);
        stream_flags.set(StreamFlags::FIX_CHANNELS, ts.read_bool()?);
        stream_flags.set(StreamFlags::DONT_MOVE, ts.read_bool()?);
        stream_flags.set(StreamFlags::VARIABLE_RATE, ts.read_bool()?);
        // proto>=13
        muted = ts.read_bool()?;
        stream_flags.set(StreamFlags::ADJUST_LATENCY, ts.read_bool()?);
        let stream_props = ts.read_proplist()?;

        if protocol_version >= 14 {
            volume_set = ts.read_bool()?;
            stream_flags.set(StreamFlags::EARLY_REQUESTS, ts.read_bool()?);
        }
        if protocol_version >= 15 {
            muted_set = ts.read_bool()?;
            stream_flags.set(StreamFlags::DONT_INHIBIT_AUTO_SUSPEND, ts.read_bool()?);
            stream_flags.set(StreamFlags::FAIL_ON_SUSPEND, ts.read_bool()?);
        }
        if protocol_version >= 17 {
            stream_flags.set(StreamFlags::RELATIVE_VOLUME, ts.read_bool()?);
        }
        if protocol_version >= 18 {
            stream_flags.set(StreamFlags::PASSTHROUGH, ts.read_bool()?);
        }
        if protocol_version >= 21 {
            for _ in 0..ts.read_u8()? {
                formats.push(ts.read_format_info()?);
            }
            // Client have a choice here: Either send no format infos, but a sample spec and channel
            // map, or send at least 1 format info and any kind of sample spec/channel map
            // TODO: (including invalid ones).
        }

        let sink_spec = match (sink_index, sink_name) {
            (INVALID_INDEX, None) => None, // default sink
            (INVALID_INDEX, Some(name)) => Some(SinkSpec::Name(name)),
            (index, None) => Some(SinkSpec::Index(index)),
            (_index, Some(_name)) => {
                // PA rejects this as well
                return Err(ProtocolError::Invalid(
                    "cannot specify both sink index and name".to_string(),
                ));
            }
        };

        // `muted_set` tells us whether to actually change the muted flag, but it only
        // exists since proto 15, so set it to `true` when `muted` is `true`.
        if muted {
            muted_set = true;
        }

        // Build the muting preference
        // (None = no pref, Some(true) = plz mute, Some(false) = plz unmute)
        let muted = match (muted_set, muted) {
            (true, muted) => Some(muted),
            (false, _) => None,
        };

        let volume = if volume_set { Some(cvolume) } else { None };

        Ok(Self {
            inner: Box::new(CreatePlaybackStreamParams {
                stream_props,
                sample_spec,
                channel_map,
                stream_flags,
                sink_spec,
                muted,
                volume,
                syncid,
            }),
        })
    }
}

impl<'a> TagStructWrite for CreatePlaybackStream<'a> {
    fn write(&self, _w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        unimplemented!() // TODO
    }
}

#[derive(Debug)]
pub struct CreatePlaybackStreamReply {
    /// Server-internal stream index.
    pub stream_index: u32,
    pub sink_input_index: u32,
    /// Number of bytes that can be written to the playback buffer.
    pub missing: u32,
    /// Attributes of the created buffer.
    pub buffer_metrics: BufferAttr,
    /// Actually chosen sample specs.
    pub sample_spec: SampleSpec,
    /// Actually chosen channel map.
    pub channel_map: ChannelMap,
    /// Stream latency in microseconds.
    pub stream_latency: u64,
    /// Server-internal sink index.
    pub sink_index: u32,
    /// Name of the sink.
    pub sink_name: CString,
    /// Whether the sink is suspended.
    pub sink_suspended: bool,
}

impl TagStructWrite for CreatePlaybackStreamReply {
    fn write(&self, w: &mut TagStructWriter, protocol_version: u16) -> Result<(), ProtocolError> {
        w.write(self.stream_index);
        w.write(self.sink_input_index);
        w.write(self.missing);
        // proto>=9
        w.write(self.buffer_metrics.maxlength);
        w.write(self.buffer_metrics.tlength);
        w.write(self.buffer_metrics.prebuf);
        w.write(self.buffer_metrics.minreq);
        // proto>=12
        w.write(self.sample_spec);
        w.write(&self.channel_map.clone());
        w.write(self.sink_index);
        w.write(&self.sink_name);
        w.write(self.sink_suspended);
        // proto>=13
        w.write(self.stream_latency);

        if protocol_version >= 21 {
            // Send back the sample format of the sink
        }
        unimplemented!()
    }
}
