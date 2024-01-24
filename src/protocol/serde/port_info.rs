//! Device ports (source and sink).

use std::ffi::CString;

use enum_primitive_derive::Primitive;

/// Specifies the direction of a port.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PortDirection {
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

/// Port type.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Primitive)]
#[allow(missing_docs)]
pub enum PortType {
    /// Unknown port type.
    #[default]
    Unknown = 0,
    Aux = 1,
    Speaker = 2,
    Headphones = 3,
    Line = 4,
    Mic = 5,
    Headset = 6,
    Handset = 7,
    Earpiece = 8,
    Spdif = 9,
    Hdmi = 10,
    Tv = 11,
    Radio = 12,
    Video = 13,
    Usb = 14,
    Bluetooth = 15,
    Portable = 16,
    Handsfree = 17,
    Car = 18,
    Hifi = 19,
    Phone = 20,
    Network = 21,
    Analog = 22,
}

/// A port on a sink or source.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PortInfo {
    /// The name of the port.
    pub name: CString,

    /// The type of port this is.
    pub port_type: PortType,

    /// A description of the port.
    pub description: Option<CString>,

    /// The direction of the port.
    pub dir: PortDirection,

    /// The priority of the port.
    pub priority: u32,

    /// Whether the port is available.
    pub available: PortAvailable,

    /// Ports in this group share availability status with each other.
    pub availability_group: Option<CString>,
}
