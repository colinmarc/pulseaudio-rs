//! A simple pulseaudio server that accepts playback streams and drops any
//! samples sent to it.
//!
//! To start the server, run:
//!     cargo run --example server -- /tmp/pulseaudio.sock
//!
//! Then you can (for example) list sinks with:
//!    PULSE_SERVER="unix:/tmp/pulseaudio.sock" pactl list sinks

use std::collections::{HashMap, VecDeque};
use std::io::{Cursor, Read, Write};
use std::time;

use anyhow::bail;
use mio::net::UnixListener;
use mio::{Events, Poll, Token};
use pulseaudio::protocol::{self, write_error};

const CLOCK_SPEED_HZ: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    // The number of bytes remaining before we can start 'playback'.
    Prebuffering(u64),
    Playing,
    // The seq of the drain request, so we can ack it.
    Draining(u32),
}

#[derive(Debug)]
struct PlaybackStream {
    state: StreamState,
    sample_spec: protocol::SampleSpec,
    buffer_attr: protocol::stream::BufferAttr,
    buffer: VecDeque<u8>,
    requested_bytes: usize,
}

#[derive(Debug)]
struct Client {
    id: u32,
    socket: mio::net::UnixStream,
    playback_streams: HashMap<u32, PlaybackStream>,
    authenticated: bool,
    protocol_version: u16,
    props: Option<protocol::Props>,
}

struct ServerState {
    sinks: Vec<protocol::SinkInfo>,
    next_playback_channel_index: u32,
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <socket>", args[0]);
        return Ok(());
    }

    // Create a new event loop that accepts UDP connections.
    let mut poll = Poll::new().unwrap();

    // These are for the whole server.
    const ACCEPT: Token = Token(0);
    const CLOCK: Token = Token(1);

    // Client tokens start at 1024.
    let mut next_client_token = 1024;

    // This is our listening socket.
    let mut listener = UnixListener::bind(&args[1])?;
    poll.registry()
        .register(&mut listener, ACCEPT, mio::Interest::READABLE)?;

    // This is a timer for reading audio data. The slower we run, the more data
    // we would have to buffer (if we weren't throwing it away).
    let mut clock = mio_timerfd::TimerFd::new(mio_timerfd::ClockId::Monotonic)?;
    clock.set_timeout_interval(&time::Duration::from_nanos(1_000_000_000 / CLOCK_SPEED_HZ))?;
    poll.registry()
        .register(&mut clock, CLOCK, mio::Interest::READABLE)?;

    let mut events = Events::with_capacity(256);

    // Keep track of our client connections.
    let mut clients: HashMap<Token, Client> = HashMap::new();

    // Global server state.
    let mut server = ServerState {
        sinks: vec![protocol::SinkInfo::new_dummy(1)],
        next_playback_channel_index: 0,
    };

    // A reusable buffer for reading incoming messages.
    let mut scratch = Vec::new();

    loop {
        poll.poll(&mut events, None).unwrap();

        for event in &events {
            match event.token() {
                ACCEPT => {
                    // Accept a new connection, set a token for it, and store
                    // the socket in the state.
                    let (mut socket, _) = listener.accept()?;

                    let index = next_client_token as u32;
                    let token = Token(next_client_token);
                    next_client_token += 1;
                    poll.registry()
                        .register(&mut socket, token, mio::Interest::READABLE)?;

                    eprintln!("new client connected!");

                    clients.insert(
                        token,
                        Client {
                            id: index,
                            socket,
                            playback_streams: HashMap::new(),
                            authenticated: false,
                            protocol_version: 0,
                            props: None,
                        },
                    );
                }
                CLOCK => {
                    clock.read()?;

                    for client in clients.values_mut() {
                        let mut done_draining = Vec::new();
                        for (id, stream) in client.playback_streams.iter_mut() {
                            // This removes samples to play (drop) them.
                            play_samples(stream)?;

                            // If we've drained the buffer, we can drop the stream.
                            if matches!(stream.state, StreamState::Draining(_))
                                && stream.buffer.is_empty()
                            {
                                done_draining.push(*id)
                            }

                            let bytes_needed = (stream.buffer_attr.target_length as usize)
                                .saturating_sub(stream.buffer.len() + stream.requested_bytes);
                            if stream.state == StreamState::Playing
                                && bytes_needed > stream.buffer_attr.minimum_request_length as usize
                            {
                                // We should request more bytes to fill the buffer.
                                eprintln!("requesting stream write for {} bytes", bytes_needed);
                                stream.requested_bytes += bytes_needed;
                                protocol::write_command_message(
                                    &mut client.socket,
                                    u32::MAX,
                                    protocol::Command::Request(protocol::Request {
                                        channel: *id,
                                        length: bytes_needed as u32,
                                    }),
                                    client.protocol_version,
                                )?;
                            }
                        }

                        for id in done_draining {
                            let stream = client.playback_streams.remove(&id).unwrap();
                            eprintln!("channel {} finished playback!", id);

                            if let StreamState::Draining(seq) = stream.state {
                                protocol::write_ack_message(&mut client.socket, seq)?;
                            } else {
                                unreachable!()
                            }
                        }
                    }
                }
                client_token if event.is_read_closed() => {
                    // The client disconnected.
                    if let Some(mut client) = clients.remove(&client_token) {
                        eprintln!("client disconnected!");
                        poll.registry().deregister(&mut client.socket)?;
                    }
                }
                client_token if event.is_readable() => {
                    let client = match clients.get_mut(&client_token) {
                        Some(v) => v,
                        None => continue,
                    };

                    'read: loop {
                        // Read the message header.
                        let desc = match protocol::read_descriptor(&mut client.socket) {
                            Ok(v) => v,
                            Err(protocol::ProtocolError::Io(e))
                                if e.kind() == std::io::ErrorKind::WouldBlock =>
                            {
                                break 'read;
                            }
                            Err(e) => bail!("reading descriptor: {:?}", e),
                        };

                        // Read the payload.
                        scratch.resize(desc.length as usize, 0);
                        let payload = &mut scratch[..desc.length as usize];
                        client.socket.read_exact(payload)?;

                        // A channel of -1 means a command message. Everything
                        // else is stream data.
                        if desc.channel == u32::MAX {
                            let (seq, cmd) = match protocol::Command::read_tag_prefixed(
                                &mut Cursor::new(payload),
                                client.protocol_version,
                            ) {
                                Ok(v) => v,
                                Err(e) => {
                                    eprintln!("error reading command: {:?}", e);
                                    write_error(
                                        &mut client.socket,
                                        u32::MAX,
                                        protocol::PulseError::Protocol,
                                    )?;

                                    break 'read;
                                }
                            };

                            handle_command(&mut server, client, seq, cmd)?;
                        } else {
                            handle_stream_write(client, desc, payload)?;
                        }
                    }
                }
                _ => (),
            }
        }
    }
}

