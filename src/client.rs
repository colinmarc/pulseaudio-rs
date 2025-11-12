use std::{
    ffi::{CStr, CString},
    io::{BufReader, Read, Write},
};

use mio::net::UnixStream;

use super::protocol;

mod playback_source;
mod playback_stream;
mod reactor;
mod record_sink;
mod record_stream;

pub use playback_source::*;
pub use playback_stream::*;
pub use record_sink::*;
pub use record_stream::*;

/// An error encountered by a [Client].
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The PulseAudio server socket couldn't be located..
    #[error("PulseAudio server unavailable")]
    ServerUnavailable,
    /// The server sent an invalid sequence number in reply to a command.
    #[error("Unexpected sequence number")]
    UnexpectedSequenceNumber,
    /// A protocol-level error, like an invalid message.
    #[error("Protocol error")]
    Protocol(#[from] protocol::ProtocolError),
    /// An error message sent by the server in response to a command.
    #[error("Server error: {0}")]
    ServerError(protocol::PulseError),
    /// An error occurred reading or writing to the socket, or communicating
    /// with the worker thread.
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    /// The client has disconnected, usually because an error occurred.
    #[error("Client disconnected")]
    Disconnected,
}

/// The result of a [Client] operation.
pub type Result<T> = std::result::Result<T, ClientError>;

/// A PulseAudio client.
///
/// The client object can be freely cloned and shared between threads.
#[derive(Clone)]
pub struct Client {
    desc: String,
    handle: reactor::ReactorHandle,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Client").field(&self.desc).finish()
    }
}

impl Client {
    /// Creates a new client, using the environment to find the socket and cookie file.
    ///
    /// See the documentation for [socket_path_from_env](super::socket_path_from_env) and
    /// [cookie_path_from_env](super::cookie_path_from_env) for an explanation
    /// of how the socket path and cookie are determined.
    pub fn from_env(client_name: impl AsRef<CStr>) -> Result<Self> {
        let socket_path = super::socket_path_from_env().ok_or(ClientError::ServerUnavailable)?;
        let cookie = super::cookie_path_from_env().and_then(|p| std::fs::read(p).ok());

        log::info!(
            "connecting to PulseAudio server at {}",
            socket_path.display()
        );
        let socket = std::os::unix::net::UnixStream::connect(socket_path)?;
        Self::new_unix(client_name, socket, cookie)
    }

    /// Creates a new client, using the given connected unix domain socket to
    /// communicate with the PulseAudio server.
    pub fn new_unix(
        client_name: impl AsRef<CStr>,
        mut socket: std::os::unix::net::UnixStream,
        cookie: Option<impl AsRef<[u8]>>,
    ) -> std::result::Result<Self, ClientError> {
        let desc = if let Some(path) = socket.peer_addr()?.as_pathname() {
            format!("unix:{}", path.display())
        } else {
            "<unknown>".into()
        };

        // Perform the handshake.
        let protocol_version;
        {
            let mut reader = BufReader::new(&mut socket);
            let cookie = cookie.as_ref().map(AsRef::as_ref).unwrap_or(&[]).to_owned();
            let auth = protocol::AuthParams {
                version: protocol::MAX_VERSION,
                supports_shm: false,
                supports_memfd: false,
                cookie,
            };

            let auth_reply: protocol::AuthReply = roundtrip_blocking(
                &mut reader,
                protocol::Command::Auth(auth),
                0,
                protocol::MAX_VERSION,
            )?;

            protocol_version = std::cmp::min(protocol::MAX_VERSION, auth_reply.version);

            let mut props = protocol::Props::new();
            props.set(protocol::Prop::ApplicationName, client_name.as_ref());

            let _: protocol::SetClientNameReply = roundtrip_blocking(
                &mut reader,
                protocol::Command::SetClientName(props),
                1,
                protocol_version,
            )?;
        }

        // Set up the reactor.
        socket.set_nonblocking(true)?;
        let socket = UnixStream::from_std(socket);
        let handle = reactor::Reactor::spawn(socket, protocol_version)?;

        Ok(Self { desc, handle })
    }

    /// Fetches basic information on the server.
    pub async fn server_info(&self) -> Result<protocol::ServerInfo> {
        self.handle
            .roundtrip_reply(protocol::Command::GetServerInfo)
            .await
    }

