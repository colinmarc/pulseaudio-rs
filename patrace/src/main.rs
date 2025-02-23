use std::{
    collections::HashMap,
    io::{Cursor, Read, Write},
};

use anyhow::{bail, Context};
use bytes::BytesMut;
use chrono::Utc;
use clap::Parser;
use console::{measure_text_width, style};
use mio::net::{UnixListener, UnixStream};
use pulseaudio::protocol::{self as pulse, SourceOutputInfoList, DESCRIPTOR_SIZE};

/// A tool for tracing PulseAudio commands. Connects to an upstream server, and
/// binds a socket for clients to connect to. All commands sent in either
/// direction are dumped to stdout.
///
/// To use it, first launch the program, binding some socket:
///     
///     $ patrace --bind /tmp/patrace.sock
///
/// Then, for example, list sinks with `pactl`:
///
///     $ PULSE_SERVER=/tmp/patrace.sock pactl list sinks
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The socket to use for the upstream connection. Defaults to fetching it
    /// from the environment.
    #[arg(long, value_name = "SOCKET")]
    upstream: Option<String>,

    /// The socket to bind as the server.
    #[arg(long, value_name = "SOCKET")]
    bind: String,
}

struct Connection {
    client_name: String,
    negotiated_version: u16,

    client: UnixStream,
    client_token: mio::Token,
    upstream: UnixStream,
    upstream_token: mio::Token,

    client_to_server: BytesMut,
    server_to_client: BytesMut,
    pending_reply: HashMap<u32, pulse::CommandTag>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Direction {
    ClientToServer,
    ServerToClient,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let upstream = args
        .upstream
        .or_else(|| pulseaudio::socket_path_from_env().map(|p| p.to_string_lossy().into_owned()))
        .ok_or(anyhow::anyhow!("failed to find upstream server socket"))?;

    let mut listener = UnixListener::bind(args.bind).context("failed to bind server socket")?;

    const LISTENER: mio::Token = mio::Token(0);

    // Client tokens start from 1024.
    let mut next_client_token = 1024;

    // Upstream tokens start from 2048.
    let mut next_upstream_token = 2048;

    let mut connections = Vec::new();

    let mut poll = mio::Poll::new()?;
    let mut events = mio::Events::with_capacity(1024);

    poll.registry()
        .register(&mut listener, LISTENER, mio::Interest::READABLE)?;

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                LISTENER => {
                    let (mut stream, addr) = listener.accept()?;
                    let token = mio::Token(next_client_token);
                    next_client_token += 1;

                    poll.registry()
                        .register(&mut stream, token, mio::Interest::READABLE)?;

                    // Connect upstream.
                    let mut upstream = UnixStream::connect(&upstream)?;

                    let upstream_token = mio::Token(next_upstream_token);
                    next_upstream_token += 1;

                    poll.registry().register(
                        &mut upstream,
                        upstream_token,
                        mio::Interest::READABLE,
                    )?;

                    let conn = Connection {
                        client_name: format!("{:?}", addr),
                        negotiated_version: pulse::MAX_VERSION,
                        client: stream,
                        client_token: token,
                        upstream,
                        upstream_token,
                        pending_reply: HashMap::new(),
                        client_to_server: BytesMut::new(),
                        server_to_client: BytesMut::new(),
                    };

                    connections.push(conn)
                }
                token if event.is_read_closed() => {
                    if let Some(pos) = connections
                        .iter()
                        .position(|c| c.client_token == token || c.upstream_token == token)
                    {
                        let mut conn = connections.remove(pos);
                        let msg = if conn.client_token == token {
                            "disconnected".into()
                        } else {
                            format!("disconnected {}", style("by server").bold())
                        };

                        println!(
                            "{}: {} {}",
                            style(Utc::now().to_string()).dim(),
                            style(conn.client_name).cyan(),
                            style(msg).red(),
                        );

                        poll.registry().deregister(&mut conn.client)?;
                        poll.registry().deregister(&mut conn.upstream)?;
                    }
                }
                token => {
                    if let Some(conn) = connections
                        .iter_mut()
                        .find(|c| c.client_token == token || c.upstream_token == token)
                    {
                        let direction = if conn.client_token == token {
                            Direction::ClientToServer
                        } else {
                            Direction::ServerToClient
                        };

                        match proxy(conn, direction) {
                            Ok(()) => (),
                            Err(e) => match e.downcast_ref::<std::io::Error>() {
                                // I/O errors might happen if the one end
                                // hangs up. We'll catch the close event.
                                Some(_) => continue,
                                _ => bail!(e),
                            },
                        }
                    }
                }
            }
        }
    }
}

