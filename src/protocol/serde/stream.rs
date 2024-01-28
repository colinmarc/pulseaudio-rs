//! A stream connects a source and a sink.
//!
//! A stream connected to a source is one of the sources "source outputs", a stream connected to a
//! sink is one of the sinks "sink inputs".
//!

// FIXME: docs are copied from C source

use enum_primitive_derive::Primitive;

use super::*;

/// The direction of a stream.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Primitive)]
pub enum StreamDirection {
    /// No direction.
    None = 0,
    /// Playback stream.
    Playback = 1,
    /// Record stream.
    Record = 2,
    /// Sample upload stream.
    Upload = 3,
}

/// Stream configuration flags.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub struct StreamFlags {
    /// Create the stream in the corked state.
    pub start_corked: bool,

    /// Don't remap channels by their name, instead map them simply by their
    /// index. Implies `no_remix_channels`.
    pub no_remap_channels: bool,

    /// When remapping channels by name, don't upmix or downmix them to
    /// related channels. Copy them into matching channels of the device
    /// 1:1.
    pub no_remix_channels: bool,

    /// Use the sample format of the sink/device this stream is being
    /// connected to, and possibly ignore the format in the passed sample
    /// spac -- but you still have to pass a valid value in it as a hint to
    /// PulseAudio what would suit your stream best. If this is used, you
    /// should query the used sample format after creating the stream.
    pub fix_format: bool,

    /// Use the sample rate of the sink, and ignore the rate in the passed
    /// sample spec. The usage is similar to `fix_format`.
    pub fix_rate: bool,

    /// Use the number of channels and the channel map of the sink, and
    /// ignore the passed map. The usage is similar to `fix_format`.
    pub fix_channels: bool,

    /// Don't allow moving of this stream to another sink/device. This might
    /// be useful if you use any of the `fix_` flags, and want to make sure
    /// that resampling never takes place -- which might happen if the
    /// stream is moved to another sink/source with a different sample spec
    /// or channel map.
    pub no_move: bool,

    /// Allow dynamic changing of the sampling rate during playback.
    pub variable_rate: bool,

    /// Find peaks instead of resampling.
    pub peak_detect: bool,

    /// Create the stream in a muted/unmuted state. If None, it is left to
    /// the server to decide whether the stream starts muted.
    pub start_muted: Option<bool>,

    /// Try to adjust the latency of the sink/source based on the requested
    /// buffer metrics and adjust buffer metrics accordingly. This option
    /// may not be specified at the same time as `early_requests`.
    pub adjust_latency: bool,

    /// Enable compatibility mode for legacy clients that rely on a
    /// "classic" hardware device fragment-style playback model. If this
    /// option is set, the minreq value of the buffer metrics gets a new
    /// meaning: instead of just specifying that no requests asking for less
    /// new data than this value will be made to the client it will also
    /// guarantee that requests are generated as early as this limit is
    /// reached. This flag should only be set in very few situations where
    /// compatibility with a fragment-based playback model needs to be kept
    /// and the client applications cannot deal with data requests that are
    /// delayed to the latest moment possible. (Usually these are programs
    /// that use usleep() or a similar call in their playback loops instead
    /// of sleeping on the device itself.) This option may ot be specified
    /// at the same time as `adjust_latency` is set on the matching [`StreamFlags`].
    pub early_requests: bool,

    /// If set, this stream won't be taken into account when the server
    /// checks whether the device this stream is connected to should
    /// auto-suspend.
    pub no_inhibit_auto_suspend: bool,

    /// If the sink/source this stream is connected to is suspended
    /// during the creation of this stream, cause it to fail. If the
    /// sink/source is suspended during creation of this stream, make
    /// sure this stream is terminated.
    pub fail_on_suspend: bool,

    /// If a volume is passed when this stream is created, consider it
    /// relative to the sink's current volume, not as absolute device
    /// volume. If this is not specified, the volume will be considered
    /// absolute if the sink is in flat volume mode, and relative otherwise.
    pub relative_volume: bool,

    /// Used to tag content that will be rendered by passthrough sinks.
    /// The data will be left as is and not reformatted or resampled.
    pub passthrough: bool,
}

