//! The `GET_*_INFO` and `GET_*_INFO_LIST` commands.

use std::ffi::CString;

use super::CommandReply;
use crate::protocol::{serde::*, *};

use bitflags::bitflags;
use enum_primitive_derive::Primitive;

bitflags! {
    /// Sink configuration flags.
    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
    pub struct SinkFlags: u32 {
        /// The sink supports hardware volume control.
        const HW_VOLUME_CTRL = 0x0001;

        /// The sink supports latency querying.
        const LATENCY = 0x0002;

        /// The sink is a hardware sink, in contrast to a "virtual" or software sink.
        const HARDWARE = 0x0004;

        /// The sink is a networked sink.
        const NETWORK = 0x0008;

        /// The sink supports hardware mute control.
        const HW_MUTE_CTRL = 0x0010;

        /// The volume can be translated to decibels.
        const DECIBEL_VOLUME = 0x0020;

        /// The sink is in "flat volume" mode, i.e. always the maximum of the
        /// volume of all connected inputs.
        const FLAT_VOLUME = 0x0040;

        /// The latency of the sink can be adjusted dynamically.
        const DYNAMIC_LATENCY = 0x0080;

        /// The sink allows allows the supported formats to be set.
        const SET_FORMATS = 0x0100;
    }
}

/// The state of a sink.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Primitive)]
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

/// Port availability status.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Primitive)]
pub enum PortAvailable {
    /// This port does not support jack detection.
    #[default]
    Unknown = 0,
    /// This port is not available, likely because the jack is not plugged in. \since 2.0
    No = 1,
    /// This port is available, likely because the jack is plugged in. \since 2.0
    Yes = 2,
}

/// A port on a sink or source.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PortInfo {
    /// The name of the port.
    pub name: CString,

    /// A description of the port.
    pub description: Option<CString>,

    /// The direction of the port.
    pub dir: Direction,

    /// The priority of the port.
    pub priority: u32,

    /// Whether the port is available.
    pub available: PortAvailable,
}

/// A sink connected to a PulseAudio server.
///
/// Every sink can have any number of Sink Inputs, or streams connected to it. If more than one
/// input is connected, the inputs will be mixed together.
///
/// A sink always has a single configured sample spec, and all sink inputs are converted to that
/// format (using resampling to match the sample rates, if necessary).
#[derive(Default, Debug, Clone, Eq, PartialEq)]
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

    /// The ID of the module that owns this sink.
    pub owner_module_index: Option<u32>,

    /// The volume of the sink.
    pub cvolume: ChannelVolume,

    /// Whether the sink is muted.
    pub muted: bool,

    /// The ID of the monitor source for the sink.
    pub monitor_source_index: Option<u32>,

    /// Name of the monitor source for the sink.
    pub monitor_source_name: Option<CString>,

    /// Flags the sink is configured with.
    pub flags: SinkFlags,

    /// The length of queued audio in the output, in microseconds.
    pub actual_latency: u64,

    /// The configured latency of the sink, in microseconds.
    pub configured_latency: u64,

    /// The name of the driver used for this sink.
    pub driver: Option<CString>,

    /// The base volume of the sink.
    pub base_volume: Volume,

    /// The number of individual steps in volume, for sinks which do not support arbitrary volumes.
    pub volume_steps: Option<u32>,

    /// The index of the card this sink belongs to.
    pub card_index: Option<u32>,

    /// A sink has at least one port a plug can be plugged into, and only *one* port can be active
    /// at any given time.
    pub ports: Vec<PortInfo>,

    /// The index of the currently active port.
    pub active_port: usize,

    /// The list of supported sample formats.
    pub formats: Vec<FormatInfo>,
}

impl SinkInfo {
    /// Creates a "dummy" sink, which the PulseAudio server returns when there
    /// are no sinks.
    pub fn new_dummy(index: u32) -> Self {
        Self {
            index,
            name: CString::new("Dummy Sink").unwrap(),
            props: Props::new(),
            state: SinkState::Idle,
            sample_spec: SampleSpec {
                format: SampleFormat::S16Le,
                channels: 2,
                sample_rate: 48000,
            },
            channel_map: ChannelMap::stereo(),
            cvolume: ChannelVolume::norm(2),
            muted: false,
            flags: SinkFlags::empty(),
            ports: vec![PortInfo {
                name: CString::new("Stereo Output").unwrap(),
                description: None,
                dir: Direction::Output,
                priority: 0,
                available: PortAvailable::Yes,
            }],
            active_port: 0,
            formats: vec![FormatInfo::new(FormatEncoding::Pcm)],
            ..Default::default()
        }
    }
}

/// The parameters for [`Command::GetSinkInfo`]. Either the sink index or the
/// sink name should be specified.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct GetSinkInfo {
    /// The index of the sink to query.
    pub index: Option<u32>,

    /// The name of the sink to query.
    pub name: Option<CString>,
}

impl TagStructRead for GetSinkInfo {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts.read_index()?,
            name: ts.read_string()?,
        })
    }
}

impl TagStructWrite for GetSinkInfo {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_index(self.index)?;
        w.write_string(self.name.as_ref())?;
        Ok(())
    }
}

impl CommandReply for SinkInfo {}

