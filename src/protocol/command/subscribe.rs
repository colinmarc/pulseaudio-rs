use bitflags::bitflags;
use enum_primitive_derive::Primitive;

use crate::protocol::{serde::*, ProtocolError};

bitflags! {
    /// A mask of events to subscribe to.
    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
    pub struct SubscriptionMask: u32 {
        /// Sink events.
        const SINK = 0x0001;

        /// Source events.
        const SOURCE = 0x0002;

        /// Sink input events.
        const SINK_INPUT = 0x0004;

        /// Source output events.
        const SOURCE_OUTPUT = 0x0008;

        /// Module events.
        const MODULE = 0x0010;

        /// Client events.
        const CLIENT = 0x0020;

        /// Sample cache events.
        const SAMPLE_CACHE = 0x0040;

        /// Server events.
        const SERVER = 0x0080;

        /// Autoload table events.
        #[deprecated]
        const AUTOLOAD = 0x0100;

        /// Card events.
        const CARD = 0x0200;

        /// All events.
        const ALL = 0x02ff;
    }
}

impl TagStructRead for SubscriptionMask {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self::from_bits_truncate(ts.read_u32()?))
    }
}

impl TagStructWrite for SubscriptionMask {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.bits())?;
        Ok(())
    }
}

/// The source of a subscription event.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Primitive)]
pub enum SubscriptionEventFacility {
    Sink = 0,
    Source = 1,
    SinkInput = 2,
    SourceOutput = 3,
    Module = 4,
    Client = 5,
    SampleCache = 6,
    Server = 7,
    Autoload = 8,
    Card = 9,
}

/// The type of event.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Primitive)]
pub enum SubscriptionEventType {
    New = 0x00,
    Changed = 0x20,
    Removed = 0x30,
}

const FACILITY_MASK: u32 = 0x0F;
const EVENT_TYPE_MASK: u32 = 0x30;

/// An event from the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubscriptionEvent {
    /// The source of the event, i.e. what kind of object it's referring to.
    pub event_facility: SubscriptionEventFacility,
    /// What kind of event it is, for example a new object, or a removed one.
    pub event_type: SubscriptionEventType,
    /// The ID of the object the event is about.
    pub index: Option<u32>,
}

impl TagStructRead for SubscriptionEvent {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        use num_traits::FromPrimitive as _;

        let raw = ts.read_u32()?;
        let event_facility = SubscriptionEventFacility::from_u32(raw & FACILITY_MASK)
            .ok_or_else(|| ProtocolError::Invalid(format!("invalid event facility: {}", raw)))?;
        let event_type = SubscriptionEventType::from_u32(raw & EVENT_TYPE_MASK)
            .ok_or_else(|| ProtocolError::Invalid(format!("invalid event type: {}", raw)))?;
        let index = ts.read_index()?;

        Ok(Self {
            event_facility,
            event_type,
            index,
        })
    }
}

impl TagStructWrite for SubscriptionEvent {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        let raw = (self.event_facility as u32) | (self.event_type as u32);
        w.write_u32(raw)?;
        w.write_index(self.index)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{test_util::test_serde_version, MAX_VERSION};

    #[test]
    fn subscription_mask_serde() -> anyhow::Result<()> {
        let mask = SubscriptionMask::SINK | SubscriptionMask::SOURCE;
        test_serde_version(&mask, MAX_VERSION)
    }

    #[test]
    fn subscription_event_serde() -> anyhow::Result<()> {
        let event = SubscriptionEvent {
            event_facility: SubscriptionEventFacility::Sink,
            event_type: SubscriptionEventType::New,
            index: Some(1),
        };
        test_serde_version(&event, MAX_VERSION)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use anyhow::Context as _;

    use super::*;
    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn subscribe() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        let mask = SubscriptionMask::SINK | SubscriptionMask::SOURCE;
        write_command_message(
            sock.get_mut(),
            0,
            Command::Subscribe(mask),
            protocol_version,
        )?;
        assert_eq!(0, read_ack_message(&mut sock).context("error reading ack")?);

        Ok(())
    }
}