fn handle_command(
    server: &mut ServerState,
    client: &mut Client,
    seq: u32,
    cmd: protocol::Command,
) -> anyhow::Result<()> {
    eprintln!("got command {:?}", cmd.tag());

    match cmd {
        // The client has to auth first.
        _ if !client.authenticated => {
            if let protocol::Command::Auth(params) = cmd {
                // Here is where we would check the client's
                // auth cookie. We won't do that in this
                // example, though.
                client.authenticated = true;
                client.protocol_version = std::cmp::min(params.version, protocol::MAX_VERSION);

                protocol::write_reply_message(
                    &mut client.socket,
                    seq,
                    &protocol::AuthReply {
                        version: client.protocol_version,
                        ..Default::default()
                    },
                    client.protocol_version,
                )?;
            } else {
                // The client needs to authenticate.
                protocol::write_error(&mut client.socket, seq, protocol::PulseError::AccessDenied)?;
            }
        }
        // Then it has to set props.
        _ if client.props.is_none() => {
            if let protocol::Command::SetClientName(props) = cmd {
                client.props = Some(props);

                protocol::write_reply_message(
                    &mut client.socket,
                    seq,
                    &protocol::SetClientNameReply {
                        client_id: client.id,
                    },
                    client.protocol_version,
                )?;
            } else {
                // PulseAudio requires this as part of the
                // handshake, so we will too.
                protocol::write_error(&mut client.socket, seq, protocol::PulseError::Protocol)?;
            }
        }
        // We won't implement all the introspection
        // commands, but here's an example.
        protocol::Command::GetSinkInfoList => {
            protocol::write_reply_message(
                &mut client.socket,
                seq,
                &server.sinks,
                client.protocol_version,
            )?;
        }
        // Here we'll handle playback streams.
        protocol::Command::CreatePlaybackStream(mut params) => {
            // Check if the client set any buffer attrs
            // to -1, which indicates that we should
            // set the value.
            apply_buffer_defaults(&mut params.buffer_attr);

            let stream = PlaybackStream {
                state: StreamState::Prebuffering(params.buffer_attr.pre_buffering as u64),
                sample_spec: params.sample_spec,
                buffer_attr: params.buffer_attr,
                buffer: VecDeque::new(),
                requested_bytes: 0,
            };

            let channel = server.next_playback_channel_index;
            server.next_playback_channel_index += 1;

            client.playback_streams.insert(channel, stream);

            // When writing the reply, we can
            // immediately request bytes to be written.
            let reply = protocol::CreatePlaybackStreamReply {
                channel,
                stream_index: 0,
                sample_spec: params.sample_spec,
                channel_map: params.channel_map,
                buffer_attr: params.buffer_attr,
                requested_bytes: params.buffer_attr.pre_buffering,
                sink_name: Some(server.sinks[0].name.clone()),
                ..Default::default()
            };

            eprintln!("created playback stream {:#?}", reply);

            protocol::write_reply_message(
                &mut client.socket,
                seq,
                &reply,
                client.protocol_version,
            )?;
        }
        protocol::Command::DrainPlaybackStream(channel) => {
            if let Some(stream) = client.playback_streams.get_mut(&channel) {
                stream.state = StreamState::Draining(seq);
            }
        }
        _ => {
            eprintln!("ignoring command {:?}", cmd.tag());
            protocol::write_error(
                &mut client.socket,
                seq,
                protocol::PulseError::NotImplemented,
            )?;
        }
    }

    Ok(())
}

