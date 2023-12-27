pub mod protocol;

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_test_util {
    use std::{io::BufReader, os::unix::net::UnixStream};

    use crate::protocol::*;

    pub fn connect_to_server() -> anyhow::Result<BufReader<UnixStream>> {
        let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR")?;
        let socket_path = std::path::Path::new(&xdg_runtime_dir).join("pulse/native");
        let sock = UnixStream::connect(socket_path)?;

        Ok(BufReader::new(sock))
    }

    pub fn init_client(mut sock: &mut BufReader<UnixStream>) -> anyhow::Result<()> {
        let cookie_path = std::path::Path::new(&std::env::var("HOME")?).join(".pulse-cookie");
        let cookie = std::fs::read(cookie_path)?;

        let auth = AuthParams {
            version: PROTOCOL_VERSION,
            supports_shm: false,
            supports_memfd: false,
            cookie,
        };

        write_command_message(sock.get_mut(), 0, Command::Auth(auth))?;
        let _ = read_reply_message::<AuthReply>(sock)?;

        let mut props = Props::new();
        props.set(Prop::ApplicationName, "pulseaudio-rs-tests");
        write_command_message(sock.get_mut(), 1, Command::SetClientName(props))?;
        let _ = read_reply_message::<SetClientNameReply>(&mut sock)?;

        Ok(())
    }

    pub fn connect_and_init() -> anyhow::Result<BufReader<UnixStream>> {
        let mut sock = connect_to_server()?;
        init_client(&mut sock)?;

        Ok(sock)
    }
}