fn proxy(conn: &mut Connection, direction: Direction) -> anyhow::Result<()> {
    let (src, dest) = if direction == Direction::ClientToServer {
        (&mut conn.client, &mut conn.upstream)
    } else {
        (&mut conn.upstream, &mut conn.client)
    };

    let buf = if direction == Direction::ClientToServer {
        &mut conn.client_to_server
    } else {
        &mut conn.server_to_client
    };

    let mut next_read = 4096;

    'read: loop {
        let off = buf.len();
        buf.resize(off + next_read, 0);

        let n = match src.read(&mut buf[off..]) {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                buf.truncate(off);
                break 'read;
            }
            v => v.context("recv error")?,
        };

        buf.truncate(off + n);

        loop {
            if buf.len() < pulse::DESCRIPTOR_SIZE {
                continue 'read;
            }

            let desc = pulse::read_descriptor(&mut Cursor::new(&buf[..pulse::DESCRIPTOR_SIZE]))?;
            if buf.len() < (desc.length as usize + pulse::DESCRIPTOR_SIZE) {
                next_read = desc.length as usize + pulse::DESCRIPTOR_SIZE - buf.len();
                continue 'read;
            }

            let msg_bytes = buf.split_to(pulse::DESCRIPTOR_SIZE + desc.length as usize);
            let mut cursor = Cursor::new(&msg_bytes);

            let mut proxy = true;

            if desc.channel == u32::MAX {
                cursor.set_position(DESCRIPTOR_SIZE as u64);
                match pulse::Command::read_tag_prefixed(&mut cursor, conn.negotiated_version) {
                    Ok((seq, cmd)) => {
                        if cmd == pulse::Command::Reply {
                            let reply = match conn.pending_reply.remove(&seq) {
                                Some(pulse::CommandTag::Auth) => {
                                    // Store the negotiated protocol
                                    // version so we can parse stuff
                                    // properly.
                                    let mut ts = pulse::TagStructReader::new(
                                        &mut cursor,
                                        conn.negotiated_version,
                                    );

                                    let reply: pulse::AuthReply = ts.read()?;
                                    conn.negotiated_version =
                                        std::cmp::min(conn.negotiated_version, reply.version);

                                    Box::new(reply)
                                }
                                Some(tag) => read_reply(&mut cursor, tag, conn.negotiated_version)
                                    .context(format!("reading reply to [{}] {:?}", seq, tag))?,
                                None => Box::new(UnknownReply(msg_bytes.len())),
                            };

                            dump_command(&conn.client_name, seq, &reply, direction);
                        } else {
                            if let pulse::Command::Auth(ref auth) = cmd {
                                // We don't support shm or memfd.
                                let mut auth = auth.clone();
                                auth.supports_memfd = false;
                                auth.supports_shm = false;

                                // Make sure the version isn't too high.
                                if auth.version > pulse::MAX_VERSION {
                                    panic!(
                                        "client requested version {} but we only support up to {}",
                                        auth.version,
                                        pulse::MAX_VERSION
                                    );
                                }

                                let version = auth.version;
                                pulse::write_command_message(
                                    dest,
                                    seq,
                                    &pulse::Command::Auth(auth),
                                    version,
                                )?;

                                conn.negotiated_version =
                                    std::cmp::min(version, conn.negotiated_version);
                                proxy = false;
                            } else if let pulse::Command::SetClientName(ref props) = cmd {
                                if let Some(name) = props.get(pulse::Prop::ApplicationName) {
                                    // Store the client name for printing.
                                    conn.client_name = String::from_utf8_lossy(name).into_owned();
                                }
                            }

                            dump_command(&conn.client_name, seq, &cmd, direction);

                            if seq != u32::MAX {
                                conn.pending_reply.insert(seq, cmd.tag());
                            }
                        }
                    }
                    Err(pulse::ProtocolError::Unimplemented(seq, tag)) => {
                        conn.pending_reply.insert(seq, tag);
                        dump_command(
                            &conn.client_name,
                            seq,
                            &UnimplementedCommand(tag, msg_bytes.len()),
                            direction,
                        );
                    }
                    Err(e) => return Err(e.into()),
                };
            } else {
                let header = header(&conn.client_name, u32::MAX, direction);
                println!(
                    "{}\n<write of len {} to channel {}>",
                    header, desc.length, desc.channel
                );
            }

            // Proxy the message upstream.
            if proxy {
                dest.write_all(&msg_bytes)?;
            }
        }
    }

    Ok(())
}

struct UnimplementedCommand(pulse::CommandTag, usize);

impl std::fmt::Debug for UnimplementedCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<unimplemented command \"{:?}\" ({} bytes)>",
            self.0, self.1,
        )
    }
}

struct UnknownReply(usize);

impl std::fmt::Debug for UnknownReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<unknown reply ({} bytes)>", self.0)
    }
}

struct Ack;

impl std::fmt::Debug for Ack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<ack>")
    }
}

