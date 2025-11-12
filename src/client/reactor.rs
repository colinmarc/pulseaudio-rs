use std::{
    collections::BTreeMap,
    io::{self},
    pin::Pin,
    sync::{
        atomic::{self, AtomicU32},
        mpsc::{Receiver, Sender, TryRecvError},
        Arc, Mutex, Weak,
    },
    task::{Context, Poll},
    thread::JoinHandle,
};

use futures::channel::oneshot;
use mio::net::UnixStream;

use crate::protocol::{self, DescriptorFlags};

use super::{ClientError, PlaybackSource, RecordSink};

type ReplyResult<'a> =
    Result<(&'a mut ReactorState, &'a mut dyn io::BufRead), protocol::PulseError>;
type ReplyHandler = Box<dyn FnOnce(ReplyResult<'_>) + Send + 'static>;

struct PlaybackStreamState {
    stream_info: protocol::CreatePlaybackStreamReply,
    source: Pin<Box<dyn PlaybackSource>>,

    requested_bytes: usize,
    done: bool,
    eof_notify: Option<oneshot::Sender<()>>,
}

pub(super) struct RecordStreamState {
    sink: Box<dyn RecordSink>,
    start_notify: Option<oneshot::Sender<()>>,
}

#[derive(Default)]
struct ReactorState {
    handlers: BTreeMap<u32, ReplyHandler>,
    playback_streams: BTreeMap<u32, PlaybackStreamState>,
    record_streams: BTreeMap<u32, RecordStreamState>,
}

struct SharedState {
    protocol_version: u16,
    next_seq: AtomicU32,
    _thread_handle: JoinHandle<super::Result<()>>,
}

// We need to wrap this to implement futures::task::ArcWake.
struct Waker(mio::Waker);

impl futures::task::ArcWake for Waker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let _ = arc_self.0.wake();
    }
}

#[derive(Clone)]
pub(super) struct ReactorHandle {
    state: Weak<Mutex<ReactorState>>,
    shared: Arc<SharedState>,
    outgoing: Sender<(u32, protocol::Command)>,
    waker: Arc<Waker>,
}

