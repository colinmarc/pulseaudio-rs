//! Commands are the top-level IPC structure used in the protocol.

use std::{
    ffi::CString,
    io::{BufRead, Write},
};

mod auth;
mod card_info;
mod client_event;
mod client_info;
mod extension;
mod load_module;
mod lookup;
mod module_info;
mod move_stream;
mod playback_stream;
mod record_stream;
mod sample;
mod sample_info;
mod server_info;
mod set_card_profile;
mod set_client_name;
mod set_port;
mod set_port_latency_offset;
mod sink_info;
mod sink_input_info;
mod source_info;
mod source_output_info;
mod stat;
mod stream_events;
mod subscribe;
mod suspend;
mod timing_info;
mod update_client;
mod update_stream;
mod upload_stream;
mod volume;

pub use auth::*;
pub use card_info::*;
pub use client_event::*;
pub use client_info::*;
pub use extension::*;
pub use load_module::*;
pub use lookup::*;
pub use module_info::*;
pub use move_stream::*;
pub use playback_stream::*;
pub use record_stream::*;
pub use sample::*;
pub use sample_info::*;
pub use server_info::*;
pub use set_card_profile::*;
pub use set_client_name::*;
pub use set_port::*;
pub use set_port_latency_offset::*;
pub use sink_info::*;
pub use sink_input_info::*;
pub use source_info::*;
pub use source_output_info::*;
pub use stat::*;
pub use stream_events::*;
pub use subscribe::*;
pub use suspend::*;
pub use timing_info::*;
pub use update_client::*;
pub use update_stream::*;
pub use upload_stream::*;
pub use volume::*;

use super::{serde::*, ProtocolError, PulseError};

use enum_primitive_derive::Primitive;

/// A tag describing a command payload.
#[allow(missing_docs)]
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Primitive)]
pub enum CommandTag {
    /* Generic commands */
    Error = 0,
    Timeout = 1,              /* pseudo command */
    Reply = 2,                /* actually used for command replies */
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
    // AddAutoloadObsolete = 53,
    // RemoveAutoloadObsolete = 54,
    // GetAutoloadInfoObsolete = 55,
    // GetAutoloadInfoListObsolete = 56,
    GetRecordLatency = 57,
    CorkRecordStream = 58,
    FlushRecordStream = 59,
    PrebufPlaybackStream = 60,
    Request = 61,
    Overflow = 62,
    Underflow = 63,
    PlaybackStreamKilled = 64,
    RecordStreamKilled = 65,
    SubscribeEvent = 66,
    MoveSinkInput = 67,
    MoveSourceOutput = 68,
    SetSinkInputMute = 69,
    SuspendSink = 70,
    SuspendSource = 71,
    SetPlaybackStreamBufferAttr = 72,
    SetRecordStreamBufferAttr = 73,
    UpdatePlaybackStreamSampleRate = 74,
    UpdateRecordStreamSampleRate = 75,
    PlaybackStreamSuspended = 76,
    RecordStreamSuspended = 77,
    PlaybackStreamMoved = 78,
    RecordStreamMoved = 79,
    UpdateRecordStreamProplist = 80,
    UpdatePlaybackStreamProplist = 81,
    UpdateClientProplist = 82,
    RemoveRecordStreamProplist = 83,
    RemovePlaybackStreamProplist = 84,
    RemoveClientProplist = 85,
    Started = 86,
    Extension = 87,
    GetCardInfo = 88,
    GetCardInfoList = 89,
    SetCardProfile = 90,
    ClientEvent = 91,
    PlaybackStreamEvent = 92,
    RecordStreamEvent = 93,
    PlaybackBufferAttrChanged = 94,
    RecordBufferAttrChanged = 95,
    SetSinkPort = 96,
    SetSourcePort = 97,
    SetSourceOutputVolume = 98,
    SetSourceOutputMute = 99,
    SetPortLatencyOffset = 100,
    EnableSrbchannel = 101,
    DisableSrbchannel = 102,
    RegisterMemfdShmid = 103,
    SendObjectMessage = 104,
}