    /// Fetches all clients connected to the server.
    pub async fn list_clients(&self) -> Result<Vec<protocol::ClientInfo>> {
        self.handle
            .roundtrip_reply(protocol::Command::GetClientInfoList)
            .await
    }

    /// Fetches a connected client by its index.
    pub async fn client_info(&self, index: u32) -> Result<protocol::ClientInfo> {
        self.handle
            .roundtrip_reply(protocol::Command::GetClientInfo(index))
            .await
    }

    /// Fetches all sinks available on the server.
    pub async fn list_sinks(&self) -> Result<Vec<protocol::SinkInfo>> {
        self.handle
            .roundtrip_reply(protocol::Command::GetSinkInfoList)
            .await
    }

    /// Fetches all sources available on the server.
    pub async fn list_sources(&self) -> Result<Vec<protocol::SourceInfo>> {
        self.handle
            .roundtrip_reply(protocol::Command::GetSourceInfoList)
            .await
    }

    /// Fetches a specific sink by its index.
    pub async fn sink_info(&self, index: u32) -> Result<protocol::SinkInfo> {
        self.handle
            .roundtrip_reply(protocol::Command::GetSinkInfo(protocol::GetSinkInfo {
                index: Some(index),
                name: None,
            }))
            .await
    }

    /// Fetches a specific sink by name.
    pub async fn sink_info_by_name(&self, name: CString) -> Result<protocol::SinkInfo> {
        self.handle
            .roundtrip_reply(protocol::Command::GetSinkInfo(protocol::GetSinkInfo {
                index: None,
                name: Some(name),
            }))
            .await
    }

    /// Fetches a specific source by its index.
    pub async fn source_info(&self, index: u32) -> Result<protocol::SourceInfo> {
        self.handle
            .roundtrip_reply(protocol::Command::GetSourceInfo(protocol::GetSourceInfo {
                index: Some(index),
                name: None,
            }))
            .await
    }

    /// Fetches a specific source by name.
    pub async fn source_info_by_name(&self, name: CString) -> Result<protocol::SourceInfo> {
        self.handle
            .roundtrip_reply(protocol::Command::GetSourceInfo(protocol::GetSourceInfo {
                index: None,
                name: Some(name),
            }))
            .await
    }

    /// Looks up a sink by its index.
    pub async fn lookup_sink(&self, index: u32) -> Result<u32> {
        let cmd = protocol::Command::LookupSink(CString::new(index.to_string()).unwrap());
        let reply = self
            .handle
            .roundtrip_reply::<protocol::LookupReply>(cmd)
            .await?;
        Ok(reply.0)
    }

    /// Looks up a sink by its name.
    pub async fn lookup_sink_by_name(&self, name: CString) -> Result<u32> {
        let cmd = protocol::Command::LookupSink(name);
        let reply = self
            .handle
            .roundtrip_reply::<protocol::LookupReply>(cmd)
            .await?;
        Ok(reply.0)
    }

    /// Looks up a source by its index.
    pub async fn lookup_source(&self, index: u32) -> Result<u32> {
        let cmd = protocol::Command::LookupSource(CString::new(index.to_string()).unwrap());
        let reply = self
            .handle
            .roundtrip_reply::<protocol::LookupReply>(cmd)
            .await?;
        Ok(reply.0)
    }

    /// Looks up a source by its name.
    pub async fn lookup_source_by_name(&self, name: CString) -> Result<u32> {
        let cmd = protocol::Command::LookupSource(name);
        let reply = self
            .handle
            .roundtrip_reply::<protocol::LookupReply>(cmd)
            .await?;
        Ok(reply.0)
    }

    /// Fetches a specific card by its index.
    pub async fn card_info(&self, index: u32) -> Result<protocol::CardInfo> {
        self.handle
            .roundtrip_reply(protocol::Command::GetCardInfo(protocol::GetCardInfo {
                index: Some(index),
                name: None,
            }))
            .await
    }

    /// Fetches a specific card by its name.
    pub async fn card_info_by_name(&self, name: CString) -> Result<protocol::CardInfo> {
        self.handle
            .roundtrip_reply(protocol::Command::GetCardInfo(protocol::GetCardInfo {
                index: None,
                name: Some(name),
            }))
            .await
    }