impl ReactorHandle {
    pub(super) async fn roundtrip_reply<R: protocol::CommandReply + Send + 'static>(
        &self,
        cmd: protocol::Command,
    ) -> Result<R, ClientError> {
        let seq = self.next_seq();

        // Install a handler for the sequence number.
        let (tx, rx) = oneshot::channel();
        let protocol_version = self.shared.protocol_version;
        self.install_handler(seq, move |res: ReplyResult<'_>| {
            let _ = match res {
                Ok((_, buf)) => tx.send(read_tagstruct(buf, protocol_version)),
                Err(err) => tx.send(Err(ClientError::ServerError(err))),
            };
        })?;

        // Send the message.
        self.write_command(seq, cmd)?;

        // Wait for the response.
        rx.await.map_err(|_| ClientError::Disconnected)?
    }

    pub(super) async fn roundtrip_ack(&self, cmd: protocol::Command) -> Result<(), ClientError> {
        let seq = self.next_seq();

        // Install a handler for the sequence number.
        let (tx, rx) = oneshot::channel();
        self.install_handler(seq, move |res: ReplyResult<'_>| {
            let _ = match res {
                Ok(_) => tx.send(Ok(())),
                Err(err) => tx.send(Err(ClientError::ServerError(err))),
            };
        })?;

        // Send the message.
        self.write_command(seq, cmd)?;

        // Wait for the response.
        rx.await.map_err(|_| ClientError::Disconnected)?
    }

    pub(super) async fn insert_playback_stream(
        &self,
        params: protocol::PlaybackStreamParams,
        source: impl PlaybackSource,
        eof_notify: Option<oneshot::Sender<()>>,
    ) -> Result<protocol::CreatePlaybackStreamReply, ClientError> {
        // This is the seq for the CreatePlaybackStream command.
        let seq = self.next_seq();

        let protocol_version = self.shared.protocol_version;
        let handler = move |res: ReplyResult<'_>| {
            let (state, buf) = res.map_err(ClientError::ServerError)?;
            let stream_info: protocol::CreatePlaybackStreamReply =
                read_tagstruct(buf, protocol_version)?;

            let requested_bytes = stream_info.requested_bytes as usize;
            state.playback_streams.insert(
                stream_info.channel,
                PlaybackStreamState {
                    stream_info: stream_info.clone(),
                    source: Box::pin(source),

                    requested_bytes,
                    done: false,
                    eof_notify,
                },
            );

            Ok(stream_info)
        };

        let (tx, rx) = oneshot::channel();
        self.install_handler(seq, move |res: ReplyResult<'_>| {
            let _ = tx.send(handler(res));
        })?;

        // Send the message.
        self.write_command(seq, protocol::Command::CreatePlaybackStream(params))?;

        // Wait for the response.
        rx.await.map_err(|_| ClientError::Disconnected)?
    }

    pub(super) async fn delete_playback_stream(&self, channel: u32) -> Result<(), ClientError> {
        let seq = self.next_seq();

        let (tx, rx) = oneshot::channel();
        self.install_handler(seq, move |res| {
            if let Ok((state, _ack)) = res {
                state.playback_streams.remove(&channel);
            }

            let _ = tx.send(());
        })?;

        self.write_command(seq, protocol::Command::DeletePlaybackStream(channel))?;
        rx.await.map_err(|_| ClientError::Disconnected)
    }

    pub(super) fn mark_playback_stream_draining(&self, channel: u32) {
        if let Some(state) = self.state.upgrade() {
            if let Some(stream) = state.lock().unwrap().playback_streams.get_mut(&channel) {
                stream.done = true;
            }
        }
    }

    pub(super) async fn insert_record_stream(
        &self,
        params: protocol::RecordStreamParams,
        sink: impl RecordSink,
        start_notify: Option<oneshot::Sender<()>>,
    ) -> Result<protocol::CreateRecordStreamReply, ClientError> {
        let seq = self.next_seq();

        let protocol_version = self.shared.protocol_version;
        let handler = move |res: ReplyResult<'_>| {
            let (state, buf) = res.map_err(ClientError::ServerError)?;
            let stream_info: protocol::CreateRecordStreamReply =
                read_tagstruct(buf, protocol_version)?;

            state.record_streams.insert(
                stream_info.channel,
                RecordStreamState {
                    sink: Box::new(sink),
                    start_notify,
                },
            );

            Ok(stream_info)
        };

        let (tx, rx) = oneshot::channel();
        self.install_handler(seq, move |res: ReplyResult<'_>| {
            let _ = tx.send(handler(res));
        })?;

        // Send the message.
        self.write_command(seq, protocol::Command::CreateRecordStream(params))?;

        // Wait for the response.
        rx.await.map_err(|_| ClientError::Disconnected)?
    }

    pub(super) async fn delete_record_stream(&self, channel: u32) -> Result<(), ClientError> {
        let seq = self.next_seq();

        let (tx, rx) = oneshot::channel();
        self.install_handler(seq, move |res| {
            if let Ok((state, _ack)) = res {
                state.record_streams.remove(&channel);
            }

            let _ = tx.send(());
        })?;

        self.write_command(seq, protocol::Command::DeleteRecordStream(channel))?;
        rx.await.map_err(|_| ClientError::Disconnected)
    }

    fn write_command(&self, seq: u32, cmd: protocol::Command) -> Result<(), ClientError> {
        self.outgoing
            .send((seq, cmd))
            .map_err(|_| ClientError::Disconnected)?;
        self.waker.0.wake()?;

        Ok(())
    }

    fn install_handler<F>(&self, seq: u32, handler: F) -> Result<(), ClientError>
    where
        F: FnOnce(ReplyResult<'_>) + Send + 'static,
    {
        self.state
            .upgrade()
            .ok_or(ClientError::Disconnected)?
            .lock()
            .unwrap()
            .handlers
            .insert(seq, Box::new(handler));

        Ok(())
    }

    fn next_seq(&self) -> u32 {
        self.shared.next_seq.fetch_add(1, atomic::Ordering::Relaxed)
    }
}

pub(super) const WAKER: mio::Token = mio::Token(0);
pub(super) const SOCKET: mio::Token = mio::Token(1);

pub(super) struct Reactor {
    socket: UnixStream,
    poll: mio::Poll,
    waker: Arc<Waker>,
    state: Arc<Mutex<ReactorState>>,
    outgoing: Receiver<(u32, protocol::Command)>,
    protocol_version: u16,

    write_buf: Vec<u8>,
    read_buf: Vec<u8>,
    in_progress_read: Option<protocol::Descriptor>,
}

