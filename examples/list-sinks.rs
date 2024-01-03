use std::{os::unix::net::UnixStream, path::Path};

use pulseaudio::protocol;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Find and connect to PulseAudio. The socket is usually in a well-known
    // location under XDG_RUNTIME_DIR.
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR")?;
    let socket_path = Path::new(&xdg_runtime_dir).join("pulse/native");
    if !socket_path.exists() {
        return Err(format!(
            "pulseaudio socket not found at {}",
            socket_path.to_string_lossy()
        )
        .into());
    }

    let mut sock = std::io::BufReader::new(UnixStream::connect(&socket_path)?);

    // PulseAudio usually puts an authentication "cookie" in ~/.config/pulse/cookie.
    let home = std::env::var("HOME")?;
    let cookie_path = Path::new(&home).join(".config/pulse/cookie");
    let auth = if cookie_path.exists() {
        let cookie = std::fs::read(&cookie_path)?;
        protocol::AuthParams {
            version: protocol::MAX_VERSION,
            supports_shm: false,
            supports_memfd: false,
            cookie,
        }
    } else {
        protocol::AuthParams {
            version: protocol::MAX_VERSION,
            supports_shm: false,
            supports_memfd: false,
            cookie: Vec::new(),
        }
    };

    // Write the auth "command" to the socket, and read the reply. The reply
    // contains the negotiated protocol version.
    protocol::write_command_message(
        sock.get_mut(),
        0,
        protocol::Command::Auth(auth),
        protocol::MAX_VERSION,
    )?;
    let (_, auth_info) = protocol::read_reply_message::<protocol::AuthReply>(&mut sock)?;
    let protocol_version = std::cmp::min(protocol::MAX_VERSION, auth_info.version);

    // The next step is to set the client name.
    let mut props = protocol::Props::new();
    props.set(protocol::Prop::ApplicationName, "list-sinks");
    protocol::write_command_message(
        sock.get_mut(),
        1,
        protocol::Command::SetClientName(props),
        protocol_version,
    )?;

    let _ = protocol::read_reply_message::<protocol::SetClientNameReply>(&mut sock)?;

    // Finally, write a command to get the list of sinks. The reply contains the information we're after.
    protocol::write_command_message(
        sock.get_mut(),
        2,
        protocol::Command::GetSinkInfoList,
        protocol_version,
    )?;

    let (_, info_list) = protocol::read_reply_message::<protocol::SinkInfoList>(&mut sock)?;
    for info in info_list {
        println!("{:#?}", info);
    }

    Ok(())
}
