//! A pure Rust implementation of the PulseAudio protocol, suitable for writing
//! servers and clients.

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

use std::path::PathBuf;

pub mod protocol;

/// Attempts to determine the socket path from the runtime environment, checking
/// the following locations in order:
///   - $PULSE_SERVER
///   - $PULSE_RUNTIME_PATH/pulse/native
///   - $XDG_RUNTIME_DIR/pulse/native
///
/// Returns None if no socket can be found or if $PULSE_SERVER points to a
/// remote server, i.e. starts with a prefix other than 'unix:'.
pub fn socket_path_from_env() -> Option<PathBuf> {
    let paths = std::env::var("PULSE_SERVER")
        .ok()
        .filter(|s| s.starts_with("unix:"))
        .map(|s| PathBuf::from(&s[5..]))
        .into_iter()
        .chain(
            std::env::var("PULSE_RUNTIME_PATH")
                .ok()
                .map(|s| PathBuf::from(s).join("pulse/native")),
        )
        .chain(
            std::env::var("XDG_RUNTIME_DIR")
                .ok()
                .map(|s| PathBuf::from(s).join("pulse/native")),
        );

    for path in paths {
        let stat = std::fs::metadata(&path).ok()?;
        if !stat.permissions().readonly() {
            return Some(path);
        }
    }

    None
}

/// Attempts to find the authentication cookie from the environment, checking
/// the following locations in order:
///
///   - $PULSE_COOKIE
///   - $HOME/.config/pulse/cookie
///   - $HOME/.pulse-cookie
pub fn cookie_path_from_env() -> Option<PathBuf> {
    #[allow(deprecated)]
    let home = std::env::home_dir()?;

    let mut paths = std::env::var("PULSE_COOKIE")
        .ok()
        .map(PathBuf::from)
        .into_iter()
        .chain(std::iter::once(home.join(".config/pulse/cookie")))
        .chain(std::iter::once(home.join(".pulse-cookie")));

    paths.find(|path| path.exists())
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_test_util {
    use std::{ffi::CString, io::BufReader, os::unix::net::UnixStream};

    use anyhow::Context;

    use super::*;
    use crate::protocol::*;

    #[test]
    fn socket_path() -> anyhow::Result<()> {
        let path = socket_path_from_env();
        assert!(path.is_some());
        assert!(path.unwrap().exists());

        Ok(())
    }

    pub(crate) fn connect_to_server() -> anyhow::Result<BufReader<UnixStream>> {
        let socket_path = socket_path_from_env().context("error finding pulse socket")?;
        let sock = UnixStream::connect(socket_path).context("error connecting to pulse socket")?;

        Ok(BufReader::new(sock))
    }

    pub(crate) fn init_client(mut sock: &mut BufReader<UnixStream>) -> anyhow::Result<u16> {
        let cookie = cookie_path_from_env()
            .and_then(|path| std::fs::read(path).ok())
            .unwrap_or_default();
        if cookie.is_empty() {
            eprintln!("warning: no pulseaudio cookie found");
        }

        let auth = AuthParams {
            version: MAX_VERSION,
            supports_shm: false,
            supports_memfd: false,
            cookie,
        };

        write_command_message(sock.get_mut(), 0, Command::Auth(auth), MAX_VERSION)
            .context("sending auth command failed")?;
        let (_, auth_reply) =
            read_reply_message::<AuthReply>(sock, MAX_VERSION).context("auth command failed")?;

        let protocol_version = std::cmp::min(MAX_VERSION, auth_reply.version);

        let mut props = Props::new();
        props.set(Prop::ApplicationName, CString::new("pulseaudio-rs-tests")?);
        write_command_message(
            sock.get_mut(),
            1,
            Command::SetClientName(props),
            protocol_version,
        )
        .context("sending set_client_name command failed")?;
        let _ = read_reply_message::<SetClientNameReply>(&mut sock, protocol_version)
            .context("set_client_name command failed")?;

        Ok(protocol_version)
    }

    pub(crate) fn connect_and_init() -> anyhow::Result<(BufReader<UnixStream>, u16)> {
        let mut sock = connect_to_server()?;
        let protocol_version = init_client(&mut sock)?;

        Ok((sock, protocol_version))
    }
}