impl TagStructRead for SinkInfo {
    fn read(
        ts: &mut TagStructReader<'_>,
        protocol_version: u16,
    ) -> Result<SinkInfo, ProtocolError> {
        let mut sink = SinkInfo {
            index: ts
                .read_index()?
                .ok_or_else(|| ProtocolError::Invalid("invalid sink index".into()))?,
            name: ts.read_string_non_null()?,
            description: ts.read_string()?,
            sample_spec: ts.read()?,
            channel_map: ts.read()?,
            owner_module_index: ts.read_index()?,
            cvolume: ts.read()?,
            muted: ts.read_bool()?,
            monitor_source_index: ts.read_index()?,
            monitor_source_name: ts.read_string()?,
            actual_latency: ts.read_usec()?,
            driver: ts.read_string()?,
            flags: SinkFlags::from_bits_truncate(ts.read_u32()?),
            props: ts.read()?,
            configured_latency: ts.read_usec()?,
            ..Default::default()
        };

        if protocol_version >= 15 {
            sink.base_volume = ts.read()?;

            sink.state = ts.read_enum()?;
            sink.volume_steps = match ts.read_u32()? {
                0 => None,
                n => Some(n),
            };
            sink.card_index = ts.read_index()?;
        }

        if protocol_version >= 16 {
            for _ in 0..ts.read_u32()? {
                let name = ts
                    .read_string()?
                    .ok_or(ProtocolError::Invalid("empty port name".into()));
                let description = ts.read_string()?;
                let priority = ts.read_u32()?;

                let available = if protocol_version >= 24 {
                    ts.read_enum()?
                } else {
                    PortAvailable::Unknown
                };

                sink.ports.push(PortInfo {
                    name: name.unwrap_or_default().to_owned(),
                    description,
                    dir: Direction::Input,
                    priority,
                    available,
                });
            }

            let active_port_name = ts.read_string()?;
            if let Some(port) = active_port_name {
                sink.active_port = sink
                    .ports
                    .iter()
                    .position(|p| port.to_bytes() == p.name.as_bytes())
                    .unwrap_or(0);
            }
        }

        if protocol_version >= 21 {
            for _ in 0..ts.read_u8()? {
                sink.formats.push(ts.read()?);
            }
        }

        Ok(sink)
    }
}

impl TagStructWrite for SinkInfo {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.index)?;
        w.write_string(Some(&self.name))?;
        w.write_string(self.description.as_ref())?;
        w.write(self.sample_spec.protocol_downgrade(protocol_version))?;
        w.write(self.channel_map)?;
        w.write_index(self.owner_module_index)?;
        w.write(self.cvolume)?;
        w.write_bool(self.muted)?;
        w.write_index(self.monitor_source_index)?; // sink's monitor source
        w.write_string(self.monitor_source_name.as_ref())?;
        w.write_usec(self.actual_latency)?;
        w.write_string(self.driver.as_ref())?; // TODO: driver name
        w.write_u32(self.flags.bits())?;
        // proto>=13
        w.write(&self.props)?;
        w.write_usec(self.configured_latency)?;
        if protocol_version >= 15 {
            w.write(self.base_volume)?;
            w.write_u32(self.state as u32)?;
            w.write_u32(self.volume_steps.unwrap_or_default())?;
            w.write_index(self.card_index)?;
        }
        if protocol_version >= 16 {
            w.write_u32(self.ports.len() as u32)?;
            for port in &self.ports {
                w.write_string(Some(&port.name))?;
                w.write_string(port.description.as_ref())?;
                w.write_u32(port.priority)?;
                if protocol_version >= 24 {
                    w.write_u32(port.available as u32)?;
                }
            }

            let active_port_name = if self.active_port < self.ports.len() {
                Some(&self.ports[self.active_port].name)
            } else {
                None
            };

            w.write_string(active_port_name)?;
        }
        if protocol_version >= 21 {
            // send supported sample formats
            w.write_u8(self.formats.len() as u8)?;
            for format in &self.formats {
                w.write(format)?;
            }
        }

        Ok(())
    }
}

/// The server reply to [`super::Command::GetSinkInfoList`].
pub type SinkInfoList = Vec<SinkInfo>;

impl CommandReply for SinkInfoList {}

impl TagStructRead for SinkInfoList {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut sinks = Vec::new();
        while ts.has_data_left()? {
            sinks.push(ts.read()?);
        }

        Ok(sinks)
    }
}

impl TagStructWrite for SinkInfoList {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        for sink in self {
            w.write(sink)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;
    use crate::protocol::serde::test_util::test_serde;

    #[test]
    fn sink_info_list_serde() {
        let mut props1 = Props::new();
        props1.set(Prop::DeviceString, CString::new("foo").unwrap());

        let mut props2 = Props::new();
        props2.set(Prop::ApplicationName, CString::new("bar").unwrap());

        let sinks = vec![
            SinkInfo {
                index: 0,
                name: CString::new("sink0").unwrap(),
                props: props1,
                sample_spec: SampleSpec {
                    format: SampleFormat::S16Le,
                    channels: 2,
                    sample_rate: 44100,
                },
                ..Default::default()
            },
            SinkInfo {
                index: 1,
                name: CString::new("sink1").unwrap(),
                props: props2,
                sample_spec: SampleSpec {
                    format: SampleFormat::S16Le,
                    channels: 2,
                    sample_rate: 44100,
                },
                ..Default::default()
            },
        ];

        test_serde(&sinks).expect("SinkInfoList roundtrip")
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use super::*;
    use crate::integration_test_util::*;

    #[test]
    fn list_sinks() -> Result<(), Box<dyn std::error::Error>> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            Command::GetSinkInfoList,
            protocol_version,
        )?;

        let (_, info_list) = read_reply_message::<SinkInfoList>(&mut sock, protocol_version)?;
        assert!(!info_list.is_empty());

        Ok(())
    }
}