fn read_reply(
    cursor: &mut Cursor<&BytesMut>,
    tag: pulse::CommandTag,
    protocol_version: u16,
) -> anyhow::Result<Box<dyn std::fmt::Debug>> {
    let reply_len = cursor.get_ref().len() - cursor.position() as usize;
    let mut ts = pulse::TagStructReader::new(cursor, protocol_version);

    let res: Box<dyn std::fmt::Debug> = match tag {
        pulse::CommandTag::Auth => Box::new(ts.read::<pulse::AuthReply>()?),
        pulse::CommandTag::CreatePlaybackStream => {
            Box::new(ts.read::<pulse::CreatePlaybackStreamReply>()?)
        }
        pulse::CommandTag::DeletePlaybackStream => Box::new(Ack),
        pulse::CommandTag::CreateRecordStream => {
            Box::new(ts.read::<pulse::CreateRecordStreamReply>()?)
        }
        pulse::CommandTag::DeleteRecordStream => Box::new(Ack),
        pulse::CommandTag::SetClientName => Box::new(ts.read::<pulse::SetClientNameReply>()?),
        pulse::CommandTag::LookupSink => Box::new(ts.read::<pulse::LookupReply>()),
        pulse::CommandTag::LookupSource => Box::new(ts.read::<pulse::LookupReply>()),
        pulse::CommandTag::DrainPlaybackStream => Box::new(Ack),
        pulse::CommandTag::Stat => Box::new(ts.read::<pulse::StatInfo>()?),
        pulse::CommandTag::GetPlaybackLatency => Box::new(ts.read::<pulse::PlaybackLatency>()?),
        pulse::CommandTag::GetServerInfo => Box::new(ts.read::<pulse::ServerInfo>()?),
        pulse::CommandTag::GetCardInfo => Box::new(ts.read::<pulse::CardInfo>()?),
        pulse::CommandTag::GetCardInfoList => Box::new(ts.read::<pulse::CardInfoList>()?),
        pulse::CommandTag::GetSinkInfo => Box::new(ts.read::<pulse::SinkInfo>()?),
        pulse::CommandTag::GetSinkInfoList => Box::new(ts.read::<pulse::SinkInfoList>()?),
        pulse::CommandTag::GetSourceInfo => Box::new(ts.read::<pulse::SourceInfo>()?),
        pulse::CommandTag::GetSourceInfoList => Box::new(ts.read::<pulse::SourceInfoList>()?),
        pulse::CommandTag::GetModuleInfo => Box::new(ts.read::<pulse::ModuleInfo>()?),
        pulse::CommandTag::GetModuleInfoList => Box::new(ts.read::<pulse::ModuleInfoList>()?),
        pulse::CommandTag::GetClientInfo => Box::new(ts.read::<pulse::ClientInfo>()?),
        pulse::CommandTag::GetClientInfoList => Box::new(ts.read::<pulse::ClientInfoList>()?),
        pulse::CommandTag::GetSinkInputInfo => Box::new(ts.read::<pulse::SinkInputInfo>()?),
        pulse::CommandTag::GetSinkInputInfoList => Box::new(ts.read::<pulse::SinkInputInfoList>()?),
        pulse::CommandTag::GetSourceOutputInfo => Box::new(ts.read::<pulse::SourceOutputInfo>()?),
        pulse::CommandTag::GetSourceOutputInfoList => Box::new(ts.read::<SourceOutputInfoList>()?),
        pulse::CommandTag::GetSampleInfo => Box::new(ts.read::<pulse::SampleInfo>()?),
        pulse::CommandTag::GetSampleInfoList => Box::new(ts.read::<pulse::SampleInfoList>()?),
        pulse::CommandTag::SetPlaybackStreamBufferAttr => {
            Box::new(ts.read::<pulse::SetPlaybackStreamBufferAttrReply>()?)
        }
        pulse::CommandTag::SetRecordStreamBufferAttr => {
            Box::new(ts.read::<pulse::SetRecordStreamBufferAttrReply>()?)
        }
        pulse::CommandTag::Subscribe => Box::new(Ack),
        _ if reply_len == 0 => Box::new(Ack),
        _ => Box::new(UnknownReply(reply_len)),
    };

    Ok(res)
}

fn dump_command(client_name: &str, seq: u32, cmd: &impl std::fmt::Debug, direction: Direction) {
    let header = header(client_name, seq, direction);

    println!(
        "{}\n{}\n{}",
        header,
        "-".repeat(measure_text_width(&header)),
        style(format!("{:#?}", cmd)).dim()
    );
}

fn header(client_name: &str, seq: u32, direction: Direction) -> String {
    let seq = match seq {
        u32::MAX => -1,
        _ => seq as i32,
    };

    match direction {
        Direction::ClientToServer => format!(
            "{} [{}]: {} {}",
            style(Utc::now().to_string()).dim(),
            style(seq).bold(),
            style(client_name).cyan(),
            style("-> server").bold(),
        ),

        Direction::ServerToClient => format!(
            "{} [{}]: {} {}",
            style(Utc::now().to_string()).dim(),
            style(seq).bold(),
            style("server ->").bold(),
            style(client_name).cyan(),
        ),
    }
}
