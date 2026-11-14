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

use std::{ffi::OsString, path::PathBuf};

mod client;
pub mod protocol;
pub use client::*;

/// Attempts to determine the socket path from the runtime environment, as
/// libpulse does:
///   - $PULSE_SERVER, which names the only server to use
///   - $PULSE_RUNTIME_PATH/native
///   - $XDG_RUNTIME_DIR/pulse/native
///
/// Returns None if no socket can be found or if $PULSE_SERVER points to a
/// remote server, i.e. starts with a prefix other than 'unix:'.
pub fn socket_path_from_env() -> Option<PathBuf> {
    socket_path_from(
        std::env::var_os("PULSE_SERVER"),
        std::env::var_os("PULSE_RUNTIME_PATH"),
        std::env::var_os("XDG_RUNTIME_DIR"),
    )
}

fn socket_path_from(
    server: Option<OsString>,
    runtime_path: Option<OsString>,
    xdg_runtime_dir: Option<OsString>,
) -> Option<PathBuf> {
    let usable = |path: PathBuf| {
        let stat = std::fs::metadata(&path).ok()?;
        (!stat.permissions().readonly()).then_some(path)
    };

    if let Some(server) = server {
        let path = server.to_str()?.strip_prefix("unix:")?;
        return usable(PathBuf::from(path));
    }

    runtime_path
        .map(|dir| PathBuf::from(dir).join("native"))
        .into_iter()
        .chain(xdg_runtime_dir.map(|dir| PathBuf::from(dir).join("pulse/native")))
        .find_map(usable)
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

        write_command_message(sock.get_mut(), 0, &Command::Auth(auth), MAX_VERSION)
            .context("sending auth command failed")?;
        let (_, auth_reply) =
            read_reply_message::<AuthReply>(sock, MAX_VERSION).context("auth command failed")?;

        let protocol_version = std::cmp::min(MAX_VERSION, auth_reply.version);

        let mut props = Props::new();
        props.set(Prop::ApplicationName, CString::new("pulseaudio-rs-tests")?);
        write_command_message(
            sock.get_mut(),
            1,
            &Command::SetClientName(props),
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

#[cfg(test)]
mod socket_path_test {
    use std::{ffi::OsString, fs, path::PathBuf};

    use super::socket_path_from;

    // A runtime directory holding the socket file at `socket` (relative)
    fn runtime_dir(name: &str, socket: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pulseaudio-rs-{}-{}", name, std::process::id()));
        let path = dir.join(socket);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"").unwrap();
        dir
    }

    fn os(path: &PathBuf) -> Option<OsString> {
        Some(path.clone().into_os_string())
    }

    #[test]
    fn falls_through_to_the_next_location() {
        let xdg = runtime_dir("fallthrough", "pulse/native");
        let missing = std::env::temp_dir().join("pulseaudio-rs-missing");

        assert_eq!(
            socket_path_from(None, os(&missing), os(&xdg)),
            Some(xdg.join("pulse/native"))
        );
    }

    #[test]
    fn pulse_runtime_path_holds_the_socket_itself() {
        let runtime = runtime_dir("runtime", "native");

        assert_eq!(
            socket_path_from(None, os(&runtime), None),
            Some(runtime.join("native"))
        );
    }

    #[test]
    fn pulse_server_is_the_only_server() {
        let xdg = runtime_dir("server", "pulse/native");
        let socket = xdg.join("pulse/native");
        let unix = |path: &str| Some(OsString::from(format!("unix:{path}")));

        assert_eq!(
            socket_path_from(unix(socket.to_str().unwrap()), None, None),
            Some(socket.clone())
        );
        assert_eq!(
            socket_path_from(unix("/nonexistent/native"), None, os(&xdg)),
            None
        );
        assert_eq!(
            socket_path_from(Some("tcp:localhost:4713".into()), None, os(&xdg)),
            None
        );
    }
}