/// A marker trait for reply data.
pub trait CommandReply: TagStructRead + TagStructWrite {}

#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum Command {
    /// An error reply to some other command.
    Error(PulseError),
    Timeout,
    Exit,

    /// A reply to some other command. If this is returned by [`Command::read_tag_prefixed`], the payload has yet to be read.
    Reply,

    // Authentication request (and protocol handshake).
    Auth(AuthParams),

    // Updates client properties (not just the name).
    SetClientName(Props),

    // Create and manage streams.
    CreatePlaybackStream(PlaybackStreamParams),
    DeletePlaybackStream(u32),
    CreateRecordStream(RecordStreamParams),
    DeleteRecordStream(u32),
    DrainPlaybackStream(u32),
    GetPlaybackLatency(LatencyParams),
    GetRecordLatency(LatencyParams),
    CreateUploadStream(UploadStreamParams),
    DeleteUploadStream(u32),
    FinishUploadStream(u32),
    CorkPlaybackStream(CorkStreamParams),
    CorkRecordStream(CorkStreamParams),
    FlushPlaybackStream(u32),
    FlushRecordStream(u32),
    PrebufPlaybackStream(u32),
    TriggerPlaybackStream(u32),
    SetPlaybackStreamName(SetStreamNameParams),
    SetRecordStreamName(SetStreamNameParams),
    SetPlaybackStreamBufferAttr(SetPlaybackStreamBufferAttrParams),
    SetRecordStreamBufferAttr(SetRecordStreamBufferAttrParams),
    UpdatePlaybackStreamProplist(UpdatePropsParams),
    UpdateRecordStreamProplist(UpdatePropsParams),
    RemovePlaybackStreamProplist(u32),
    RemoveRecordStreamProplist(u32),
    UpdatePlaybackStreamSampleRate(UpdateSampleRateParams),
    UpdateRecordStreamSampleRate(UpdateSampleRateParams),

    // So-called introspection commands, to read back the state of the server.
    Stat,
    GetServerInfo,
    GetCardInfo(u32),
    GetCardInfoList,
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
    LookupSink(CString),
    LookupSource(CString),
    Subscribe(SubscriptionMask),

    // Server management commands.
    SetDefaultSink(CString),
    SetDefaultSource(CString),
    SetSinkPort(SetPortParams),
    SetSourcePort(SetPortParams),
    SetCardProfile(SetCardProfileParams),
    KillClient(u32),
    KillSinkInput(u32),
    KillSourceOutput(u32),
    MoveSinkInput(MoveStreamParams),
    MoveSourceOutput(MoveStreamParams),
    SuspendSink(SuspendParams),
    SuspendSource(SuspendParams),
    UpdateClientProplist(UpdateClientProplistParams),
    RemoveClientProplist,
    SetPortLatencyOffset(SetPortLatencyOffsetParams),

    // Manage samples.
    PlaySample(PlaySampleParams),
    RemoveSample(CString),

    // Manage modules.
    LoadModule(LoadModuleParams),
    UnloadModule(u32),
    Extension(ExtensionParams),

    // Set volume and mute.
    SetSinkVolume(SetDeviceVolumeParams),
    SetSinkInputVolume(SetStreamVolumeParams),
    SetSourceVolume(SetDeviceVolumeParams),
    SetSourceOutputVolume(SetStreamVolumeParams),
    SetSinkMute(SetDeviceMuteParams),
    SetSinkInputMute(SetStreamMuteParams),
    SetSourceMute(SetDeviceMuteParams),
    SetSourceOutputMute(SetStreamMuteParams),

    // Events from the server to the client.
    Started(u32),
    Request(Request),
    Overflow(u32),
    Underflow(Underflow),
    PlaybackStreamKilled(u32),
    RecordStreamKilled(u32),
    PlaybackStreamSuspended(StreamSuspendedParams),
    RecordStreamSuspended(StreamSuspendedParams),
    PlaybackStreamMoved(PlaybackStreamMovedParams),
    RecordStreamMoved(RecordStreamMovedParams),
    PlaybackBufferAttrChanged(PlaybackBufferAttrChanged),
    RecordBufferAttrChanged(RecordBufferAttrChanged),
    ClientEvent(ClientEvent),
    PlaybackStreamEvent(GenericStreamEvent),
    RecordStreamEvent(GenericStreamEvent),
    SubscribeEvent(SubscriptionEvent),
}

