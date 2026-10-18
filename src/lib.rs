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

use std::path::{Path, PathBuf};

mod client;
pub mod protocol;
pub use client::*;

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

fn cookie_from_env() -> Option<Vec<u8>> {
    if let Some(path) = cookie_path_from_env() {
        return std::fs::read(path).ok();
    }

    #[allow(deprecated)]
    let path = std::env::var_os("PULSE_COOKIE")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::home_dir().map(|home| home.join(".config/pulse/cookie")))?;

    match create_cookie(&path) {
        Ok(cookie) => Some(cookie),
        Err(err) => {
            log::warn!("failed to create cookie at {}: {err}", path.display());
            None
        }
    }
}

fn create_cookie(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::{
        fs,
        io::{Read, Write},
        os::unix::fs::{DirBuilderExt, OpenOptionsExt},
        sync::atomic::{AtomicUsize, Ordering},
    };

    static DRAFTS: AtomicUsize = AtomicUsize::new(0);

    let mut cookie = vec![0; 256];
    fs::File::open("/dev/urandom")?.read_exact(&mut cookie)?;

    if let Some(dir) = path.parent() {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
    }

    let mut draft = path.as_os_str().to_owned();
    draft.push(format!(
        ".{}.{}",
        std::process::id(),
        DRAFTS.fetch_add(1, Ordering::Relaxed)
    ));

    let linked = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&draft)
        .and_then(|mut file| file.write_all(&cookie))
        .and_then(|()| fs::hard_link(&draft, path));
    let _ = fs::remove_file(&draft);

    match linked {
        Ok(()) => Ok(cookie),
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => fs::read(path),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod cookie_test {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn create_cookie_like_libpulse() -> std::io::Result<()> {
        let dir = std::env::temp_dir().join(format!("pulseaudio-cookie-{}", std::process::id()));
        let path = dir.join("pulse/cookie");

        let created = create_cookie(&path)?;
        assert_eq!(created.len(), 256);
        assert_eq!(std::fs::read(&path)?, created);
        assert_eq!(
            std::fs::metadata(&path)?.permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(path.parent().unwrap())?
                .permissions()
                .mode()
                & 0o777,
            0o700
        );

        assert_eq!(create_cookie(&path)?, created);
        assert_eq!(std::fs::read_dir(path.parent().unwrap())?.count(), 1);

        std::fs::remove_dir_all(dir)
    }
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
