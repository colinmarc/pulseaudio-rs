//! A stream connects a source and a sink.
//!
//! A stream connected to a source is one of the sources "source outputs", a stream connected to a
//! sink is one of the sinks "sink inputs".

use bitflags::bitflags;

/// The direction of a stream.
#[derive(Debug)]
pub enum StreamDirection {
    /// Playback stream.
    Playback,
    /// Record stream.
    Record,
    /// Sample upload stream.
    Upload,
}

bitflags! {
    #[derive(Debug, Clone, Copy, Default)]
    pub struct StreamFlags: u32 {
        /// Create the stream corked, requiring an explicit `pa_stream_cork()` call to uncork it.
        const START_CORKED = 0x0001;
        /// Interpolate the latency for this stream. When enabled,
        /// `pa_stream_get_latency()` and `pa_stream_get_time()` will try to
        /// estimate the current record/playback time based on the local
        /// time that passed since the last timing info update. Using this
        /// option has the advantage of not requiring a whole roundtrip
        /// when the current playback/recording time is needed. Consider
        /// using this option when requesting latency information
        /// frequently. This is especially useful on long latency network
        /// connections. It makes a lot of sense to combine this option
        /// with `AUTO_TIMING_UPDATE`.
        const INTERPOLATE_TIMING = 0x0002;
        /// Don't force the time to increase monotonically. If this
        /// option is enabled, `pa_stream_get_time()` will not necessarily
        /// return always monotonically increasing time values on each
        /// call. This may confuse applications which cannot deal with time
        /// going 'backwards', but has the advantage that bad transport
        /// latency estimations that caused the time to jump ahead can
        /// be corrected quickly, without the need to wait. (Please note
        /// that this flag was named `NOT_MONOTONOUS` in releases
        /// prior to 0.9.11)
        const NOT_MONOTONIC = 0x0004;
        /// If set timing update requests are issued periodically
        /// automatically. Combined with `INTERPOLATE_TIMING` you
        /// will be able to query the current time and latency with
        /// `pa_stream_get_time()` and `pa_stream_get_latency()` at all times
        /// without a packet round trip.
        const AUTO_TIMING_UPDATE = 0x0008;
        /// Don't remap channels by their name, instead map them simply
        /// by their index. Implies `NO_REMIX_CHANNELS`. Only
        /// supported when the server is at least PA 0.9.8. It is ignored
        /// on older servers. \since 0.9.8
        const NO_REMAP_CHANNELS = 0x0010;
        /// When remapping channels by name, don't upmix or downmix them
        /// to related channels. Copy them into matching channels of the
        /// device 1:1. Only supported when the server is at least PA
        /// 0.9.8. It is ignored on older servers. \since 0.9.8
        const NO_REMIX_CHANNELS = 0x0020;
        /// Use the sample format of the sink/device this stream is being
        /// connected to, and possibly ignore the format the sample spec
        /// contains -- but you still have to pass a valid value in it as a
        /// hint to PulseAudio what would suit your stream best. If this is
        /// used you should query the used sample format after creating the
        /// stream by using pa_stream_get_sample_spec(). Also, if you
        /// specified manual buffer metrics it is recommended to update
        /// them with pa_stream_set_buffer_attr() to compensate for the
        /// changed frame sizes. Only supported when the server is at least
        /// PA 0.9.8. It is ignored on older servers.
        ///
        /// When creating streams with pa_stream_new_extended(), this flag has no
        /// effect. If you specify a format with PCM encoding, and you want the
        /// server to choose the sample format, then you should leave the sample
        /// format unspecified in the pa_format_info object. This also means that
        /// you can't use pa_format_info_from_sample_spec(), because that function
        /// always sets the sample format.
        ///
        /// \since 0.9.8
        const FIX_FORMAT = 0x0040;
        /// Use the sample rate of the sink, and possibly ignore the rate
        /// the sample spec contains. Usage similar to
        /// PA_STREAM_FIX_FORMAT. Only supported when the server is at least
        /// PA 0.9.8. It is ignored on older servers.
        ///
        /// When creating streams with pa_stream_new_extended(), this flag has no
        /// effect. If you specify a format with PCM encoding, and you want the
        /// server to choose the sample rate, then you should leave the rate
        /// unspecified in the pa_format_info object. This also means that you can't
        /// use pa_format_info_from_sample_spec(), because that function always sets
        /// the sample rate.
        ///
        /// \since 0.9.8
        const FIX_RATE = 0x0080;
        /// Use the number of channels and the channel map of the sink,
        /// and possibly ignore the number of channels and the map the
        /// sample spec and the passed channel map contain. Usage similar
        /// to PA_STREAM_FIX_FORMAT. Only supported when the server is at
        /// least PA 0.9.8. It is ignored on older servers.
        ///
        /// When creating streams with pa_stream_new_extended(), this flag has no
        /// effect. If you specify a format with PCM encoding, and you want the
        /// server to choose the channel count and/or channel map, then you should
        /// leave the channels and/or the channel map unspecified in the
        /// pa_format_info object. This also means that you can't use
        /// pa_format_info_from_sample_spec(), because that function always sets
        /// the channel count (but if you only want to leave the channel map
        /// unspecified, then pa_format_info_from_sample_spec() works, because it
        /// accepts a NULL channel map).
        ///
        /// \since 0.9.8
        const FIX_CHANNELS = 0x0100;
        /// Don't allow moving of this stream to another
        /// sink/device. Useful if you use any of the PA_STREAM_FIX_ flags
        /// and want to make sure that resampling never takes place --
        /// which might happen if the stream is moved to another
        /// sink/source with a different sample spec/channel map. Only
        /// supported when the server is at least PA 0.9.8. It is ignored
        /// on older servers. \since 0.9.8
        const DONT_MOVE = 0x0200;
        /// Allow dynamic changing of the sampling rate during playback
        /// with pa_stream_update_sample_rate(). Only supported when the
        /// server is at least PA 0.9.8. It is ignored on older
        /// servers. \since 0.9.8
        const VARIABLE_RATE = 0x0400;
        /// Find peaks instead of resampling. \since 0.9.11
        const PEAK_DETECT = 0x0800;
        /// Create in muted state. If neither `START_UNMUTED` nor
        /// `START_MUTED` are set, it is left to the server to decide
        /// whether to create the stream in muted or in unmuted
        /// state. \since 0.9.11
        const START_MUTED = 0x1000;
        /// Try to adjust the latency of the sink/source based on the
        /// requested buffer metrics and adjust buffer metrics
        /// accordingly. Also see pa_buffer_attr. This option may not be
        /// specified at the same time as PA_STREAM_EARLY_REQUESTS. \since
        /// 0.9.11
        const ADJUST_LATENCY = 0x2000;
        /// Enable compatibility mode for legacy clients that rely on a
        /// "classic" hardware device fragment-style playback model. If
        /// this option is set, the minreq value of the buffer metrics gets
        /// a new meaning: instead of just specifying that no requests
        /// asking for less new data than this value will be made to the
        /// client it will also guarantee that requests are generated as
        /// early as this limit is reached. This flag should only be set in
        /// very few situations where compatibility with a fragment-based
        /// playback model needs to be kept and the client applications
        /// cannot deal with data requests that are delayed to the latest
        /// moment possible. (Usually these are programs that use usleep()
        /// or a similar call in their playback loops instead of sleeping
        /// on the device itself.) Also see pa_buffer_attr. This option may
        /// not be specified at the same time as
        /// PA_STREAM_ADJUST_LATENCY. \since 0.9.12
        const EARLY_REQUESTS = 0x4000;
        /// If set this stream won't be taken into account when it is
        /// checked whether the device this stream is connected to should
        /// auto-suspend. \since 0.9.15
        const DONT_INHIBIT_AUTO_SUSPEND = 0x8000;
        /// Create in unmuted state. If neither PA_STREAM_START_UNMUTED
        /// nor PA_STREAM_START_MUTED are set it is left to the server to decide
        /// whether to create the stream in muted or in unmuted
        /// state. \since 0.9.15
        const START_UNMUTED = 0x10000;
        /// If the sink/source this stream is connected to is suspended
        /// during the creation of this stream, cause it to fail. If the
        /// sink/source is being suspended during creation of this stream,
        /// make sure this stream is terminated. \since 0.9.15
        const FAIL_ON_SUSPEND = 0x20000;
        /// If a volume is passed when this stream is created, consider
        /// it relative to the sink's current volume, never as absolute
        /// device volume. If this is not specified the volume will be
        /// consider absolute when the sink is in flat volume mode,
        /// relative otherwise. \since 0.9.20
        const RELATIVE_VOLUME = 0x40000;
        /// Used to tag content that will be rendered by passthrough sinks.
        /// The data will be left as is and not reformatted, resampled.
        /// \since 1.0
        const PASSTHROUGH = 0x80000;
    }
}