impl Command {
    /// Read a command message from a tagstruct. A result of [`Command::Reply`]
    /// indicates that the payload has yet to be read.
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

            CommandTag::Exit => Ok(Command::Exit),
            CommandTag::Auth => Ok(Command::Auth(ts.read()?)),
            CommandTag::SetClientName => Ok(Command::SetClientName(ts.read()?)),

            CommandTag::CreatePlaybackStream => Ok(Command::CreatePlaybackStream(ts.read()?)),
            CommandTag::DeletePlaybackStream => Ok(Command::DeletePlaybackStream(ts.read_u32()?)),
            CommandTag::CreateRecordStream => Ok(Command::CreateRecordStream(ts.read()?)),
            CommandTag::DeleteRecordStream => Ok(Command::DeleteRecordStream(ts.read_u32()?)),
            CommandTag::LookupSink => Ok(Command::LookupSink(ts.read_string_non_null()?)),
            CommandTag::LookupSource => Ok(Command::LookupSource(ts.read_string_non_null()?)),
            CommandTag::DrainPlaybackStream => Ok(Command::DrainPlaybackStream(ts.read_u32()?)),
            CommandTag::Stat => Ok(Command::Stat),
            CommandTag::GetPlaybackLatency => Ok(Command::GetPlaybackLatency(ts.read()?)),
            CommandTag::CreateUploadStream => Ok(Command::CreateUploadStream(ts.read()?)),
            CommandTag::DeleteUploadStream => Ok(Command::DeleteUploadStream(ts.read_u32()?)),
            CommandTag::FinishUploadStream => Ok(Command::FinishUploadStream(ts.read_u32()?)),
            CommandTag::PlaySample => Ok(Command::PlaySample(ts.read()?)),
            CommandTag::RemoveSample => Ok(Command::RemoveSample(ts.read_string_non_null()?)),

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

