use std::ffi::CString;

use enum_primitive_derive::Primitive;

use crate::protocol::{serde::*, *};

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Primitive)]
pub enum SourceState {
    /// The source is recording and used by at least one non-corked source-output.
    Running = 0,
    /// the source is still recording, but there is no non-corked source-output.
    Idle = 1,
    /// The source is suspended and not recording.
    #[default]
    Suspended = 2,
}

bitflags! {
    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
    pub struct SourceFlags: u32 {
        /// The source supports hardware volume control.
        const HW_VOLUME_CTRL = 0x0001;

        /// The source supports latency querying.
        const LATENCY = 0x0002;

        /// This is a hardware source, in contrast to a "virtual" or software source.
        const HARDWARE = 0x0004;

        /// This is a networked source.
        const NETWORK = 0x0008;

        /// The source supports hardware mute control.
        const HW_MUTE_CTRL = 0x0010;

        /// The volume can be translated to decibels.
        const DECIBEL_VOLUME = 0x0020;

        /// The latency of the source can be adjusted dynamically.
        const DYNAMIC_LATENCY = 0x0040;

        /// The source is in "flat volume" mode, i.e. always the maximum of the
        /// volume of all connected outputs.
        const FLAT_VOLUME = 0x0080;
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SourceInfo {
    /// The server-internal source ID.
    pub index: u32,

    /// The name of the source.
    pub name: CString,

    /// A description of the source.
    pub description: Option<CString>,

    /// Properties of the source.
    pub props: Props,

    /// The state of the source.
    pub state: SourceState,

    /// The format of the samples that the source produces.
    pub sample_spec: SampleSpec,

    /// The mapping of channels to positions that the source will produce.
    pub channel_map: ChannelMap,

    /// The ID of the module that owns this source.
    pub owner_module_index: Option<u32>,

    /// The volume of the source.
    pub cvolume: ChannelVolume,

    /// The "base volume" of the source.
    pub base_volume: Volume,

    /// The number of individual steps in volume, for sources which do not support arbitrary volumes.
    pub volume_steps: Option<u32>,

    /// Whether the source is muted.
    pub muted: bool,

    /// If this is a monitor source, this refers to the index of the matching sink.
    pub monitor_of_sink_index: Option<u32>,

    /// If this is a monitor source, this is the name of the matching sink.
    pub monitor_of_sink_name: Option<CString>,

    /// Flags the source is configured with.
    pub flags: SourceFlags,

    /// The length of queued audio in the input, in microseconds.
    pub actual_latency: u64,

    /// The latency the source has been configured with, in microseconds.
    pub configured_latency: u64,

    /// The name of the driver this source belongs to.
    pub driver: Option<CString>,

    /// The index of the card this source belongs to.
    pub card_index: Option<u32>,

    /// A source has at least one port a plug can be plugged into, and only *one* port can be active
    /// at any given time.
    pub ports: Vec<PortInfo>,

    /// The index of the currently active port.
    pub active_port: usize,

    /// The list of supported sample formats.
    pub formats: Vec<FormatInfo>,
}

impl TagStructRead for SourceInfo {
    fn read(ts: &mut TagStructReader, protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut source = SourceInfo {
            index: ts
                .read_index()?
                .ok_or_else(|| ProtocolError::Invalid("invalid source index".into()))?,
            name: ts
                .read_string()?
                .ok_or_else(|| ProtocolError::Invalid("null source name".into()))?,
            description: ts.read_string()?,
            sample_spec: ts.read()?,
            channel_map: ts.read()?,
            owner_module_index: ts.read_index()?,
            cvolume: ts.read()?,
            muted: ts.read_bool()?,
            monitor_of_sink_index: ts.read_index()?,
            monitor_of_sink_name: ts.read_string()?,
            actual_latency: ts.read_usec()?,
            driver: ts.read_string()?,
            flags: SourceFlags::from_bits_truncate(ts.read_u32()?),
            ..Default::default()
        };

        if protocol_version >= 13 {
            source.props = ts.read()?;
            source.configured_latency = ts.read_usec()?;
        }

        if protocol_version >= 15 {
            source.base_volume = ts.read()?;
            source.state = ts.read_enum()?;
            let steps = ts.read_u32()?;
            source.volume_steps = if steps == 0 { None } else { Some(steps) };
            source.card_index = ts.read_index()?;
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

                source.ports.push(PortInfo {
                    name: name.unwrap_or_default().to_owned(),
                    description,
                    dir: Direction::Output,
                    priority,
                    available,
                });
            }

            let active_port_name = ts.read_string()?;
            if let Some(port) = active_port_name {
                source.active_port = source
                    .ports
                    .iter()
                    .position(|p| port.to_bytes() == p.name.as_bytes())
                    .unwrap_or(0);
            }
        }

        if protocol_version >= 21 {
            for _ in 0..ts.read_u8()? {
                source.formats.push(ts.read()?);
            }
        }

        Ok(source)
    }
}

impl TagStructWrite for SourceInfo {
    fn write(&self, w: &mut TagStructWriter, protocol_version: u16) -> Result<(), ProtocolError> {
        w.write_index(Some(self.index))?;
        w.write_string(Some(&self.name))?;
        w.write_string(self.description.as_ref())?;
        w.write(self.sample_spec)?;
        w.write(&self.channel_map)?;
        w.write_index(self.owner_module_index)?;
        w.write(&self.cvolume)?;
        w.write_bool(self.muted)?;
        w.write_index(self.monitor_of_sink_index)?;
        w.write_string(self.monitor_of_sink_name.as_ref())?;
        w.write_usec(self.actual_latency)?;
        w.write_string(self.driver.as_ref())?;
        w.write_u32(self.flags.bits())?;

        if protocol_version >= 13 {
            w.write(&self.props)?;
            w.write_usec(self.configured_latency)?;
        }

        if protocol_version >= 15 {
            w.write(self.base_volume)?;
            w.write_u32(self.state as u32)?;
            w.write_u32(self.volume_steps.unwrap_or(0))?;
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
            w.write_u8(self.formats.len() as u8)?;
            for format in &self.formats {
                w.write(format)?;
            }
        }

        Ok(())
    }
}

impl CommandReply for SourceInfo {}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct GetSourceInfo {
    pub index: Option<u32>,
    pub name: Option<CString>,
}

impl TagStructRead for GetSourceInfo {
    fn read(ts: &mut TagStructReader, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts.read_index()?,
            name: ts.read_string()?,
        })
    }
}

