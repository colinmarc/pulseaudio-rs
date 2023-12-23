use std::io::{BufRead, Write};

use enum_primitive_derive::Primitive;

mod auth;
// mod create_playback_stream;
mod get_info;
mod register_memfd_shmid;
mod set_client_name;

pub use auth::{Auth, AuthReply};
pub use get_info::SinkInfoList;
// pub use create_playback_stream::CreatePlaybackStream;
pub use register_memfd_shmid::RegisterMemfdShmid;
pub use set_client_name::{SetClientName, SetClientNameReply};

use super::{serde::*, ProtocolError, PulseError};

use num_traits::FromPrimitive as _;

#[repr(u8)]
#[derive(Debug, Copy, Clone, Primitive)]
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
    //PA_COMMAND_MAX,
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
    /// Authentication request (and protocol handshake).
    Auth(auth::Auth),

    /// Updates client properties (not just the name).
    SetClientName(SetClientName),

    /// Create a new playback stream.
    // CreatePlaybackStream(CreatePlaybackStream<'a>),

    // TODO: Payload for forwards-compatibility
    GetSinkInfoList,
    GetSourceInfoList,
    GetClientInfoList,
    GetCardInfoList,
    GetModuleInfoList,
    GetSinkInputInfoList,
    GetSourceOutputInfoList,
    GetSampleInfoList,
    // / Register `memfd`-based shared memory.
    // /
    // / This command can be sent from client to server and from server to
    // / client. It can only be sent over a Unix domain socket and *must* be
    // / accompanied by the `memfd` file descriptor to share (see [`unix(7)`]
    // / and the `SCM_RIGHTS` ancillary message).
    // /
    // / [`unix(7)`]: https://linux.die.net/man/7/unix
    // TODO: Better docs
    // RegisterMemfdShmid(RegisterMemfdShmid),
}

impl Command {
    pub fn read_tag_prefixed<R: BufRead>(
        r: &mut R,
        protocol_version: u16,
    ) -> Result<(u32, Self), ProtocolError> {
        let mut ts = TagStructReader::new(r, protocol_version);
        let (command, seq) = (ts.read_enum()?, ts.read_u32()?);

        let cmd = match command {
            CommandTag::Auth => Ok(Command::Auth(ts.read()?)),
            CommandTag::SetClientName => Ok(Command::SetClientName(ts.read()?)),
            // CommandTag::CreatePlaybackStream => Ok(Command::CreatePlaybackStream(TagStructRead::read(&mut crate::protocol::tagstruct::TagStructReader::new(r), 0)?)),
            CommandTag::GetSinkInfoList => Ok(Command::GetSinkInfoList),
            CommandTag::GetSourceInfoList => Ok(Command::GetSourceInfoList),
            CommandTag::GetClientInfoList => Ok(Command::GetClientInfoList),
            CommandTag::GetCardInfoList => Ok(Command::GetCardInfoList),
            CommandTag::GetModuleInfoList => Ok(Command::GetModuleInfoList),
            CommandTag::GetSinkInputInfoList => Ok(Command::GetSinkInputInfoList),
            CommandTag::GetSourceOutputInfoList => Ok(Command::GetSourceOutputInfoList),
            CommandTag::GetSampleInfoList => Ok(Command::GetSampleInfoList),
            _ => Err(crate::protocol::ProtocolError::Unimplemented(command)),
        }?;

        Ok((seq, cmd))
    }

    pub fn write_tag_prefixed<W: Write>(&self, seq: u32, w: &mut W) -> Result<(), ProtocolError> {
        let mut ts = TagStructWriter::new(w, 0);

        ts.write_u32(self.tag() as u32)?;
        ts.write_u32(seq)?;
        ts.write(self)?;

        Ok(())
    }

    pub fn tag(&self) -> CommandTag {
        match self {
            Command::Auth(_) => CommandTag::Auth,
            Command::SetClientName(_) => CommandTag::SetClientName,
            // Command::CreatePlaybackStream(_) => CommandTag::CreatePlaybackStream,
            Command::GetSinkInfoList => CommandTag::GetSinkInfoList,
            Command::GetSourceInfoList => CommandTag::GetSourceInfoList,
            Command::GetClientInfoList => CommandTag::GetClientInfoList,
            Command::GetCardInfoList => CommandTag::GetCardInfoList,
            Command::GetModuleInfoList => CommandTag::GetModuleInfoList,
            Command::GetSinkInputInfoList => CommandTag::GetSinkInputInfoList,
            Command::GetSourceOutputInfoList => CommandTag::GetSourceOutputInfoList,
            Command::GetSampleInfoList => CommandTag::GetSampleInfoList,
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
            Command::Auth(ref p) => w.write(p),
            Command::SetClientName(ref p) => w.write(p),
            Command::GetSinkInfoList
            | Command::GetSourceInfoList
            | Command::GetClientInfoList
            | Command::GetCardInfoList
            | Command::GetModuleInfoList
            | Command::GetSinkInputInfoList
            | Command::GetSourceOutputInfoList
            | Command::GetSampleInfoList => Ok(()),
        }
    }
}