/// Playback and record buffer metrics.
#[derive(Default, Debug)]
pub struct BufferAttr {
    /// Maximum length of the buffer in bytes. Setting this to `u32::MAX`
    /// will initialize this to the maximum value supported by server,
    /// which is recommended.
    ///
    /// In strict low-latency playback scenarios you might want to set this to
    /// a lower value, likely together with the PA_STREAM_ADJUST_LATENCY flag.
    /// If you do so, you ensure that the latency doesn't grow beyond what is
    /// acceptable for the use case, at the cost of getting more underruns if
    /// the latency is lower than what the server can reliably handle.
    pub maxlength: u32,
    /// Playback only: target length of the buffer. The server tries
    /// to assure that at least `tlength` bytes are always available in
    /// the per-stream server-side playback buffer. The server will
    /// only send requests for more data as long as the buffer has
    /// less than this number of bytes of data.
    ///
    /// It is recommended to set this to `u32::MAX`, which will
    /// initialize this to a value that is deemed sensible by the
    /// server. However, this value will default to something like 2s;
    /// for applications that have specific latency requirements
    /// this value should be set to the maximum latency that the
    /// application can deal with.
    ///
    /// When PA_STREAM_ADJUST_LATENCY is not set this value will
    /// influence only the per-stream playback buffer size. When
    /// PA_STREAM_ADJUST_LATENCY is set the overall latency of the sink
    /// plus the playback buffer size is configured to this value. Set
    /// PA_STREAM_ADJUST_LATENCY if you are interested in adjusting the
    /// overall latency. Don't set it if you are interested in
    /// configuring the server-side per-stream playback buffer
    /// size.
    pub tlength: u32,
    /// Playback only: pre-buffering. The server does not start with
    /// playback before at least prebuf bytes are available in the
    /// buffer. It is recommended to set this to (uint32_t) -1, which
    /// will initialize this to the same value as tlength, whatever
    /// that may be.
    ///
    /// Initialize to 0 to enable manual start/stop control of the stream.
    /// This means that playback will not stop on underrun and playback
    /// will not start automatically, instead pa_stream_cork() needs to
    /// be called explicitly. If you set this value to 0 you should also
    /// set PA_STREAM_START_CORKED. Should underrun occur, the read index
    /// of the output buffer overtakes the write index, and hence the
    /// fill level of the buffer is negative.
    ///
    /// Start of playback can be forced using pa_stream_trigger() even
    /// though the prebuffer size hasn't been reached. If a buffer
    /// underrun occurs, this prebuffering will be again enabled.
    pub prebuf: u32,
    /// Playback only: minimum request. The server does not request
    /// less than minreq bytes from the client, instead waits until the
    /// buffer is free enough to request more bytes at once. It is
    /// recommended to set this to (uint32_t) -1, which will initialize
    /// this to a value that is deemed sensible by the server. This
    /// should be set to a value that gives PulseAudio enough time to
    /// move the data from the per-stream playback buffer into the
    /// hardware playback buffer.
    pub minreq: u32,
    /// Recording only: fragment size. The server sends data in
    /// blocks of fragsize bytes size. Large values diminish
    /// interactivity with other operations on the connection context
    /// but decrease control overhead. It is recommended to set this to
    /// (uint32_t) -1, which will initialize this to a value that is
    /// deemed sensible by the server. However, this value will default
    /// to something like 2s; For applications that have specific
    /// latency requirements this value should be set to the maximum
    /// latency that the application can deal with.
    ///
    /// If PA_STREAM_ADJUST_LATENCY is set the overall source latency
    /// will be adjusted according to this value. If it is not set the
    /// source latency is left unmodified.
    pub fragsize: u32,
}

// /// A playback stream connecting a source with a sink.
// ///
// /// A stream always has a fixed sample format that's not necessarily equivalent with the current
// /// format of the sink it's connected to.
// #[derive(Debug)]
// pub struct Stream {
//     /// Latency in microseconds.
//     latency: u64,
//     /// The sink this stream is outputting data to.
//     sink: Rc<Sink>,
// }

// impl Stream {
//     pub fn latency(&self) -> time::Duration {
//         time::Duration::from_micros(self.latency)
//     }
// }
