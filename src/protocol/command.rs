//! Commands are the top-level IPC structure used in the protocol.

use std::io::{BufRead, Write};

mod auth;
// mod create_playback_stream;
mod client_info;
mod module_info;
mod playback_stream;
mod playback_stream_events;
mod record_stream;
mod sample_info;
mod server_info;
mod set_client_name;
mod sink_info;
mod sink_input_info;
mod source_info;
mod source_output_info;
mod subscribe;
mod timing_info;

pub use auth::{AuthParams, AuthReply};
pub use client_info::*;
pub use module_info::*;
pub use playback_stream::*;
pub use playback_stream_events::*;
pub use record_stream::*;
pub use sample_info::*;
pub use server_info::*;
pub use set_client_name::*;
pub use sink_info::*;
pub use sink_input_info::*;
pub use source_info::*;
pub use source_output_info::*;
pub use subscribe::*;
pub use timing_info::*;

use super::{serde::*, ProtocolError, PulseError};

use enum_primitive_derive::Primitive;
use num_traits::FromPrimitive as _;

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Primitive)]
pub enum CommandTag {
    /* Generic commands */
    Error = 0,
    Timeout = 1, /* pseudo command */
    Reply = 2,   /* actually used for command replies */

    /* CLIENT->SERVER */
    CreatePlaybackStream = 3, /* Payload changed in v9, v12 (0.9.0, 0.9.8) */
    DeletePlaybackStream = 4,
    CreateRecordStream = 5, /* Payload changed in v9, v12 (0.9.0, 0.9.8) */
    DeleteRecordStream = 6,
    Exit = 7,
    Auth = 8,
    SetClientName = 9,
    LookupSink = 10,
    LookupSource = 11,
    DrainPlaybackStream = 12,
    Stat = 13,
    GetPlaybackLatency = 14,
    CreateUploadStream = 15,
    DeleteUploadStream = 16,
    FinishUploadStream = 17,
    PlaySample = 18,
    RemoveSample = 19,

    GetServerInfo = 20,
    GetSinkInfo = 21,
    GetSinkInfoList = 22,
    GetSourceInfo = 23,
    GetSourceInfoList = 24,
    GetModuleInfo = 25,
    GetModuleInfoList = 26,
    GetClientInfo = 27,
    GetClientInfoList = 28,
    GetSinkInputInfo = 29,     /* Payload changed in v11 (0.9.7) */
    GetSinkInputInfoList = 30, /* Payload changed in v11 (0.9.7) */
    GetSourceOutputInfo = 31,
    GetSourceOutputInfoList = 32,
    GetSampleInfo = 33,
    GetSampleInfoList = 34,
    Subscribe = 35,

    SetSinkVolume = 36,
    SetSinkInputVolume = 37,
    SetSourceVolume = 38,

    SetSinkMute = 39,
    SetSourceMute = 40,

    CorkPlaybackStream = 41,
    FlushPlaybackStream = 42,
    TriggerPlaybackStream = 43,

    SetDefaultSink = 44,
    SetDefaultSource = 45,

    SetPlaybackStreamName = 46,
    SetRecordStreamName = 47,

    KillClient = 48,
    KillSinkInput = 49,
    KillSourceOutput = 50,

    LoadModule = 51,
    UnloadModule = 52,

    /* Obsolete */
    AddAutoloadObsolete = 53,
    RemoveAutoloadObsolete = 54,
    GetAutoloadInfoObsolete = 55,
    GetAutoloadInfoListObsolete = 56,

    GetRecordLatency = 57,
    CorkRecordStream = 58,
    FlushRecordStream = 59,
    PrebufPlaybackStream = 60,

    /* SERVER->CLIENT */
    Request = 61,
    Overflow = 62,
    Underflow = 63,
    PlaybackStreamKilled = 64,
    RecordStreamKilled = 65,
    SubscribeEvent = 66,

    /* A few more client->server commands */

    /* Supported since protocol v10 (0.9.5) */
    MoveSinkInput = 67,
    MoveSourceOutput = 68,

    /* Supported since protocol v11 (0.9.7) */
    SetSinkInputMute = 69,

    SuspendSink = 70,
    SuspendSource = 71,

    /* Supported since protocol v12 (0.9.8) */
    SetPlaybackStreamBufferAttr = 72,
    SetRecordStreamBufferAttr = 73,

    UpdatePlaybackStreamSampleRate = 74,
    UpdateRecordStreamSampleRate = 75,

    /* SERVER->CLIENT */
    PlaybackStreamSuspended = 76,
    RecordStreamSuspended = 77,
    PlaybackStreamMoved = 78,
    RecordStreamMoved = 79,

