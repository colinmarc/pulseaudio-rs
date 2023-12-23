//! The `GET_*_INFO` and `GET_*_INFO_LIST` commands.

use crate::protocol::{serde::*, sink::*, ProtocolError};

use super::CommandReply;

pub type SinkInfoList = Vec<SinkInfo>;

impl CommandReply for SinkInfoList {}

impl TagStructRead for SinkInfoList {
    fn read(ts: &mut TagStructReader, protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut sinks = Vec::new();
        while ts.has_data_left()? {
            sinks.push(read_sink_info(ts, protocol_version)?);
        }

        Ok(sinks)
    }
}

fn read_sink_info(
    ts: &mut TagStructReader,
    protocol_version: u16,
) -> Result<SinkInfo, ProtocolError> {
    let mut sink = SinkInfo {
        index: ts.read_u32()?,
        name: ts.read_string()?.unwrap_or_default().to_owned(),
        description: ts.read_string()?,
        sample_spec: ts.read_sample_spec()?,
        channel_map: ts.read_channel_map()?,
        owner_module_index: ts.read_index()?,
        cvolume: ts.read_cvolume()?,
        muted: ts.read_bool()?,
        monitor_source_index: ts.read_index()?,
        monitor_source_name: ts.read_string()?,
        actual_latency: ts.read_usec()?,
        driver: ts.read_string()?,
        flags: SinkFlags::from_bits_truncate(ts.read_u32()?),
        props: ts.read_proplist()?,
        requested_latency: ts.read_usec()?,
        ..Default::default()
    };

    if protocol_version >= 15 {
        sink.base_volume = ts.read_volume()?;

        sink.state = ts.read_enum()?;
        sink.volume_steps = ts.read_u32()?;
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
            sink.formats.push(ts.read_format_info()?);
        }
    }

    Ok(sink)
}

impl TagStructWrite for SinkInfoList {
    fn write(&self, w: &mut TagStructWriter, protocol_version: u16) -> Result<(), ProtocolError> {
        for sink in self {
            w.write_u32(sink.index)?;
            w.write_string(Some(&sink.name))?;
            w.write_string(sink.description.as_ref())?;
            w.write(sink.sample_spec.protocol_downgrade(protocol_version))?;
            w.write(&sink.channel_map)?;
            w.write_index(sink.owner_module_index)?;
            w.write(&sink.cvolume)?;
            w.write_bool(sink.muted)?;
            w.write_index(sink.monitor_source_index)?; // sink's monitor source
            w.write_string(sink.monitor_source_name.as_ref())?;
            w.write_usec(sink.actual_latency)?;
            w.write_string(sink.driver.as_ref())?; // TODO: driver name
            w.write_u32(sink.flags.bits())?;
            // proto>=13
            w.write(&sink.props)?;
            w.write_usec(sink.requested_latency)?;
            if protocol_version >= 15 {
                w.write(sink.base_volume)?;
                w.write_u32(sink.state as u32)?;
                w.write_u32(sink.volume_steps)?;
                w.write_index(sink.card_index)?;
            }
            if protocol_version >= 16 {
                // send sink port info
                w.write_u32(sink.ports.len() as u32)?;
                for port in &sink.ports {
                    w.write_string(Some(&port.name))?;
                    w.write_string(port.description.as_ref())?;
                    w.write_u32(port.priority)?;
                    if protocol_version >= 24 {
                        w.write_u32(port.available as u32)?;
                    }
                }

                // active port name
                let active_port_name = if sink.active_port < sink.ports.len() {
                    Some(&sink.ports[sink.active_port].name)
                } else {
                    None
                };

                w.write_string(active_port_name)?;
            }
            if protocol_version >= 21 {
                // send supported sample formats
                w.write_u8(sink.formats.len() as u8)?;
                for format in &sink.formats {
                    w.write(format)?;
                }
            }
        }

        Ok(())
    }
}