            CommandTag::SetSinkVolume => Ok(Command::SetSinkVolume(ts.read()?)),
            CommandTag::SetSinkInputVolume => Ok(Command::SetSinkInputVolume(ts.read()?)),
            CommandTag::SetSourceVolume => Ok(Command::SetSourceVolume(ts.read()?)),
            CommandTag::SetSinkMute => Ok(Command::SetSinkMute(ts.read()?)),
            CommandTag::SetSourceMute => Ok(Command::SetSourceMute(ts.read()?)),
            CommandTag::CorkPlaybackStream => Ok(Command::CorkPlaybackStream(ts.read()?)),
            CommandTag::FlushPlaybackStream => Ok(Command::FlushPlaybackStream(ts.read_u32()?)),
            CommandTag::TriggerPlaybackStream => Ok(Command::TriggerPlaybackStream(ts.read_u32()?)),
            CommandTag::SetDefaultSink => Ok(Command::SetDefaultSink(ts.read_string_non_null()?)),
            CommandTag::SetDefaultSource => {
                Ok(Command::SetDefaultSource(ts.read_string_non_null()?))
            }
            CommandTag::SetPlaybackStreamName => Ok(Command::SetPlaybackStreamName(ts.read()?)),
            CommandTag::SetRecordStreamName => Ok(Command::SetRecordStreamName(ts.read()?)),
            CommandTag::KillClient => Ok(Command::KillClient(ts.read_u32()?)),
            CommandTag::KillSinkInput => Ok(Command::KillSinkInput(ts.read_u32()?)),
            CommandTag::KillSourceOutput => Ok(Command::KillSourceOutput(ts.read_u32()?)),
            CommandTag::LoadModule => Ok(Command::LoadModule(ts.read()?)),
            CommandTag::UnloadModule => Ok(Command::UnloadModule(ts.read_u32()?)),
            CommandTag::GetRecordLatency => Ok(Command::GetRecordLatency(ts.read()?)),
            CommandTag::CorkRecordStream => Ok(Command::CorkRecordStream(ts.read()?)),
            CommandTag::FlushRecordStream => Ok(Command::FlushRecordStream(ts.read_u32()?)),
            CommandTag::PrebufPlaybackStream => Ok(Command::PrebufPlaybackStream(ts.read_u32()?)),
            CommandTag::MoveSinkInput => Ok(Command::MoveSinkInput(ts.read()?)),
            CommandTag::MoveSourceOutput => Ok(Command::MoveSourceOutput(ts.read()?)),
            CommandTag::SetSinkInputMute => Ok(Command::SetSinkInputMute(ts.read()?)),
            CommandTag::SuspendSink => Ok(Command::SuspendSink(ts.read()?)),
            CommandTag::SuspendSource => Ok(Command::SuspendSource(ts.read()?)),
            CommandTag::SetPlaybackStreamBufferAttr => {
                Ok(Command::SetPlaybackStreamBufferAttr(ts.read()?))
            }
            CommandTag::SetRecordStreamBufferAttr => {
                Ok(Command::SetRecordStreamBufferAttr(ts.read()?))
            }
            CommandTag::UpdatePlaybackStreamSampleRate => {
                Ok(Command::UpdatePlaybackStreamSampleRate(ts.read()?))
            }
            CommandTag::UpdateRecordStreamSampleRate => {
                Ok(Command::UpdateRecordStreamSampleRate(ts.read()?))
            }
            CommandTag::PlaybackStreamSuspended => Ok(Command::PlaybackStreamSuspended(ts.read()?)),
            CommandTag::RecordStreamSuspended => Ok(Command::RecordStreamSuspended(ts.read()?)),
            CommandTag::PlaybackStreamMoved => Ok(Command::PlaybackStreamMoved(ts.read()?)),
            CommandTag::RecordStreamMoved => Ok(Command::RecordStreamMoved(ts.read()?)),
            CommandTag::UpdateRecordStreamProplist => {
                Ok(Command::UpdateRecordStreamProplist(ts.read()?))
            }
            CommandTag::UpdatePlaybackStreamProplist => {
                Ok(Command::UpdatePlaybackStreamProplist(ts.read()?))
            }
            CommandTag::UpdateClientProplist => Ok(Command::UpdateClientProplist(ts.read()?)),
            CommandTag::RemoveRecordStreamProplist => {
                Ok(Command::RemoveRecordStreamProplist(ts.read_u32()?))
            }
            CommandTag::RemovePlaybackStreamProplist => {
                Ok(Command::RemovePlaybackStreamProplist(ts.read_u32()?))
            }
            CommandTag::RemoveClientProplist => Ok(Command::RemoveClientProplist),
            CommandTag::Extension => Ok(Command::Extension(ts.read()?)),
            CommandTag::GetCardInfo => Ok(Command::GetCardInfo(ts.read_u32()?)),
            CommandTag::GetCardInfoList => Ok(Command::GetCardInfoList),
            CommandTag::SetCardProfile => Ok(Command::SetCardProfile(ts.read()?)),
            CommandTag::ClientEvent => Ok(Command::ClientEvent(ts.read()?)),
            CommandTag::PlaybackStreamEvent => Ok(Command::PlaybackStreamEvent(ts.read()?)),
            CommandTag::RecordStreamEvent => Ok(Command::RecordStreamEvent(ts.read()?)),

            CommandTag::RecordBufferAttrChanged => Ok(Command::RecordBufferAttrChanged(ts.read()?)),