    /* Supported since protocol v13 (0.9.11) */
    UpdateRecordStreamProplist = 80,
    UpdatePlaybackStreamProplist = 81,
    UpdateClientProplist = 82,
    RemoveRecordStreamProplist = 83,
    RemovePlaybackStreamProplist = 84,
    RemoveClientProplist = 85,

    /* SERVER->CLIENT */
    Started = 86,

    /* Supported since protocol v14 (0.9.12) */
    Extension = 87,

    /* Supported since protocol v15 (0.9.15) */
    GetCardInfo = 88,
    GetCardInfoList = 89,
    SetCardProfile = 90,

    ClientEvent = 91,
    PlaybackStreamEvent = 92,
    RecordStreamEvent = 93,

    /* SERVER->CLIENT */
    PlaybackBufferAttrChanged = 94,
    RecordBufferAttrChanged = 95,

    /* Supported since protocol v16 (0.9.16) */
    SetSinkPort = 96,
    SetSourcePort = 97,

    /* Supported since protocol v22 (1.0) */
    SetSourceOutputVolume = 98,
    SetSourceOutputMute = 99,

    /* Supported since protocol v27 (3.0) */
    SetPortLatencyOffset = 100,

    /* Supported since protocol v30 (6.0) */
    /* BOTH DIRECTIONS */
    EnableSrbchannel = 101,
    DisableSrbchannel = 102,

    /* Supported since protocol v31 (9.0)
     * BOTH DIRECTIONS */
    RegisterMemfdShmid = 103,
}

impl TagStructRead for CommandTag {
    fn read(r: &mut TagStructReader, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let v = r.read_u32()?;

        CommandTag::from_u32(v)
            .ok_or_else(|| ProtocolError::Invalid(format!("invalid command tag: {}", v)))
    }
}

impl TagStructWrite for CommandTag {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.write_u32(*self as u32)?;

        Ok(())
    }
}

// A marker trait for reply data.
pub trait CommandReply: TagStructRead + TagStructWrite {}

pub struct CommandError {
    pub code: PulseError,
}

#[derive(Debug)]
pub enum Command {
    /// A reply to some other command. If this is returned by read_tag_prefixed, the payload has yet to be read.
    Reply,

    /// Authentication request (and protocol handshake).
    Auth(AuthParams),

    /// Updates client properties (not just the name).
    SetClientName(Props),

    /// Create and delete streams.
    CreatePlaybackStream(PlaybackStreamParams),
    DeletePlaybackStream(u32),
    CreateRecordStream(RecordStreamParams),
    DeleteRecordStream(u32),
    DrainPlaybackStream(u32),
    GetPlaybackLatency(LatencyParams),
    GetRecordLatency(LatencyParams),

    /// So-called introspection commands, to read back the state of the server.
    GetServerInfo,
    GetSinkInfo(GetSinkInfo),
    GetSinkInfoList,
    GetSourceInfo(GetSourceInfo),
    GetSourceInfoList,
    GetModuleInfo(u32),
    GetModuleInfoList,
    GetClientInfo(u32),
    GetClientInfoList,
    GetSinkInputInfo(u32),
    GetSinkInputInfoList,
    GetSourceOutputInfo(u32),
    GetSourceOutputInfoList,
    GetSampleInfo(u32),
    GetSampleInfoList,
    Subscribe(SubscriptionMask),

    Request(Request),
    Overflow(u32),
    Underflow(Underflow),
    PlaybackStreamKilled(u32),
    RecordStreamKilled(u32),
    Started(u32),
    PlaybackBufferAttrChanged(PlaybackBufferAttrChanged),
    SubscribeEvent(SubscriptionEvent),
}