fn handle_stream_write(
    client: &mut Client,
    desc: protocol::Descriptor,
    payload: &[u8],
) -> anyhow::Result<()> {
    eprintln!(
        "got stream write to channel {} ({} bytes)",
        desc.channel,
        payload.len()
    );

    let stream = match client.playback_streams.get_mut(&desc.channel) {
        Some(v) => v,
        None => {
            bail!("invalid channel")
        }
    };

    // We don't handle seeks in this example.
    assert_eq!(0, desc.offset);

    // Check for overrun.
    let remaining = (stream.buffer_attr.max_length as usize).saturating_sub(stream.buffer.len());
    let overflow = payload.len().saturating_sub(remaining);
    let payload = if overflow > 0 {
        protocol::write_command_message(
            &mut client.socket,
            u32::MAX,
            protocol::Command::Overflow(overflow as u32),
            client.protocol_version,
        )?;

        &payload[..remaining as usize]
    } else {
        payload
    };

    if let StreamState::Prebuffering(n) = stream.state {
        let needed = n.saturating_sub(payload.len() as u64);
        if needed > 0 {
            stream.state = StreamState::Prebuffering(needed)
        } else {
            stream.state = StreamState::Playing
        }
    }

    // Read the data into the buffer.
    stream.buffer.write_all(payload)?;
    stream.requested_bytes = stream.requested_bytes.saturating_sub(payload.len());

    Ok(())
}

fn play_samples(stream: &mut PlaybackStream) -> anyhow::Result<()> {
    // We'll simulate playback by discarding samples from the front of the buffer.
    if matches!(
        stream.state,
        StreamState::Playing | StreamState::Draining(_)
    ) {
        let sample_size = stream.sample_spec.format.bytes_per_sample();

        let bytes_per_tick = sample_size
            * stream.sample_spec.channels as usize
            * stream.sample_spec.sample_rate as usize
            / CLOCK_SPEED_HZ as usize;

        stream
            .buffer
            .drain(..std::cmp::min(bytes_per_tick, stream.buffer.len()));
    }

    Ok(())
}

fn apply_buffer_defaults(attr: &mut protocol::stream::BufferAttr) {
    if attr.max_length == u32::MAX {
        attr.max_length = 4096 * 1024;
    }

    if attr.target_length == u32::MAX {
        attr.target_length = 128 * 1024;
    }

    if attr.pre_buffering == u32::MAX {
        attr.pre_buffering = attr.target_length;
    }

    if attr.minimum_request_length == u32::MAX {
        attr.minimum_request_length = attr.target_length / 8;
    }
}