    /// Fetches all cards available on the server.
    pub async fn list_cards(&self) -> Result<Vec<protocol::CardInfo>> {
        self.handle
            .roundtrip_reply(protocol::Command::GetCardInfoList)
            .await
    }

    /// Fetches a specific module.
    pub async fn module_info(&self, index: u32) -> Result<protocol::ModuleInfo> {
        self.handle
            .roundtrip_reply(protocol::Command::GetModuleInfo(index))
            .await
    }

    /// Fetches all modules.
    pub async fn list_modules(&self) -> Result<Vec<protocol::ModuleInfo>> {
        self.handle
            .roundtrip_reply(protocol::Command::GetModuleInfoList)
            .await
    }

    /// Fetches memory usage information from the server.
    pub async fn stat(&self) -> Result<protocol::StatInfo> {
        self.handle.roundtrip_reply(protocol::Command::Stat).await
    }

    /// Fetches a specific sample.
    pub async fn sample_info(&self, index: u32) -> Result<protocol::SampleInfo> {
        self.handle
            .roundtrip_reply(protocol::Command::GetSampleInfo(index))
            .await
    }

    /// Fetches all samples available on the server.
    pub async fn list_samples(&self) -> Result<Vec<protocol::SampleInfo>> {
        self.handle
            .roundtrip_reply(protocol::Command::GetSampleInfoList)
            .await
    }

    /// Sets the default sink.
    pub async fn set_default_sink(&self, name: CString) -> Result<()> {
        self.handle
            .roundtrip_ack(protocol::Command::SetDefaultSink(name))
            .await
    }

    /// Sets the default source.
    pub async fn set_default_source(&self, name: CString) -> Result<()> {
        self.handle
            .roundtrip_ack(protocol::Command::SetDefaultSource(name))
            .await
    }

    /// Kills a client.
    pub async fn kill_client(&self, index: u32) -> Result<()> {
        self.handle
            .roundtrip_ack(protocol::Command::KillClient(index))
            .await
    }

    /// Kills a sink input.
    pub async fn kill_sink_input(&self, index: u32) -> Result<()> {
        self.handle
            .roundtrip_ack(protocol::Command::KillSinkInput(index))
            .await
    }

    /// Kills a source output.
    pub async fn kill_source_output(&self, index: u32) -> Result<()> {
        self.handle
            .roundtrip_ack(protocol::Command::KillSourceOutput(index))
            .await
    }

    /// Suspends a sink by its index.
    pub async fn suspend_sink(&self, index: u32, suspend: bool) -> Result<()> {
        self.handle
            .roundtrip_ack(protocol::Command::SuspendSink(protocol::SuspendParams {
                device_index: Some(index),
                device_name: None,
                suspend,
            }))
            .await
    }

    /// Suspends a sink by its name.
    pub async fn suspend_sink_by_name(&self, name: CString, suspend: bool) -> Result<()> {
        self.handle
            .roundtrip_ack(protocol::Command::SuspendSink(protocol::SuspendParams {
                device_index: None,
                device_name: Some(name),
                suspend,
            }))
            .await
    }

    /// Suspends a source by its index.
    pub async fn suspend_source(&self, index: u32, suspend: bool) -> Result<()> {
        self.handle
            .roundtrip_ack(protocol::Command::SuspendSource(protocol::SuspendParams {
                device_index: Some(index),
                device_name: None,
                suspend,
            }))
            .await
    }

    /// Suspends a source by its name.
    pub async fn suspend_source_by_name(&self, name: CString, suspend: bool) -> Result<()> {
        self.handle
            .roundtrip_ack(protocol::Command::SuspendSource(protocol::SuspendParams {
                device_index: None,
                device_name: Some(name),
                suspend,
            }))
            .await
    }

    /// Creates a new playback stream. The given callback will be called when the
    /// server requests data for the stream.
    pub async fn create_playback_stream(
        &self,
        params: protocol::PlaybackStreamParams,
        source: impl PlaybackSource,
    ) -> Result<PlaybackStream> {
        PlaybackStream::new(self.handle.clone(), params, source).await
    }