impl Command {
    pub fn read_tag_prefixed<R: BufRead>(
        r: &mut R,
        protocol_version: u16,
    ) -> Result<(u32, Self), ProtocolError> {
        let mut ts = TagStructReader::new(r, protocol_version);
        let (command, seq) = (ts.read_enum()?, ts.read_u32()?);

        let cmd = match command {
            CommandTag::Error => Err(ProtocolError::ServerError(ts.read_enum()?)),
            CommandTag::Timeout => Err(ProtocolError::Timeout),
            CommandTag::Reply => Ok(Command::Reply),

            CommandTag::Exit => Err(ProtocolError::Unimplemented(command)),
            CommandTag::Auth => Ok(Command::Auth(ts.read()?)),
            CommandTag::SetClientName => Ok(Command::SetClientName(ts.read()?)),

            CommandTag::CreatePlaybackStream => Ok(Command::CreatePlaybackStream(ts.read()?)),
            CommandTag::DeletePlaybackStream => Ok(Command::DeletePlaybackStream(ts.read_u32()?)),
            CommandTag::CreateRecordStream => Ok(Command::CreateRecordStream(ts.read()?)),
            CommandTag::DeleteRecordStream => Ok(Command::DeleteRecordStream(ts.read_u32()?)),
            CommandTag::LookupSink => Err(ProtocolError::Unimplemented(command)),
            CommandTag::LookupSource => Err(ProtocolError::Unimplemented(command)),
            CommandTag::DrainPlaybackStream => Err(ProtocolError::Unimplemented(command)),
            CommandTag::Stat => Err(ProtocolError::Unimplemented(command)),
            CommandTag::GetPlaybackLatency => Err(ProtocolError::Unimplemented(command)),
            CommandTag::CreateUploadStream => Err(ProtocolError::Unimplemented(command)),
            CommandTag::DeleteUploadStream => Err(ProtocolError::Unimplemented(command)),
            CommandTag::FinishUploadStream => Err(ProtocolError::Unimplemented(command)),
            CommandTag::PlaySample => Err(ProtocolError::Unimplemented(command)),
            CommandTag::RemoveSample => Err(ProtocolError::Unimplemented(command)),

            CommandTag::GetServerInfo => Ok(Command::GetServerInfo),
            CommandTag::GetSinkInfo => Ok(Command::GetSinkInfo(ts.read()?)),
            CommandTag::GetSinkInfoList => Ok(Command::GetSinkInfoList),
            CommandTag::GetSourceInfo => Ok(Command::GetSourceInfo(ts.read()?)),
            CommandTag::GetSourceInfoList => Ok(Command::GetSourceInfoList),
            CommandTag::GetModuleInfo => Ok(Command::GetModuleInfo(ts.read_u32()?)),
            CommandTag::GetModuleInfoList => Ok(Command::GetModuleInfoList),
            CommandTag::GetClientInfo => Ok(Command::GetClientInfo(ts.read_u32()?)),
            CommandTag::GetClientInfoList => Ok(Command::GetClientInfoList),
            CommandTag::GetSinkInputInfo => Ok(Command::GetSinkInputInfo(ts.read_u32()?)),
            CommandTag::GetSinkInputInfoList => Ok(Command::GetSinkInputInfoList),
            CommandTag::GetSourceOutputInfo => Ok(Command::GetSourceOutputInfo(ts.read_u32()?)),
            CommandTag::GetSourceOutputInfoList => Ok(Command::GetSourceOutputInfoList),
            CommandTag::GetSampleInfo => Ok(Command::GetSampleInfo(ts.read_u32()?)),
            CommandTag::GetSampleInfoList => Ok(Command::GetSampleInfoList),
            CommandTag::Subscribe => Ok(Command::Subscribe(ts.read()?)),
            CommandTag::SubscribeEvent => Ok(Command::SubscribeEvent(ts.read()?)),

            CommandTag::Request => Ok(Command::Request(ts.read()?)),
            CommandTag::Overflow => Ok(Command::Overflow(ts.read_u32()?)),
            CommandTag::Underflow => Ok(Command::Underflow(ts.read()?)),
            CommandTag::PlaybackStreamKilled => Ok(Command::PlaybackStreamKilled(ts.read_u32()?)),
            CommandTag::RecordStreamKilled => Ok(Command::RecordStreamKilled(ts.read_u32()?)),
            CommandTag::Started => Ok(Command::Started(ts.read_u32()?)),
            CommandTag::PlaybackBufferAttrChanged => {
                Ok(Command::PlaybackBufferAttrChanged(ts.read()?))
            }

            CommandTag::SetSinkVolume => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetSinkInputVolume => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetSourceVolume => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetSinkMute => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetSourceMute => Err(ProtocolError::Unimplemented(command)),
            CommandTag::CorkPlaybackStream => Err(ProtocolError::Unimplemented(command)),
            CommandTag::FlushPlaybackStream => Err(ProtocolError::Unimplemented(command)),
            CommandTag::TriggerPlaybackStream => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetDefaultSink => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetDefaultSource => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetPlaybackStreamName => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetRecordStreamName => Err(ProtocolError::Unimplemented(command)),
            CommandTag::KillClient => Err(ProtocolError::Unimplemented(command)),
            CommandTag::KillSinkInput => Err(ProtocolError::Unimplemented(command)),
            CommandTag::KillSourceOutput => Err(ProtocolError::Unimplemented(command)),
            CommandTag::LoadModule => Err(ProtocolError::Unimplemented(command)),
            CommandTag::UnloadModule => Err(ProtocolError::Unimplemented(command)),
            CommandTag::AddAutoloadObsolete => Err(ProtocolError::Unimplemented(command)),
            CommandTag::RemoveAutoloadObsolete => Err(ProtocolError::Unimplemented(command)),
            CommandTag::GetAutoloadInfoObsolete => Err(ProtocolError::Unimplemented(command)),
            CommandTag::GetAutoloadInfoListObsolete => Err(ProtocolError::Unimplemented(command)),
            CommandTag::GetRecordLatency => Err(ProtocolError::Unimplemented(command)),
            CommandTag::CorkRecordStream => Err(ProtocolError::Unimplemented(command)),
            CommandTag::FlushRecordStream => Err(ProtocolError::Unimplemented(command)),
            CommandTag::PrebufPlaybackStream => Err(ProtocolError::Unimplemented(command)),
            CommandTag::MoveSinkInput => Err(ProtocolError::Unimplemented(command)),
            CommandTag::MoveSourceOutput => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetSinkInputMute => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SuspendSink => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SuspendSource => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetPlaybackStreamBufferAttr => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetRecordStreamBufferAttr => Err(ProtocolError::Unimplemented(command)),
            CommandTag::UpdatePlaybackStreamSampleRate => {
                Err(ProtocolError::Unimplemented(command))
            }
            CommandTag::UpdateRecordStreamSampleRate => Err(ProtocolError::Unimplemented(command)),
            CommandTag::PlaybackStreamSuspended => Err(ProtocolError::Unimplemented(command)),
            CommandTag::RecordStreamSuspended => Err(ProtocolError::Unimplemented(command)),
            CommandTag::PlaybackStreamMoved => Err(ProtocolError::Unimplemented(command)),
            CommandTag::RecordStreamMoved => Err(ProtocolError::Unimplemented(command)),
            CommandTag::UpdateRecordStreamProplist => Err(ProtocolError::Unimplemented(command)),
            CommandTag::UpdatePlaybackStreamProplist => Err(ProtocolError::Unimplemented(command)),
            CommandTag::UpdateClientProplist => Err(ProtocolError::Unimplemented(command)),
            CommandTag::RemoveRecordStreamProplist => Err(ProtocolError::Unimplemented(command)),
            CommandTag::RemovePlaybackStreamProplist => Err(ProtocolError::Unimplemented(command)),
            CommandTag::RemoveClientProplist => Err(ProtocolError::Unimplemented(command)),
            CommandTag::Extension => Err(ProtocolError::Unimplemented(command)),
            CommandTag::GetCardInfo => Err(ProtocolError::Unimplemented(command)),
            CommandTag::GetCardInfoList => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetCardProfile => Err(ProtocolError::Unimplemented(command)),
            CommandTag::ClientEvent => Err(ProtocolError::Unimplemented(command)),
            CommandTag::PlaybackStreamEvent => Err(ProtocolError::Unimplemented(command)),
            CommandTag::RecordStreamEvent => Err(ProtocolError::Unimplemented(command)),

            CommandTag::RecordBufferAttrChanged => Err(ProtocolError::Unimplemented(command)),

            CommandTag::SetSinkPort => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetSourcePort => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetSourceOutputVolume => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetSourceOutputMute => Err(ProtocolError::Unimplemented(command)),
            CommandTag::SetPortLatencyOffset => Err(ProtocolError::Unimplemented(command)),
            CommandTag::EnableSrbchannel => Err(ProtocolError::Unimplemented(command)),
            CommandTag::DisableSrbchannel => Err(ProtocolError::Unimplemented(command)),
            CommandTag::RegisterMemfdShmid => Err(ProtocolError::Unimplemented(command)),
        }?;

        Ok((seq, cmd))
    }