            CommandTag::SetSinkPort => Ok(Command::SetSinkPort(ts.read()?)),
            CommandTag::SetSourcePort => Ok(Command::SetSourcePort(ts.read()?)),
            CommandTag::SetSourceOutputVolume => Ok(Command::SetSourceOutputVolume(ts.read()?)),
            CommandTag::SetSourceOutputMute => Ok(Command::SetSourceOutputMute(ts.read()?)),
            CommandTag::SetPortLatencyOffset => Ok(Command::SetPortLatencyOffset(ts.read()?)),
            CommandTag::EnableSrbchannel => Err(ProtocolError::Unimplemented(seq, command)),
            CommandTag::DisableSrbchannel => Err(ProtocolError::Unimplemented(seq, command)),
            CommandTag::RegisterMemfdShmid => Err(ProtocolError::Unimplemented(seq, command)),
            CommandTag::SendObjectMessage => Err(ProtocolError::Unimplemented(seq, command)),
        }?;

        Ok((seq, cmd))
    }

    /// Write a command message as a tagstruct. In the case of a [`Command::Reply`], the payload must be written separately.
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

    /// The matching tag for this command.
    pub fn tag(&self) -> CommandTag {
        match self {
            /* Generic commands */
            Command::Error(_) => CommandTag::Error,
            Command::Timeout => CommandTag::Timeout, /* pseudo command */
            Command::Reply => CommandTag::Reply,     /* actually used for command replies */
            Command::CreatePlaybackStream(_) => CommandTag::CreatePlaybackStream, /* Payload changed in v9, v12 (0.9.0, 0.9.8) */
            Command::DeletePlaybackStream(_) => CommandTag::DeletePlaybackStream,
            Command::CreateRecordStream(_) => CommandTag::CreateRecordStream, /* Payload changed in v9, v12 (0.9.0, 0.9.8) */
            Command::DeleteRecordStream(_) => CommandTag::DeleteRecordStream,
            Command::Exit => CommandTag::Exit,
            Command::Auth(_) => CommandTag::Auth,
            Command::SetClientName(_) => CommandTag::SetClientName,
            Command::LookupSink(_) => CommandTag::LookupSink,
            Command::LookupSource(_) => CommandTag::LookupSource,
            Command::DrainPlaybackStream(_) => CommandTag::DrainPlaybackStream,
            Command::Stat => CommandTag::Stat,
            Command::GetPlaybackLatency(_) => CommandTag::GetPlaybackLatency,
            Command::CreateUploadStream(_) => CommandTag::CreateUploadStream,
            Command::DeleteUploadStream(_) => CommandTag::DeleteUploadStream,
            Command::FinishUploadStream(_) => CommandTag::FinishUploadStream,
            Command::PlaySample(_) => CommandTag::PlaySample,
            Command::RemoveSample(_) => CommandTag::RemoveSample,
            Command::GetServerInfo => CommandTag::GetServerInfo,
            Command::GetSinkInfo(_) => CommandTag::GetSinkInfo,
            Command::GetSinkInfoList => CommandTag::GetSinkInfoList,
            Command::GetSourceInfo(_) => CommandTag::GetSourceInfo,
            Command::GetSourceInfoList => CommandTag::GetSourceInfoList,
            Command::GetModuleInfo(_) => CommandTag::GetModuleInfo,
            Command::GetModuleInfoList => CommandTag::GetModuleInfoList,
            Command::GetClientInfo(_) => CommandTag::GetClientInfo,
            Command::GetClientInfoList => CommandTag::GetClientInfoList,
            Command::GetSinkInputInfo(_) => CommandTag::GetSinkInputInfo, /* Payload changed in v11 (0.9.7) */
            Command::GetSinkInputInfoList => CommandTag::GetSinkInputInfoList, /* Payload changed in v11 (0.9.7) */
            Command::GetSourceOutputInfo(_) => CommandTag::GetSourceOutputInfo,
            Command::GetSourceOutputInfoList => CommandTag::GetSourceOutputInfoList,
            Command::GetSampleInfo(_) => CommandTag::GetSampleInfo,
            Command::GetSampleInfoList => CommandTag::GetSampleInfoList,
            Command::Subscribe(_) => CommandTag::Subscribe,
            Command::SetSinkVolume(_) => CommandTag::SetSinkVolume,
            Command::SetSinkInputVolume(_) => CommandTag::SetSinkInputVolume,
            Command::SetSourceVolume(_) => CommandTag::SetSourceVolume,
            Command::SetSinkMute(_) => CommandTag::SetSinkMute,
            Command::SetSourceMute(_) => CommandTag::SetSourceMute,
            Command::CorkPlaybackStream(_) => CommandTag::CorkPlaybackStream,
            Command::FlushPlaybackStream(_) => CommandTag::FlushPlaybackStream,
            Command::TriggerPlaybackStream(_) => CommandTag::TriggerPlaybackStream,
            Command::SetDefaultSink(_) => CommandTag::SetDefaultSink,
            Command::SetDefaultSource(_) => CommandTag::SetDefaultSource,
            Command::SetPlaybackStreamName(_) => CommandTag::SetPlaybackStreamName,
            Command::SetRecordStreamName(_) => CommandTag::SetRecordStreamName,
            Command::KillClient(_) => CommandTag::KillClient,
            Command::KillSinkInput(_) => CommandTag::KillSinkInput,
            Command::KillSourceOutput(_) => CommandTag::KillSourceOutput,
            Command::LoadModule(_) => CommandTag::LoadModule,
            Command::UnloadModule(_) => CommandTag::UnloadModule,
            // Command::AddAutoloadObsolete(_) => CommandTag::AddAutoloadObsolete,
            // Command::RemoveAutoloadObsolete(_) => CommandTag::RemoveAutoloadObsolete,
            // Command::GetAutoloadInfoObsolete(_) => CommandTag::GetAutoloadInfoObsolete,
            // Command::GetAutoloadInfoListObsolete(_) => CommandTag::GetAutoloadInfoListObsolete,
            Command::GetRecordLatency(_) => CommandTag::GetRecordLatency,
            Command::CorkRecordStream(_) => CommandTag::CorkRecordStream,
            Command::FlushRecordStream(_) => CommandTag::FlushRecordStream,
            Command::PrebufPlaybackStream(_) => CommandTag::PrebufPlaybackStream,
            Command::Request(_) => CommandTag::Request,
            Command::Overflow(_) => CommandTag::Overflow,
            Command::Underflow(_) => CommandTag::Underflow,
            Command::PlaybackStreamKilled(_) => CommandTag::PlaybackStreamKilled,
            Command::RecordStreamKilled(_) => CommandTag::RecordStreamKilled,
            Command::SubscribeEvent(_) => CommandTag::SubscribeEvent,
            Command::MoveSinkInput(_) => CommandTag::MoveSinkInput,
            Command::MoveSourceOutput(_) => CommandTag::MoveSourceOutput,
            Command::SetSinkInputMute(_) => CommandTag::SetSinkInputMute,
            Command::SuspendSink(_) => CommandTag::SuspendSink,
            Command::SuspendSource(_) => CommandTag::SuspendSource,
            Command::SetPlaybackStreamBufferAttr(_) => CommandTag::SetPlaybackStreamBufferAttr,
            Command::SetRecordStreamBufferAttr(_) => CommandTag::SetRecordStreamBufferAttr,
            Command::UpdatePlaybackStreamSampleRate(_) => {
                CommandTag::UpdatePlaybackStreamSampleRate
            }
            Command::UpdateRecordStreamSampleRate(_) => CommandTag::UpdateRecordStreamSampleRate,
            Command::PlaybackStreamSuspended(_) => CommandTag::PlaybackStreamSuspended,
            Command::RecordStreamSuspended(_) => CommandTag::RecordStreamSuspended,
            Command::PlaybackStreamMoved(_) => CommandTag::PlaybackStreamMoved,
            Command::RecordStreamMoved(_) => CommandTag::RecordStreamMoved,
            Command::UpdateRecordStreamProplist(_) => CommandTag::UpdateRecordStreamProplist,
            Command::UpdatePlaybackStreamProplist(_) => CommandTag::UpdatePlaybackStreamProplist,
            Command::UpdateClientProplist(_) => CommandTag::UpdateClientProplist,
            Command::RemoveRecordStreamProplist(_) => CommandTag::RemoveRecordStreamProplist,
            Command::RemovePlaybackStreamProplist(_) => CommandTag::RemovePlaybackStreamProplist,
            Command::RemoveClientProplist => CommandTag::RemoveClientProplist,
            Command::Started(_) => CommandTag::Started,
            Command::Extension(_) => CommandTag::Extension,
            Command::GetCardInfo(_) => CommandTag::GetCardInfo,
            Command::GetCardInfoList => CommandTag::GetCardInfoList,
            Command::SetCardProfile(_) => CommandTag::SetCardProfile,
            Command::ClientEvent(_) => CommandTag::ClientEvent,
            Command::PlaybackStreamEvent(_) => CommandTag::PlaybackStreamEvent,
            Command::RecordStreamEvent(_) => CommandTag::RecordStreamEvent,
            Command::PlaybackBufferAttrChanged(_) => CommandTag::PlaybackBufferAttrChanged,
            Command::RecordBufferAttrChanged(_) => CommandTag::RecordBufferAttrChanged,
            Command::SetSinkPort(_) => CommandTag::SetSinkPort,
            Command::SetSourcePort(_) => CommandTag::SetSourcePort,
            Command::SetSourceOutputVolume(_) => CommandTag::SetSourceOutputVolume,
            Command::SetSourceOutputMute(_) => CommandTag::SetSourceOutputMute,
            Command::SetPortLatencyOffset(_) => CommandTag::SetPortLatencyOffset,
            // Command::EnableSrbchannel(_) => CommandTag::EnableSrbchannel,
            // Command::DisableSrbchannel(_) => CommandTag::DisableSrbchannel,
            // Command::RegisterMemfdShmid(_) => CommandTag::RegisterMemfdShmid,
            // Command::SendObjectMessage(_) => CommandTag::SendObjectMessage,
        }
    }
}

