use std::ffi::CString;

use super::*;

/// A port on a card.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CardPortInfo {
    /// The name of the port.
    pub name: CString,

    /// The type of port this is.
    pub port_type: port_info::PortType,

    /// A description of the port.
    pub description: Option<CString>,

    /// The properties of the port.
    pub props: Props,

    /// The direction of the port.
    pub dir: port_info::PortDirection,

    /// The priority of the port.
    pub priority: u32,

    /// Whether the port is available.
    pub available: port_info::PortAvailable,

    /// Ports in this group share availability status with each other.
    pub availability_group: Option<CString>,

    /// The list of profile names that apply to the port.
    pub profiles: Vec<CString>,

    /// The latency offset of the port, added to the sink/source latency.
    pub latency_offset: u64,
}

/// A profile for a card.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CardProfileInfo {
    /// The name of the profile.
    pub name: CString,

    /// A description of the profile.
    pub description: Option<CString>,

    /// The priority of the profile.
    pub priority: u32,

    /// Whether the profile is available.
    pub available: u32,

    /// The number of sinks this profile would create.
    pub num_sinks: u32,

    /// The number of sources this profile would create.
    pub num_sources: u32,
}

/// A card on a PulseAudio server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardInfo {
    /// Server-internal card ID.
    pub index: u32,

    /// The human readable name of the card.
    pub name: CString,

    /// A list of properties.
    pub props: Props,

    /// The ID of the module that owns this card.
    pub owner_module_index: Option<u32>,

    /// The name of the driver used for this card.
    pub driver: Option<CString>,

    /// The ports of the card.
    pub ports: Vec<CardPortInfo>,

    /// A list of available profiles for the card.
    pub profiles: Vec<CardProfileInfo>,

    /// The name of the currently active profile.
    pub active_profile: Option<CString>,
}

/// The parameters for [`Command::GetCardInfo`]. Either the card index or the
/// card name should be specified.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct GetCardInfo {
    /// The index of the card to query.
    pub index: Option<u32>,

    /// The name of the card to query.
    pub name: Option<CString>,
}

impl TagStructRead for GetCardInfo {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts.read_index()?,
            name: ts.read_string()?,
        })
    }
}

impl TagStructWrite for GetCardInfo {
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

impl CommandReply for CardInfo {}

impl TagStructRead for CardInfo {
    fn read(ts: &mut TagStructReader<'_>, protocol_version: u16) -> Result<Self, ProtocolError> {
        let index = ts
            .read_index()?
            .ok_or_else(|| ProtocolError::Invalid("invalid index".to_string()))?;

        let name = ts.read_string_non_null()?;
        let owner_module_index = ts.read_index()?;
        let driver = ts.read_string()?;

        let mut profiles = Vec::new();
        for _ in 0..ts.read_u32()? {
            profiles.push(CardProfileInfo {
                name: ts.read_string_non_null()?,
                description: ts.read_string()?,
                num_sinks: ts.read_u32()?,
                num_sources: ts.read_u32()?,
                priority: ts.read_u32()?,
                available: if protocol_version >= 29 {
                    ts.read_u32()?
                } else {
                    0
                },
            });
        }

        let active_profile = ts.read_string()?;
        let props = ts.read()?;

        let mut ports = Vec::new();
        if protocol_version >= 26 {
            for _ in 0..ts.read_u32()? {
                let name = ts.read_string_non_null()?;
                let description = ts.read_string()?;
                let priority = ts.read_u32()?;
                let available = ts.read_enum()?;
                let dir = ts
                    .read_u8()?
                    .try_into()
                    .map_err(|_| ProtocolError::Invalid("invalid port direction".to_string()))?;
                let props = ts.read()?;

                let mut profiles = Vec::new();
                for _ in 0..ts.read_u32()? {
                    profiles.push(ts.read_string_non_null()?);
                }

                let latency_offset = if protocol_version >= 27 {
                    ts.read_i64()?.try_into().map_err(|_| {
                        ProtocolError::Invalid("latency offset cannot be negative".to_string())
                    })?
                } else {
                    0
                };

                let (availability_group, port_type) = if protocol_version >= 34 {
                    (ts.read_string()?, ts.read_enum()?)
                } else {
                    (None, port_info::PortType::Unknown)
                };

                ports.push(CardPortInfo {
                    name,
                    description,
                    priority,
                    available,
                    dir,
                    props,
                    profiles,
                    port_type,
                    availability_group,
                    latency_offset,
                });
            }
        }

        Ok(Self {
            index,
            name,
            props,
            owner_module_index,
            driver,
            ports,
            profiles,
            active_profile,
        })
    }
}

impl TagStructWrite for CardInfo {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(Some(self.index))?;
        ts.write_string(Some(&self.name))?;
        ts.write_index(self.owner_module_index)?;
        ts.write_string(self.driver.as_ref())?;