    /// Creates a new record stream. The returned handle implements
    /// [AsyncRead](futures::io::AsyncRead) for extracting the raw audio data.
    pub async fn create_record_stream(
        &self,
        params: protocol::RecordStreamParams,
        sink: impl RecordSink,
    ) -> Result<RecordStream> {
        RecordStream::new(self.handle.clone(), params, sink).await
    }
}

fn roundtrip_blocking<R: protocol::CommandReply>(
    socket: &mut BufReader<impl Read + Write>,
    cmd: protocol::Command,
    req_seq: u32,
    protocol_version: u16,
) -> Result<R> {
    log::debug!("CLIENT [{req_seq}]: {cmd:?}");
    protocol::write_command_message(socket.get_mut(), req_seq, &cmd, protocol_version)?;

    let (reply_seq, reply) = protocol::read_reply_message(socket, protocol_version)?;
    if req_seq != reply_seq {
        return Err(ClientError::UnexpectedSequenceNumber);
    }

    Ok(reply)
}
#[cfg(all(test, feature = "_integration-tests"))]
mod tests {
    use std::time;

    use super::*;
    use anyhow::anyhow;
    use anyhow::Context as _;
    use futures::executor::block_on;
    use rand::Rng;

    fn random_client_name() -> CString {
        CString::new(format!(
            "pulseaudio-rs-test-{}",
            rand::rng().random_range(0..10000)
        ))
        .unwrap()
    }

    #[test_log::test]
    fn server_info() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let server_info = block_on(client.server_info())?;
        assert!(server_info.server_name.is_some());