impl TagStructWrite for Command {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        match self {
            Command::Error(e) => w.write_u32(*e as u32),
            Command::Timeout => Ok(()),
            Command::Reply => Ok(()),
            Command::Exit => Ok(()),
            Command::Auth(p) => w.write(p),
            Command::SetClientName(p) => w.write(p),
            Command::CreatePlaybackStream(p) => w.write(p),
            Command::DeletePlaybackStream(id) => w.write_u32(*id),
            Command::CreateRecordStream(p) => w.write(p),
            Command::DeleteRecordStream(id) => w.write_u32(*id),
            Command::DrainPlaybackStream(id) => w.write_u32(*id),
            Command::GetPlaybackLatency(p) => w.write(p),
            Command::GetRecordLatency(p) => w.write(p),
            Command::CreateUploadStream(p) => w.write(p),
            Command::DeleteUploadStream(id) => w.write_u32(*id),
            Command::FinishUploadStream(id) => w.write_u32(*id),
            Command::CorkPlaybackStream(p) => w.write(p),
            Command::CorkRecordStream(p) => w.write(p),
            Command::FlushPlaybackStream(id) => w.write_u32(*id),
            Command::FlushRecordStream(id) => w.write_u32(*id),
            Command::PrebufPlaybackStream(id) => w.write_u32(*id),
            Command::TriggerPlaybackStream(id) => w.write_u32(*id),
            Command::SetPlaybackStreamName(p) => w.write(p),
            Command::SetRecordStreamName(p) => w.write(p),
            Command::SetPlaybackStreamBufferAttr(p) => w.write(p),
            Command::SetRecordStreamBufferAttr(p) => w.write(p),
            Command::UpdatePlaybackStreamProplist(p) => w.write(p),
            Command::UpdateRecordStreamProplist(p) => w.write(p),
            Command::RemovePlaybackStreamProplist(id) => w.write_u32(*id),
            Command::RemoveRecordStreamProplist(id) => w.write_u32(*id),
            Command::UpdatePlaybackStreamSampleRate(p) => w.write(p),
            Command::UpdateRecordStreamSampleRate(p) => w.write(p),
            Command::Stat => Ok(()),
            Command::GetServerInfo => Ok(()),
            Command::GetCardInfo(id) => w.write_u32(*id),
            Command::GetCardInfoList => Ok(()),
            Command::GetSinkInfo(p) => w.write(p),
            Command::GetSinkInfoList => Ok(()),
            Command::GetSourceInfo(id) => w.write(id),
            Command::GetSourceInfoList => Ok(()),
            Command::GetModuleInfo(id) => w.write_u32(*id),
            Command::GetModuleInfoList => Ok(()),
            Command::GetClientInfo(id) => w.write_u32(*id),
            Command::GetClientInfoList => Ok(()),
            Command::GetSinkInputInfo(id) => w.write_u32(*id),
            Command::GetSinkInputInfoList => Ok(()),
            Command::GetSourceOutputInfo(id) => w.write_u32(*id),
            Command::GetSourceOutputInfoList => Ok(()),
            Command::GetSampleInfo(p) => w.write_u32(*p),
            Command::GetSampleInfoList => Ok(()),
            Command::LookupSink(p) => w.write_string(Some(p)),
            Command::LookupSource(p) => w.write_string(Some(p)),
            Command::Subscribe(p) => w.write(p),
            Command::SetDefaultSink(p) => w.write_string(Some(p)),
            Command::SetDefaultSource(p) => w.write_string(Some(p)),
            Command::SetSinkPort(p) => w.write(p),
            Command::SetSourcePort(p) => w.write(p),
            Command::SetCardProfile(p) => w.write(p),
            Command::KillClient(id) => w.write_u32(*id),
            Command::KillSinkInput(id) => w.write_u32(*id),
            Command::KillSourceOutput(id) => w.write_u32(*id),
            Command::MoveSinkInput(p) => w.write(p),
            Command::MoveSourceOutput(p) => w.write(p),
            Command::SuspendSink(p) => w.write(p),
            Command::SuspendSource(p) => w.write(p),
            Command::UpdateClientProplist(p) => w.write(p),
            Command::RemoveClientProplist => Ok(()),
            Command::SetPortLatencyOffset(p) => w.write(p),
            Command::PlaySample(p) => w.write(p),
            Command::RemoveSample(p) => w.write_string(Some(p)),
            Command::LoadModule(p) => w.write(p),
            Command::UnloadModule(id) => w.write_u32(*id),
            Command::Extension(p) => w.write(p),
            Command::SetSinkVolume(p) => w.write(p),
            Command::SetSinkInputVolume(p) => w.write(p),
            Command::SetSourceVolume(p) => w.write(p),
            Command::SetSourceOutputVolume(p) => w.write(p),
            Command::SetSinkMute(p) => w.write(p),
            Command::SetSinkInputMute(p) => w.write(p),
            Command::SetSourceMute(p) => w.write(p),
            Command::SetSourceOutputMute(p) => w.write(p),
            Command::Started(id) => w.write_u32(*id),
            Command::Request(p) => w.write(p),
            Command::Overflow(id) => w.write_u32(*id),
            Command::Underflow(p) => w.write(p),
            Command::PlaybackStreamKilled(id) => w.write_u32(*id),
            Command::RecordStreamKilled(id) => w.write_u32(*id),
            Command::PlaybackStreamSuspended(p) => w.write(p),
            Command::RecordStreamSuspended(p) => w.write(p),
            Command::PlaybackStreamMoved(p) => w.write(p),
            Command::RecordStreamMoved(p) => w.write(p),
            Command::PlaybackBufferAttrChanged(p) => w.write(p),
            Command::RecordBufferAttrChanged(p) => w.write(p),
            Command::ClientEvent(p) => w.write(p),
            Command::PlaybackStreamEvent(p) => w.write(p),
            Command::RecordStreamEvent(p) => w.write(p),
            Command::SubscribeEvent(p) => w.write(p),
        }
    }
}
