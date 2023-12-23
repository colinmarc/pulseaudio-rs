//! Defines sink data and utilities.

use std::ffi::CString;

use bitflags::bitflags;
use enum_primitive_derive::Primitive;

use crate::protocol::*;

bitflags! {
    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
    pub struct SinkFlags: u32 {
        /// Supports hardware volume control. This is a dynamic flag and may
        /// change at runtime after the sink has initialized.
        const HW_VOLUME_CTRL = 0x0001;

        /// Supports latency querying.
        const LATENCY = 0x0002;

        /// Is a hardware sink of some kind, in contrast to
        /// "virtual"/software sinks. \since 0.9.3
        const HARDWARE = 0x0004;

        /// Is a networked sink of some kind. \since 0.9.7
        const NETWORK = 0x0008;

        /// Supports hardware mute control. This is a dynamic flag and may
        /// change at runtime after the sink has initialized. \since 0.9.11
        const HW_MUTE_CTRL = 0x0010;

        /// Volume can be translated to dB with pa_sw_volume_to_dB(). This is a
        /// dynamic flag and may change at runtime after the sink has initialized.
        /// \since 0.9.11
        const DECIBEL_VOLUME = 0x0020;

        /// This sink is in flat volume mode, i.e.\ always the maximum of
        /// the volume of all connected inputs. \since 0.9.15
        const FLAT_VOLUME = 0x0040;

        /// The latency can be adjusted dynamically depending on the
        /// needs of the connected streams. \since 0.9.15
        const DYNAMIC_LATENCY = 0x0080;

        /// The sink allows setting what formats are supported by the connected
        /// hardware. The actual functionality to do this might be provided by an
        /// extension. \since 1.0
        const SET_FORMATS = 0x0100;
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Primitive, Default)]
pub enum SinkState {
    /// Sink is playing samples: The sink is used by at least one non-paused input.
    Running = 0,
    /// Sink is playing but has no connected inputs that send samples.
    Idle = 1,
    /// Sink is not currently playing and can be closed.
    // FIXME: Is this what pasuspender uses?
    #[default]
    Suspended = 2,
}

/// Specifies the direction of a port.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    /// The port is an input, ie. part of a source.
    Input,
    /// The port is an output, ie. part of a sink.
    Output,
}

/// Port availability / jack detection status.
/// \since 2.0
// TODO: Clarify if this means "port available for playback/recording"
#[derive(Debug, Copy, Clone, Primitive, PartialEq, Eq)]
pub enum PortAvailable {
    /// This port does not support jack detection.
    Unknown = 0,
    /// This port is not available, likely because the jack is not plugged in. \since 2.0
    No = 1,
    /// This port is available, likely because the jack is plugged in. \since 2.0
    Yes = 2,
}

/// A port on a sink, to which a speaker or microphone can be connected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortInfo {
    pub name: CString,
    pub description: Option<CString>,
    pub dir: Direction,
    pub priority: u32,
    pub available: PortAvailable,
}

/// A sink connected to a PulseAudio server.
///
/// Every sink can have any number of Sink Inputs, or streams connected to it. If more than one
/// input is connected, the inputs will be mixed together.
///
/// A sink always has a single configured sample spec, and all sink inputs are converted to that
/// format (using resampling to match the sample rates, if necessary).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SinkInfo {
    /// Server-internal sink ID.
    pub index: u32,

    /// The human readable name of the sink.
    pub name: CString,

    /// A description of the sink.
    pub description: Option<CString>,

    /// A list of properties.
    pub props: Props,

    /// The current state of the sink.
    pub state: SinkState,

    /// The format of samples that the sink expects.
    pub sample_spec: SampleSpec,

    /// The mapping of channels to positions that the sink expects.
    pub channel_map: ChannelMap, // make sure channel map length == sample spec channels

    /// Index of the owning module of the sink.
    pub owner_module_index: Option<u32>,

    /// The volume of the sink.
    pub cvolume: ChannelVolume,

    /// Overrides `cvolume` if set.
    pub muted: bool,

    /// ID of the monitor source for the sink.
    pub monitor_source_index: Option<u32>,

    /// Name of the monitor source for the sink.
    pub monitor_source_name: Option<CString>,

    /// Flags the sink is configured with.
    pub flags: SinkFlags,

    /// In microseconds, the length of queued audio in the output.
    pub actual_latency: u64,

    /// In microseconds, the configured latency of the sink.
    pub requested_latency: u64,

    /// The name of the driver used for this sink.
    pub driver: Option<CString>,

    /// The base volume of the sink.
    pub base_volume: Volume,

    /// The number of individual steps in volume for sinks which do not support arbitrary volumes.
    pub volume_steps: u32,

    /// The index of the card this sink belongs to.
    pub card_index: Option<u32>,

    /// A sink has at least one port a plug can be plugged into, and only *one* port can be active
    /// at any given time.
    pub ports: Vec<PortInfo>,

    /// The index of the currently active port.
    pub active_port: usize,

    /// The list of supported sample formats.
    ///
    /// Most commonly used sinks of consumer hardware will only have support for a single format,
    /// PCM.
    pub formats: Vec<FormatInfo>,
}

impl SinkInfo {
    // /// Creates a dummy sink that will simply drop all samples sent to it.
    // ///
    // /// The server will create a dummy sink on startup if no other sinks can be found.
    // pub fn new_dummy(index: u32) -> Self {
    //     Self {
    //         index,
    //         name: CString::new("Dummy Sink").unwrap(),
    //         props: PropList::new(),
    //         state: SinkState::Idle,
    //         sample_spec: SampleSpec::new(SampleFormat::Float32Le, 2, 48000).unwrap(),
    //         channel_map: {
    //             let mut map = ChannelMap::new();
    //             map.push(ChannelPosition::FrontLeft).unwrap();
    //             map.push(ChannelPosition::FrontRight).unwrap();
    //             map
    //         },
    //         cvolume: {
    //             let mut vol = CVolume::new();
    //             vol.push(Volume::from_linear(1.0)).unwrap();
    //             vol.push(Volume::from_linear(1.0)).unwrap();
    //             vol
    //         },
    //         muted: false,
    //         flags: SinkFlags::empty(),
    //         ports: vec![Port::new_output(
    //             CString::new("Stereo Output").unwrap(),
    //             CString::new("").unwrap(),
    //             0,
    //         )],
    //         active_port: 0,
    //         formats: vec![FormatInfo::new(FormatEncoding::Pcm)],
    //     }
    // }
}