        Ok(())
    }

    #[test_log::test]
    fn list_clients() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let client_list = block_on(client.list_clients())?;
        assert!(!client_list.is_empty());

        Ok(())
    }

    #[test_log::test]
    fn client_info() -> anyhow::Result<()> {
        let client_name = random_client_name();
        let client =
            Client::from_env(client_name.clone()).context("connecting to PulseAudio server")?;

        let client_list = block_on(client.list_clients())?;
        assert!(!client_list.is_empty());

        let expected = &client_list
            .iter()
            .find(|client| client.name == client_name)
            .ok_or(anyhow!("no client with matching name"))?;
        let client_info = block_on(client.client_info(expected.index))?;

        assert_eq!(**expected, client_info);

        Ok(())
    }

    #[test_log::test]
    fn list_sinks() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let info_list = block_on(client.list_sinks())?;
        assert!(!info_list.is_empty());

        Ok(())
    }

    #[test_log::test]
    fn list_sources() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let info_list = block_on(client.list_sources())?;
        assert!(!info_list.is_empty());

        Ok(())
    }

    #[test_log::test]
    fn sink_info() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let sink_list = block_on(client.list_sinks())?;
        assert!(!sink_list.is_empty());

        let mut expected = sink_list[0].clone();
        let mut sink_info = block_on(client.sink_info(expected.index))?;

        expected.actual_latency = 0;
        sink_info.actual_latency = 0;
        assert_eq!(expected, sink_info);

        Ok(())
    }

    #[test_log::test]
    fn sink_info_by_name() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let sink_list = block_on(client.list_sinks())?;
        assert!(!sink_list.is_empty());

        let mut expected = sink_list[0].clone();
        let mut sink_info = block_on(client.sink_info_by_name(expected.name.clone()))?;

        expected.actual_latency = 0;
        sink_info.actual_latency = 0;
        assert_eq!(expected, sink_info);

        Ok(())
    }

    #[test_log::test]
    fn source_info() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let source_list = block_on(client.list_sources())?;
        assert!(!source_list.is_empty());

        let expected = &source_list[0];
        let source_info = block_on(client.source_info(expected.index))?;

        assert_eq!(expected, &source_info);

        Ok(())
    }

    #[test_log::test]
    fn source_info_by_name() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let source_list = block_on(client.list_sources())?;
        assert!(!source_list.is_empty());

        let expected = &source_list[0];
        let source_info = block_on(client.source_info_by_name(expected.name.clone()))?;

        assert_eq!(expected, &source_info);

        Ok(())
    }

    #[test_log::test]
    fn lookup_sink() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let sink_list = block_on(client.list_sinks())?;
        assert!(!sink_list.is_empty());

        let expected = &sink_list[0];
        let sink_index = block_on(client.lookup_sink(expected.index))?;

        assert_eq!(expected.index, sink_index);

        Ok(())
    }

    #[test_log::test]
    fn lookup_sink_by_name() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let sink_list = block_on(client.list_sinks())?;
        assert!(!sink_list.is_empty());

        let expected = &sink_list[0];
        let sink_index = block_on(client.lookup_sink_by_name(expected.name.clone()))?;

        assert_eq!(expected.index, sink_index);

        Ok(())
    }

    #[test_log::test]
    fn lookup_source() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let source_list = block_on(client.list_sources())?;
        assert!(!source_list.is_empty());

        let expected = &source_list[0];
        let source_index = block_on(client.lookup_source(expected.index))?;

        assert_eq!(expected.index, source_index);

        Ok(())
    }

    #[test_log::test]
    fn lookup_source_by_name() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let source_list = block_on(client.list_sources())?;
        assert!(!source_list.is_empty());

        let expected = &source_list[0];
        let source_index = block_on(client.lookup_source_by_name(expected.name.clone()))?;

        assert_eq!(expected.index, source_index);

        Ok(())
    }

    #[test_log::test]
    fn card_info() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let card_list = block_on(client.list_cards())?;

        if !card_list.is_empty() {
            let expected = &card_list[0];
            let card_info = block_on(client.card_info(expected.index))?;

            assert_eq!(expected, &card_info);
        }

        Ok(())
    }

    #[test_log::test]
    fn card_info_by_name() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let card_list = block_on(client.list_cards())?;

        if !card_list.is_empty() {
            let expected = &card_list[0];
            let card_info = block_on(client.card_info_by_name(expected.name.clone()))?;

            assert_eq!(expected, &card_info);
        }

        Ok(())
    }

    #[test_log::test]
    fn list_cards() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let _card_list = block_on(client.list_cards())?;
        Ok(())
    }

    #[test_log::test]
    fn module_info() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let module_list = block_on(client.list_modules())?;
        assert!(!module_list.is_empty());

        let expected = &module_list[0];
        let module_info = block_on(client.module_info(expected.index))?;

        assert_eq!(expected, &module_info);

        Ok(())
    }

    #[test_log::test]
    fn list_modules() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let module_list = block_on(client.list_modules())?;
        assert!(!module_list.is_empty());

        Ok(())
    }

    #[test_log::test]
    fn stat() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let stat_info = block_on(client.stat())?;
        assert!(stat_info.memblock_total > 0);

        Ok(())
    }

    #[test_log::test]
    fn sample_info() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let sample_list = block_on(client.list_samples())?;
        if sample_list.is_empty() {
            return Ok(());
        }

        let expected = &sample_list[0];
        let sample_info = block_on(client.sample_info(expected.index))?;

        assert_eq!(expected, &sample_info);

        Ok(())
    }

    #[test_log::test]
    fn list_samples() -> anyhow::Result<()> {
        let client =
            Client::from_env(random_client_name()).context("connecting to PulseAudio server")?;

        let _sample_list = block_on(client.list_samples())?;
        Ok(())
    }

    #[test_log::test]
    fn kill_client() -> anyhow::Result<()> {
        let client_name1 = random_client_name();
        let client1 = Client::from_env(client_name1.clone())?;
        let client2 = Client::from_env(random_client_name())?;

        let client_list = block_on(client2.list_clients())?;
        assert!(!client_list.is_empty());

        let client1_info = client_list
            .iter()
            .find(|client| client.name == client_name1)
            .ok_or(anyhow!("no client1 with matching name"))?;

        block_on(client2.kill_client(client1_info.index))?;

        // Listing things should eventually fail with client1.
        let start = time::Instant::now();
        loop {
            match block_on(client1.server_info()).err() {
                Some(ClientError::Disconnected) => break,
                _ if start.elapsed() < time::Duration::from_secs(1) => {
                    std::thread::sleep(time::Duration::from_millis(10))
                }
                _ => panic!("client still connected"),
            }
        }

        let client_list = block_on(client2.list_clients())?;
        assert!(client_list
            .iter()
            .find(|client| client.name == client1_info.name)
            .is_none());

        Ok(())
    }
}