// // FIXME: `pactl list` hangs after receiving this - maybe it expects at least one module?
// // (doesn't seem like it - it still hangs)
// #[derive(Debug)]
// pub struct GetModuleInfoListReply {}

// impl GetModuleInfoListReply {
//     /// Creates a dummy reply for servers that do not support modules.
//     pub fn new_dummy() -> Self {
//         Self {}
//     }
// }

// impl ToTagStruct for GetModuleInfoListReply {
//     fn write<W>(&self, w: &mut TagStructWriter, protocol_version: u16) -> Result<(), ProtocolError> {
//         w.write(0u32); // ID
//         w.write(CStr::from_bytes_with_nul(b"Default Module\0").unwrap());
//         w.write(<&CStr>::default()); // "argument"
//         w.write(1u32); // "get_n_used" users of module?

//         if protocol_version < 15 {
//             w.write(false); // autoload
//         } else {
//             w.write(PropList::new()); // module props
//         }

//         Ok(())
//     }
// }

// #[derive(Debug)]
// pub struct GetClientInfoListReply<I>(I);

// impl<I: IntoIterator<Item = impl Borrow<ClientInfo<'a>>>> GetClientInfoListReply<I>
// where
//     for<'a> &'a I: IntoIterator<Item = &'a ClientInfo<'a>>,
// {
//     pub fn new(sinks: I) -> Self {
//         Self(sinks)
//     }
// }

// impl<I: IntoIterator<Item = impl Borrow<Sink>>> ToTagStruct for GetClientInfoListReply<I>
// where
//     for<'a> &'a I: IntoIterator<Item = &'a Sink>,
// {
//     fn write<W>(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
//         for client in sink.clients {
//             w.write(client.index);
//             w.write(client.app_name);
//             w.write(u32::MAX); // INVALID_INDEX = no/unknown module
//             w.write(client.driver);
//             w.write(client.props);
//         }
//         Ok(())
//     }
// }

// #[derive(Debug)]
// pub struct ClientInfo<'a> {
//     index: u32,
//     app_name: &'a CStr,
//     driver: &'a CStr,
//     props: &'a PropList, // proto>=13
// }

// impl<'a> ClientInfo<'a> {
//     pub fn new(index: u32, driver: &'a CStr, props: &'a PropList) -> Self {
//         Self {
//             index,
//             app_name: props.get_c_str(Prop::ApplicationName).unwrap_or_default(),
//             driver,
//             props,
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;
    use crate::protocol::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_sink_info_list_round_trip() {
        let mut props1 = Props::new();
        props1.set(Prop::DeviceString, "foo");

        let mut props2 = Props::new();
        props2.set(Prop::ApplicationName, "bar");

        let sinks = vec![
            SinkInfo {
                index: 0,
                name: CString::new("sink0").unwrap(),
                props: props1,
                sample_spec: SampleSpec::new(SampleFormat::S16Le, 2, 44100).expect("samplespec"),
                ..Default::default()
            },
            SinkInfo {
                index: 1,
                name: CString::new("sink1").unwrap(),
                props: props2,
                sample_spec: SampleSpec::new(SampleFormat::S16Le, 2, 44100).expect("samplespec"),
                ..Default::default()
            },
        ];

        for version in PROTOCOL_MIN_VERSION..PROTOCOL_VERSION {
            let mut buf = Cursor::new(Vec::with_capacity(1024));

            {
                let mut w = TagStructWriter::new(&mut buf, version);
                w.write(&sinks).unwrap_or_else(|e| {
                    panic!("serialize for protocol version {}: {}", version, e)
                });
            }

            buf.set_position(0);
            let mut ts = TagStructReader::new(&mut buf, version);
            let sinks2: SinkInfoList = ts
                .read()
                .unwrap_or_else(|e| panic!("deserialize for protocol version {}: {}", version, e));

            assert_eq!(&sinks, &sinks2);
        }
    }
}
