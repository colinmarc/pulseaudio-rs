//! Utilities for finding PulseAudio paths.

use std::env;
use std::path::PathBuf;
use std::ffi::OsString;

/// Locates the PulseAudio runtime directory.
///
/// This is the directory that contains the server's Unix socket.
pub fn runtime_dir() -> PathBuf {
    env::var_os("PULSE_RUNTIME_PATH").map(PathBuf::from)
        .or_else(|| env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).map(|mut xdg| {
            xdg.push("pulse");
            xdg
        }))
        .unwrap_or_else(|| {
            let mut path = PathBuf::from(user_home());
            path.push(".pulse");
            path
        })
}

pub fn user_home() -> OsString {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .unwrap_or_else(|| {
            // TODO: Use getpwuid
            unimplemented!();
        })
}

pub fn config_home_dir() -> PathBuf {
    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
        let mut dir = PathBuf::from(xdg);
        dir.push("pulse");
        dir
    } else {
        let mut home = PathBuf::from(user_home());
        home.push(".config");
        home.push("pulse");
        home
    }
}

const COOKIE_NAME: &str = "cookie";
const COOKIE_NAME_FALLBACK: &str = ".pulse-cookie";

pub fn cookie_path() -> PathBuf {
    let mut conf_cookie = config_home_dir();
    conf_cookie.push(COOKIE_NAME);
    if conf_cookie.exists() {
        return conf_cookie;
    }

    let mut home_cookie = PathBuf::from(user_home());
    home_cookie.push(COOKIE_NAME_FALLBACK);
    if home_cookie.exists() {
        return home_cookie;
    }

    // return conf_cookie anyways and have the application create it
    conf_cookie
}

/*

$conf = "config home dir"
* $XDG_CONFIG_HOME/pulse
* $HOME/.config/pulse


let relpath =
* native-protocol module params: auth-cookie, then cookie (legacy) -> $conf/$param-val, create if not exist
* try $conf/cookie
* try $HOME/.pulse-cookie
* create $conf/cookie

*/