impl Reactor {
    pub(super) fn spawn(
        mut socket: UnixStream,
        protocol_version: u16,
    ) -> Result<ReactorHandle, ClientError> {
        let poll = mio::Poll::new()?;
        let waker = Arc::new(Waker(mio::Waker::new(poll.registry(), WAKER)?));
        poll.registry().register(
            &mut socket,
            SOCKET,
            mio::Interest::READABLE | mio::Interest::WRITABLE,
        )?;

        let state = Arc::new(Mutex::new(ReactorState::default()));

        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
        let mut reactor = Self {
            socket,
            poll,
            waker: waker.clone(),
            state: state.clone(),
            outgoing: cmd_rx,
            protocol_version,

            write_buf: Vec::new(),
            read_buf: Vec::new(),
            in_progress_read: None,
        };

        let reactor_thread = std::thread::spawn(move || match reactor.run() {
            Ok(_) => Ok(()),
            Err(err) => {
                log::error!("Reactor error: {err}");
                Err(err)
            }
        });

        Ok(ReactorHandle {
            state: Arc::downgrade(&state),
            outgoing: cmd_tx,
            waker,
            shared: Arc::new(SharedState {
                protocol_version,
                next_seq: AtomicU32::new(1024),
                _thread_handle: reactor_thread,
            }),
        })
    }

    pub(super) fn run(&mut self) -> Result<(), ClientError> {
        let mut events = mio::Events::with_capacity(1024);

        loop {
            self.poll.poll(&mut events, None)?;
            self.recv()?;

            // Handle any requested writes.
            self.write_streams()?;
            self.write_commands()?;
        }
    }

    fn recv(&mut self) -> Result<(), ClientError> {
        use io::Read;

        'read: loop {
            let off = self.read_buf.len();
            self.read_buf.resize(off + 1024 * 1024, 0);

            match self.socket.read(&mut self.read_buf[off..]) {
                Ok(0) => return Err(ClientError::Disconnected),
                Ok(n) => self.read_buf.truncate(off + n),
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.read_buf.truncate(off);
                    return Ok(());
                }
                Err(err) => return Err(err.into()),
            };

