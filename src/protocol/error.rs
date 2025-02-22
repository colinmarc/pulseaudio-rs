//! Defines error types and codes.

use enum_primitive_derive::Primitive;
use thiserror::Error;

use super::command::CommandTag;

/// A generic protocol error.
#[derive(Error, Debug)]
pub enum ProtocolError {
    /// The version is not supported by this library.
    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(u16),
    /// A command other than what we were expecting was received.
    #[error("unexpected command: {0:?}")]
    UnexpectedCommand(CommandTag),
    /// The message is invalid.
    #[error("invalid IPC message: {0}")]
    Invalid(String),
    /// An I/O error occurred, such as an unexpected EOF while reading a tagstruct.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// The command is not yet implemented.
    #[error("unimplemented command: {1:?}")]
    Unimplemented(u32, CommandTag),
    /// An error from a remote server.
    #[error("server error: {0:?}")]
    ServerError(PulseError),
    /// The server disconnected.
    #[error("timeout received from server")]
    Timeout,
}

/// An error code understood by the PulseAudio protocol.
///
/// Can be sent to clients to inform them of a specific error.
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Primitive)]
pub enum PulseError {
    /// Access failure
    AccessDenied = 1,
    /// Unknown command
    Command = 2,
    /// Invalid argument
    Invalid = 3,
    /// Entity exists
    Exist = 4,
    /// No such entity
    NoEntity = 5,
    /// Connection refused
    ConnectionRefused = 6,
    /// Protocol error
    Protocol = 7,
    /// Timeout
    Timeout = 8,
    /// No authentication key
    AuthKey = 9,
    /// Internal error
    Internal = 10,
    /// Connection terminated
    ConnectionTerminated = 11,
    /// Entity killed
    Killed = 12,
    /// Invalid server
    InvalidServer = 13,
    /// Module initialization failed
    ModInitFailed = 14,
    /// Bad state
    BadState = 15,
    /// No data
    NoData = 16,
    /// Incompatible protocol version
    Version = 17,
    /// Data too large
    TooLarge = 18,
    /// Operation not supported (since 0.9.5)
    NotSupported = 19,
    /// The error code was unknown to the client
    Unknown = 20,
    /// Extension does not exist. (since 0.9.12)
    NoExtension = 21,
    /// Obsolete functionality. (since 0.9.15)
    Obsolete = 22,
    /// Missing implementation. (since 0.9.15)
    NotImplemented = 23,
    /// The caller forked without calling execve() and tried to reuse the context. \since 0.9.15
    Forked = 24,
    /// An IO error happened. (since 0.9.16)
    Io = 25,
    /// Device or resource busy. (since 0.9.17)
    Busy = 26,
}

impl std::fmt::Display for PulseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PulseError::AccessDenied => write!(f, "Access failure"),
            PulseError::Command => write!(f, "Unknown command"),
            PulseError::Invalid => write!(f, "Invalid argument"),
            PulseError::Exist => write!(f, "Entity exists"),
            PulseError::NoEntity => write!(f, "No such entity"),
            PulseError::ConnectionRefused => write!(f, "Connection refused"),
            PulseError::Protocol => write!(f, "Protocol error"),
            PulseError::Timeout => write!(f, "Timeout"),
            PulseError::AuthKey => write!(f, "No authentication key"),
            PulseError::Internal => write!(f, "Internal error"),
            PulseError::ConnectionTerminated => write!(f, "Connection terminated"),
            PulseError::Killed => write!(f, "Entity killed"),
            PulseError::InvalidServer => write!(f, "Invalid server"),
            PulseError::ModInitFailed => write!(f, "Module initialization failed"),
            PulseError::BadState => write!(f, "Bad state"),
            PulseError::NoData => write!(f, "No data"),
            PulseError::Version => write!(f, "Incompatible protocol version"),
            PulseError::TooLarge => write!(f, "Data too large"),
            PulseError::NotSupported => write!(f, "Operation not supported"),
            PulseError::Unknown => write!(f, "The error code was unknown to the client"),
            PulseError::NoExtension => write!(f, "Extension does not exist"),
            PulseError::Obsolete => write!(f, "Obsolete functionality"),
            PulseError::NotImplemented => write!(f, "Missing implementation"),
            PulseError::Forked => write!(f, "Caller tried to reuse the context"),
            PulseError::Io => write!(f, "An IO error occurred"),
            PulseError::Busy => write!(f, "Device or resource busy"),
        }
    }
}