    pub fn write_tag_prefixed<W: Write>(
        &self,
        seq: u32,
        w: &mut W,
        protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        let mut ts = TagStructWriter::new(w, protocol_version);

        ts.write_u32(self.tag() as u32)?;
        ts.write_u32(seq)?;
        ts.write(self)?;

        Ok(())
    }

    pub fn tag(&self) -> CommandTag {
        match self {
            Command::Reply => CommandTag::Reply,

            Command::Auth(_) => CommandTag::Auth,
            Command::SetClientName(_) => CommandTag::SetClientName,
            Command::CreatePlaybackStream(_) => CommandTag::CreatePlaybackStream,
            Command::DeletePlaybackStream(_) => CommandTag::DeletePlaybackStream,
            Command::CreateRecordStream(_) => CommandTag::CreateRecordStream,
            Command::DeleteRecordStream(_) => CommandTag::DeleteRecordStream,
            Command::DrainPlaybackStream(_) => CommandTag::DrainPlaybackStream,
            Command::GetPlaybackLatency(_) => CommandTag::GetPlaybackLatency,
            Command::GetRecordLatency(_) => CommandTag::GetRecordLatency,

            Command::GetServerInfo => CommandTag::GetServerInfo,
            Command::GetSinkInfo(_) => CommandTag::GetSinkInfo,
            Command::GetSinkInfoList => CommandTag::GetSinkInfoList,
            Command::GetSourceInfo(_) => CommandTag::GetSourceInfo,
            Command::GetSourceInfoList => CommandTag::GetSourceInfoList,
            Command::GetClientInfo(_) => CommandTag::GetClientInfo,
            Command::GetClientInfoList => CommandTag::GetClientInfoList,
            // Command::GetCardInfoList => CommandTag::GetCardInfoList,
            Command::GetModuleInfo(_) => CommandTag::GetModuleInfo,
            Command::GetModuleInfoList => CommandTag::GetModuleInfoList,
            Command::GetSinkInputInfo(_) => CommandTag::GetSinkInputInfo,
            Command::GetSinkInputInfoList => CommandTag::GetSinkInputInfoList,
            Command::GetSourceOutputInfo(_) => CommandTag::GetSourceOutputInfo,
            Command::GetSourceOutputInfoList => CommandTag::GetSourceOutputInfoList,
            Command::GetSampleInfo(_) => CommandTag::GetSampleInfo,
            Command::GetSampleInfoList => CommandTag::GetSampleInfoList,
            Command::Subscribe(_) => CommandTag::Subscribe,
            Command::SubscribeEvent(_) => CommandTag::SubscribeEvent,
            Command::Request(_) => CommandTag::Request,
            Command::Overflow(_) => CommandTag::Overflow,
            Command::Underflow(_) => CommandTag::Underflow,
            Command::PlaybackStreamKilled(_) => CommandTag::PlaybackStreamKilled,
            Command::RecordStreamKilled(_) => CommandTag::RecordStreamKilled,
            Command::Started(_) => CommandTag::Started,
            Command::PlaybackBufferAttrChanged(_) => CommandTag::PlaybackBufferAttrChanged,
        }
    }
}