        ts.write_u32(self.profiles.len() as u32)?;
        for profile in &self.profiles {
            ts.write_string(Some(&profile.name))?;
            ts.write_string(profile.description.as_ref())?;
            ts.write_u32(profile.num_sinks)?;
            ts.write_u32(profile.num_sources)?;
            ts.write_u32(profile.priority)?;
            if protocol_version >= 29 {
                ts.write_u32(profile.available)?;
            }
        }

        ts.write_string(self.active_profile.as_ref())?;
        ts.write(&self.props)?;

        if protocol_version >= 26 {
            ts.write_u32(self.ports.len() as u32)?;
            for port in &self.ports {
                ts.write_string(Some(&port.name))?;
                ts.write_string(port.description.as_ref())?;
                ts.write_u32(port.priority)?;
                ts.write_u32(port.available as u32)?;
                ts.write_u8(port.dir as u8)?;
                ts.write(&port.props)?;
                ts.write_u32(port.profiles.len() as u32)?;
                for profile in &port.profiles {
                    ts.write_string(Some(profile))?;
                }

                if protocol_version >= 27 {
                    ts.write_i64(port.latency_offset as i64)?;
                }

                if protocol_version >= 34 {
                    ts.write_string(port.availability_group.as_ref())?;
                    ts.write_u32(port.port_type as u32)?;
                }
            }
        }

        Ok(())
    }
}

/// The server reply to [`super::Command::GetCardInfoList`].
pub type CardInfoList = Vec<CardInfo>;

impl CommandReply for CardInfoList {}

impl TagStructRead for CardInfoList {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut cards = Vec::new();
        while ts.has_data_left()? {
            cards.push(ts.read()?);
        }

        Ok(cards)
    }
}

impl TagStructWrite for CardInfoList {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        for card in self {
            w.write(card)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{self, test_util::test_serde_version};

    #[test]
    fn test_card_info_serde() -> anyhow::Result<()> {
        let info = CardInfo {
            index: 0,
            name: CString::new("name").unwrap(),
            props: Props::new(),
            owner_module_index: None,
            driver: None,
            ports: vec![
                CardPortInfo {
                    name: CString::new("name").unwrap(),
                    description: None,
                    priority: 0,
                    available: port_info::PortAvailable::Unknown,
                    dir: port_info::PortDirection::Input,
                    props: Props::new(),
                    profiles: Vec::new(),
                    port_type: port_info::PortType::Unknown,
                    availability_group: None,
                    latency_offset: 0,
                },
                CardPortInfo {
                    name: CString::new("name").unwrap(),
                    description: None,
                    priority: 0,
                    available: port_info::PortAvailable::Unknown,
                    dir: port_info::PortDirection::Output,
                    props: Props::new(),
                    profiles: vec![CString::new("profile1")?],
                    port_type: port_info::PortType::Unknown,
                    availability_group: None,
                    latency_offset: 0,
                },
            ],
            profiles: vec![CardProfileInfo {
                name: CString::new("profile1").unwrap(),
                description: None,
                priority: 123,
                available: 0,
                num_sinks: 1,
                num_sources: 1,
            }],
            active_profile: Some(CString::new("profile1").unwrap()),
        };

        test_serde_version(&info, protocol::MAX_VERSION)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use super::*;
    use crate::{integration_test_util::*, protocol};

    #[test]
    fn get_card_info_list() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        protocol::write_command_message(
            sock.get_mut(),
            0,
            &Command::GetCardInfoList,
            protocol_version,
        )?;
        let _ = protocol::read_reply_message::<CardInfoList>(&mut sock, protocol_version)?;

        Ok(())
    }
}
