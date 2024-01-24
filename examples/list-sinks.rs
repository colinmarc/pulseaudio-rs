use std::{ffi::CString, os::unix::net::UnixStream};

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
    let (_, auth_info) =
        protocol::read_reply_message::<protocol::AuthReply>(&mut sock, protocol::MAX_VERSION)?;
    let protocol_version = std::cmp::min(protocol::MAX_VERSION, auth_info.version);

    // The next step is to set the client name.
    let mut props = protocol::Props::new();
    props.set(
        protocol::Prop::ApplicationName,
        CString::new("list-sinks").unwrap(),
    );
    protocol::write_command_message(
        sock.get_mut(),
        1,
        protocol::Command::SetClientName(props),
        protocol_version,
    )?;

    let _ =
        protocol::read_reply_message::<protocol::SetClientNameReply>(&mut sock, protocol_version)?;

    // Finally, write a command to get the list of sinks. The reply contains the information we're after.
    protocol::write_command_message(
        sock.get_mut(),
        2,
        protocol::Command::GetSinkInfoList,
        protocol_version,
    )?;

    let (_, info_list) =
        protocol::read_reply_message::<protocol::SinkInfoList>(&mut sock, protocol_version)?;
    for info in info_list {
        println!("{:#?}", info);
    }

    Ok(())
}