impl TagStructWrite for Command {
    fn write(
        &self,
        w: &mut crate::protocol::serde::TagStructWriter,
        _protocol_version: u16,
    ) -> Result<(), crate::protocol::ProtocolError> {
        match self {
            Command::Reply => Ok(()),

            Command::Auth(ref p) => w.write(p),
            Command::SetClientName(ref p) => w.write(p),
            Command::CreatePlaybackStream(ref p) => w.write(p),
            Command::DeletePlaybackStream(chan) => w.write_u32(*chan),
            Command::CreateRecordStream(ref p) => w.write(p),
            Command::DeleteRecordStream(chan) => w.write_u32(*chan),
            Command::DrainPlaybackStream(chan) => w.write_u32(*chan),
            Command::GetPlaybackLatency(ref p) => w.write(p),
            Command::GetRecordLatency(ref p) => w.write(p),

            Command::GetSinkInfo(ref p) => w.write(p),
            Command::GetSourceInfo(ref p) => w.write(p),
            Command::GetModuleInfo(id) => w.write_u32(*id),
            Command::GetClientInfo(id) => w.write_u32(*id),
            Command::GetSinkInputInfo(id) => w.write_u32(*id),
            Command::GetSourceOutputInfo(id) => w.write_u32(*id),
            Command::GetSampleInfo(id) => w.write_u32(*id),
            Command::Subscribe(mask) => w.write(mask),
            Command::SubscribeEvent(ref p) => w.write(p),
            Command::Request(ref p) => w.write(p),
            Command::Overflow(chan) => w.write_u32(*chan),
            Command::Underflow(ref p) => w.write(p),
            Command::PlaybackStreamKilled(chan) => w.write_u32(*chan),
            Command::RecordStreamKilled(chan) => w.write_u32(*chan),
            Command::Started(chan) => w.write_u32(*chan),
            Command::PlaybackBufferAttrChanged(ref p) => w.write(p),
            Command::GetServerInfo
            | Command::GetSinkInfoList
            | Command::GetSourceInfoList
            | Command::GetModuleInfoList
            | Command::GetClientInfoList
            | Command::GetSinkInputInfoList
            | Command::GetSourceOutputInfoList
            | Command::GetSampleInfoList => Ok(()),
        }
    }
}