/// Playback and record buffer settings.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BufferAttr {
    /// Maximum length of the buffer in bytes. Setting this to `u32::MAX` will
    /// initialize this to the maximum value supported by server, which is
    /// recommended.
    ///
    /// In strict low-latency playback scenarios you might want to set this to a
    /// lower value, likely together with the `adjust_latency` field on
    /// [`StreamFlags`]. If you do, you can be sure that the latency doesn't
    /// grow beyond what is acceptable for the use case, at the cost of getting
    /// more underruns if the latency is lower than what the server can reliably
    /// handle.
    pub max_length: u32,

    ///  The target length of the buffer. The server tries to assure that at
    /// least `tlength` bytes are always available in the per-stream server-side
    /// playback buffer. The server will only send requests for more data as
    /// long as the buffer has less than this number of bytes of data.
    ///
    /// It is recommended to set this to `u32::MAX`, which will initialize this
    /// to a value that is deemed sensible by the server. However, this value
    /// will default to something like 2s; for applications that have specific
    /// latency requirements this value should be set to the maximum latency
    /// that the application can deal with.
    ///
    /// Unless `adjust_latency` is set on the matching [`StreamFlags`] is set,
    /// this value will influence only the per-stream playback buffer size. When
    /// `adjust_latency` is set on the matching [`StreamFlags`], the overall
    /// latency of the sink plus the playback buffer size is configured to this
    /// value. Set `adjust_latency` on the matching [`StreamFlags`] if you are
    /// interested in adjusting the overall latency. Don't set it if you are
    /// interested in configuring the server-side per-stream playback buffer
    /// size.
    ///
    /// Only valid for playback.
    pub target_length: u32,

    /// Configure pre-buffering. The server does not start with playback before
    /// at least prebuf bytes are available in the buffer. It is recommended to
    /// set this to `u32::MAX`, which will initialize this to the same value as
    /// tlength, whatever that may be.
    ///
    /// Initialize to 0 to enable manual start/stop control of the stream. This
    /// means that playback will not stop on underrun and playback will not
    /// start automatically, instead pa_stream_cork() needs to be called
    /// explicitly. If you set this value to 0 you should also set
    /// `start_corked` on [`StreamFlags`]. Should underrun occur, the read index
    /// of the output buffer overtakes the write index, and hence the fill level
    /// of the buffer is negative.
    ///
    /// Only valid for playback.
    pub pre_buffering: u32,

    /// Configure the minimum request. The server does not request less than
    /// minreq bytes from the client, instead waits until the buffer is free
    /// enough to request more bytes at once. It is recommended to set this to
    /// `u32::MAX`, which will initialize this to a value that is deemed
    /// sensible by the server. This should be set to a value that gives
    /// PulseAudio enough time to move the data from the per-stream playback
    /// buffer into the hardware playback buffer.
    ///
    /// Only valid for playback.
    pub minimum_request_length: u32,

    /// Configure the fragment size. The server sends data in blocks of fragsize
    /// bytes size. Large values diminish interactivity with other operations on
    /// the connection context but decrease control overhead. It is recommended
    /// to set this to `u32::MAX`, which will initialize this to a value that is
    /// deemed sensible by the server. However, this value will default to
    /// something like 2s; For applications that have specific latency
    /// requirements this value should be set to the maximum latency that the
    /// application can deal with.
    ///
    /// If `adjust_latency` is set on the matching [`StreamFlags`], the overall
    /// source latency will be adjusted according to this value. If it is not
    /// set the source latency is left unmodified.
    ///
    /// Only valid for recording.
    pub fragment_size: u32,
}

impl Default for BufferAttr {
    fn default() -> Self {
        Self {
            max_length: u32::MAX,
            target_length: u32::MAX,
            pre_buffering: u32::MAX,
            minimum_request_length: u32::MAX,
            fragment_size: u32::MAX,
        }
    }
}

/// Parameters for a cork/uncork command.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub struct CorkStreamParams {
    /// The channel to cork or uncork.
    pub channel: u32,

    /// Whether to cork or uncork the stream.
    pub cork: bool,
}

impl TagStructRead for CorkStreamParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            channel: ts
                .read_index()?
                .ok_or_else(|| ProtocolError::Invalid("invalid channel index".to_string()))?,
            cork: ts.read_bool()?,
        })
    }
}

impl TagStructWrite for CorkStreamParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(Some(self.channel))?;
        ts.write_bool(self.cork)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cork_params_serde() -> anyhow::Result<()> {
        let params = CorkStreamParams {
            channel: 0,
            cork: true,
        };

        test_util::test_serde(&params)
    }
}