            // Decode messages (there may be multiple).
            while !self.read_buf.is_empty() {
                // Continue the previous read, if it was unfinished.
                let desc = if let Some(desc) = self.in_progress_read.take() {
                    desc
                } else if self.read_buf.len() >= protocol::DESCRIPTOR_SIZE {
                    protocol::read_descriptor(&mut io::Cursor::new(&self.read_buf))?
                } else {
                    log::trace!("very short read ({} bytes)", self.read_buf.len());
                    continue 'read;
                };

                // If we don't have all the message, poll until we do.
                let len = desc.length as usize + protocol::DESCRIPTOR_SIZE;
                if self.read_buf.len() < len {
                    self.in_progress_read = Some(desc);
                    log::trace!("partial read ({}/{} bytes)", self.read_buf.len(), len);
                    continue 'read;
                }

                if desc.channel == u32::MAX {
                    self.handle_command(len);
                } else {
                    // Stream data for a record stream.
                    let mut guard = self.state.lock().unwrap();
                    if let Some(RecordStreamState { sink, start_notify }) =
                        guard.record_streams.get_mut(&desc.channel)
                    {
                        log::trace!("reading {len} bytes from stream {}", desc.channel,);
                        if let Some(start_notify) = start_notify.take() {
                            let _ = start_notify.send(());
                        }

                        sink.write(&self.read_buf[protocol::DESCRIPTOR_SIZE..len])
                    } else {
                        log::warn!("Received data for unknown record stream {}", desc.channel);
                    }
                }

                self.read_buf.drain(..len);
            }
        }
    }

    fn handle_command(&mut self, len: usize) {
        let mut cursor = io::Cursor::new(&self.read_buf[protocol::DESCRIPTOR_SIZE..len]);
        let (seq, cmd) =
            match protocol::Command::read_tag_prefixed(&mut cursor, self.protocol_version) {
                Ok((seq, cmd)) => (seq, cmd),
                Err(err) => {
                    log::error!("failed to read command message: {err}");
                    return;
                }
            };

        let mut state = self.state.lock().unwrap();

        log::debug!("SERVER [{}]: {cmd:?}", seq as i32);
        if matches!(cmd, protocol::Command::Reply | protocol::Command::Error(_)) {
            let Some(handler) = state.handlers.remove(&seq) else {
                log::warn!("no reply handler found for sequence {seq}");
                return;
            };

            match cmd {
                protocol::Command::Reply => handler(Ok((&mut state, &mut cursor))),
                protocol::Command::Error(err) => handler(Err(err)),
                _ => unreachable!(),
            }
            return;
        }

        match cmd {
            protocol::Command::Started(channel) => {
                if state.playback_streams.contains_key(&channel) {
                    log::debug!("stream started: {channel}");
                } else {
                    log::error!("unknown stream: {channel}");
                }
            }
            protocol::Command::Request(protocol::Request { channel, length }) => {
                if let Some(stream) = state.playback_streams.get_mut(&channel) {
                    stream.requested_bytes += length as usize;
                } else {
                    log::error!("unknown stream: {channel}");
                }
            }
            _ => log::debug!("ignoring unexpected command: {cmd:?}"),
        }
    }

    fn write_commands(&mut self) -> Result<(), ClientError> {
        loop {
            // Drain the write buffer...
            if !drain_buf(&mut self.write_buf, &mut self.socket)? {
                return Ok(());
            }

            // ...and encode new command messages into it.
            match self.outgoing.try_recv() {
                Ok((seq, cmd)) => {
                    log::debug!("CLIENT [{seq}]: {cmd:?}");
                    protocol::encode_command_message(
                        &mut self.write_buf,
                        seq,
                        &cmd,
                        self.protocol_version,
                    )?;
                }
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => return Err(ClientError::Disconnected),
            };
        }
    }

    fn write_streams(&mut self) -> Result<(), ClientError> {
        if !drain_buf(&mut self.write_buf, &mut self.socket)? {
            return Ok(());
        }

        let mut state = self.state.lock().unwrap();
        for stream in state.playback_streams.values_mut() {
            if stream.done {
                continue;
            }

            while stream.requested_bytes > 0 {
                let requested = stream.requested_bytes;

                self.write_buf
                    .resize(protocol::DESCRIPTOR_SIZE + requested, 0);

                let waker = futures::task::waker(self.waker.clone());
                let mut cx = Context::from_waker(&waker);
                let buf = &mut self.write_buf[protocol::DESCRIPTOR_SIZE..];
                let len = match PlaybackSource::poll_read(stream.source.as_mut(), &mut cx, buf) {
                    Poll::Ready(0) => {
                        log::debug!(
                            "source for stream {} reached EOF",
                            stream.stream_info.channel
                        );

                        stream.done = true;
                        stream.eof_notify.take().map(|done| done.send(()));
                        self.write_buf.clear();
                        break;
                    }
                    Poll::Pending => {
                        self.write_buf.clear();
                        break;
                    }
                    Poll::Ready(n) => n,
                };

                let len = len.min(requested);
                if len == 0 {
                    log::debug!(
                        "callback for stream {} returned no data",
                        stream.stream_info.channel
                    );

                    self.write_buf.clear();
                    break;
                }

                log::trace!(
                    "writing {len} bytes to stream {} (requested {})",
                    stream.stream_info.channel,
                    stream.requested_bytes
                );

                self.write_buf.truncate(protocol::DESCRIPTOR_SIZE + len);
                stream.requested_bytes -= len;

                let desc = protocol::Descriptor {
                    length: len as u32,
                    channel: stream.stream_info.channel,
                    offset: 0,
                    flags: DescriptorFlags::empty(),
                };

                protocol::encode_descriptor(
                    (&mut self.write_buf[..protocol::DESCRIPTOR_SIZE])
                        .try_into()
                        .unwrap(),
                    &desc,
                );

                if !drain_buf(&mut self.write_buf, &mut self.socket)? {
                    return Ok(());
                }
            }
        }

        Ok(())
    }
}

fn drain_buf(buf: &mut Vec<u8>, w: &mut impl io::Write) -> Result<bool, io::Error> {
    while !buf.is_empty() {
        match w.write(buf) {
            Ok(0) => return Ok(false),
            Ok(n) => buf.drain(..n),
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => return Ok(false),
            Err(err) => return Err(err),
        };
    }

    Ok(true)
}

fn read_tagstruct<R: protocol::CommandReply>(
    buf: &mut dyn io::BufRead,
    protocol_version: u16,
) -> Result<R, ClientError> {
    protocol::TagStructReader::new(buf, protocol_version)
        .read()
        .map_err(Into::into)
}