impl TagStructWrite for GetSourceInfo {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.write_index(self.index)?;
        w.write_string(self.name.as_ref())?;
        Ok(())
    }
}

pub type SourceInfoList = Vec<SourceInfo>;

impl CommandReply for SourceInfoList {}

impl TagStructRead for SourceInfoList {
    fn read(ts: &mut TagStructReader, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut sources = Vec::new();
        while ts.has_data_left()? {
            sources.push(ts.read()?);
        }

        Ok(sources)
    }
}

impl TagStructWrite for SourceInfoList {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        for info in self {
            w.write(info)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde::test_util::test_serde;

    use super::*;

    #[test]
    fn source_info_serde() -> anyhow::Result<()> {
        let source = SourceInfo {
            index: 0,
            name: CString::new("test").unwrap(),
            ..Default::default()
        };

        test_serde(&source)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use super::*;
    use crate::{
        integration_test_util::connect_and_init,
        protocol::{read_reply_message, write_command_message},
    };

    use pretty_assertions::assert_eq;

    #[test]
    fn list_sources() -> Result<(), Box<dyn std::error::Error>> {
        let mut sock = connect_and_init()?;

        write_command_message(sock.get_mut(), 0, Command::GetSourceInfoList)?;
        let (seq, info_list) = read_reply_message::<SourceInfoList>(&mut sock)?;
        assert_eq!(seq, 0);
        assert!(info_list.len() > 0);

        write_command_message(
            sock.get_mut(),
            1,
            Command::GetSourceInfo(GetSourceInfo {
                index: Some(info_list[0].index),
                ..Default::default()
            }),
        )?;

        let (seq, info) = read_reply_message::<SourceInfo>(&mut sock)?;
        assert_eq!(seq, 1);
        assert_eq!(info, info_list[0]);

        Ok(())
    }
}
