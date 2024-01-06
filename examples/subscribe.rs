use std::os::unix::net::UnixStream;

use pulseaudio::protocol;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Find and connect to PulseAudio. The socket is usually in a well-known
    // location under XDG_RUNTIME_DIR.
    let socket_path = pulseaudio::socket_path_from_env().ok_or("PulseAudio not available")?;
    let mut sock = std::io::BufReader::new(UnixStream::connect(socket_path)?);

    // PulseAudio usually puts an authentication "cookie" in ~/.config/pulse/cookie.
    let cookie = pulseaudio::cookie_path_from_env()
        .and_then(|path| std::fs::read(path).ok())
        .unwrap_or_default();
    let auth = protocol::AuthParams {
        version: protocol::MAX_VERSION,
        supports_shm: false,
        supports_memfd: false,
        cookie,
    };

    // Write the auth "command" to the socket, and read the reply. The reply
    // contains the negotiated protocol version.
    protocol::write_command_message(
        sock.get_mut(),
        0,
        protocol::Command::Auth(auth),
        protocol::MAX_VERSION,
    )?;

    let (_, auth_reply) =
        protocol::read_reply_message::<protocol::AuthReply>(&mut sock, protocol::MAX_VERSION)?;
    let protocol_version = std::cmp::min(protocol::MAX_VERSION, auth_reply.version);

    // The next step is to set the client name.
    let mut props = protocol::Props::new();
    props.set(protocol::Prop::ApplicationName, "list-sinks");
    protocol::write_command_message(
        sock.get_mut(),
        1,
        protocol::Command::SetClientName(props),
        protocol_version,
    )?;

    // The reply contains our client ID.
    let _ =
        protocol::read_reply_message::<protocol::SetClientNameReply>(&mut sock, protocol_version)?;

    // Finally, write a command to create a subscription. The mask we pass will
    // determine which events we get.
    protocol::write_command_message(
        sock.get_mut(),
        2,
        protocol::Command::Subscribe(protocol::SubscriptionMask::ALL),
        protocol_version,
    )?;

    // The first reply is just an ACK.
    let seq = protocol::read_ack_message(&mut sock)?;
    assert_eq!(2, seq);

    eprintln!("waiting for events...");
    loop {
        let (_, event) = protocol::read_command_message(&mut sock, protocol_version)?;

        match event {
            protocol::Command::SubscribeEvent(event) => eprintln!(
                "got event {:?} for ID {:?} ({:?})",
                event.event_type, event.index, event.event_facility
            ),
            _ => eprintln!("got unexpected event {:?}", event),
        }
    }
}
