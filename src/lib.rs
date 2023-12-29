//! A pure Rust implementation of the PulseAudio protocol, suitable for writing servers and clients.

#![warn(
    anonymous_parameters,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]

pub mod protocol;

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_test_util {
    use std::{io::BufReader, os::unix::net::UnixStream};

    use anyhow::Context;

    use crate::protocol::*;

    pub(crate) fn connect_to_server() -> anyhow::Result<BufReader<UnixStream>> {
        let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR")?;
        let socket_path = std::path::Path::new(&xdg_runtime_dir).join("pulse/native");
        let sock = UnixStream::connect(socket_path).context("error connecting to pulse socket")?;

        Ok(BufReader::new(sock))
    }

    pub(crate) fn init_client(mut sock: &mut BufReader<UnixStream>) -> anyhow::Result<()> {
        let home = std::env::var("HOME")?;
        let mut cookie = Vec::new();
        for cookie_name in &[".pulse-cookie", ".config/pulse/cookie"] {
            let cookie_path = std::path::Path::new(&home).join(cookie_name);
            if cookie_path.exists() {
                cookie = std::fs::read(&cookie_path)?;
                break;
            }
        }

        if cookie.is_empty() {
            eprintln!("warning: no pulseaudio cookie found");
        }

        let auth = AuthParams {
            version: MAX_VERSION,
            supports_shm: false,
            supports_memfd: false,
            cookie,
        };

        write_command_message(sock.get_mut(), 0, Command::Auth(auth))
            .context("sending auth command failed")?;
        let _ = read_reply_message::<AuthReply>(sock).context("auth command failed")?;

        let mut props = Props::new();
        props.set(Prop::ApplicationName, "pulseaudio-rs-tests");
        write_command_message(sock.get_mut(), 1, Command::SetClientName(props))
            .context("sending set_client_name command failed")?;
        let _ = read_reply_message::<SetClientNameReply>(&mut sock)
            .context("set_client_name command failed")?;

        Ok(())
    }

    pub(crate) fn connect_and_init() -> anyhow::Result<BufReader<UnixStream>> {
        let mut sock = connect_to_server()?;
        init_client(&mut sock)?;

        Ok(sock)
    }
}
