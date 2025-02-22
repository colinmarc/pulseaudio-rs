use std::ffi::CString;

use crate::protocol::{serde::*, ProtocolError};

use super::CommandReply;

/// Server state for a server, in response to [`super::Command::GetServerInfo`].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct ServerInfo {
    /// Server "package name" (usually "pulseaudio")
    pub server_name: Option<CString>,

    /// Version string of the daemon.
    pub server_version: Option<CString>,

    /// User name of the daemon process.
    pub user_name: Option<CString>,

    /// Host name the daemon is running on.
    pub host_name: Option<CString>,

    /// Default sample specification.
    pub sample_spec: SampleSpec,

    /// A random ID to indentify the server.
    pub cookie: u32,

    /// Name of the current default sink.
    pub default_sink_name: Option<CString>,

    /// Name of the current default source.
    pub default_source_name: Option<CString>,

    /// Channel map for the default sink.
    pub channel_map: ChannelMap,
}

impl CommandReply for ServerInfo {}

impl TagStructRead for ServerInfo {
    fn read(ts: &mut TagStructReader<'_>, protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut info = Self {
            server_name: ts.read_string()?,
            server_version: ts.read_string()?,
            user_name: ts.read_string()?,
            host_name: ts.read_string()?,
            sample_spec: ts.read()?,
            default_sink_name: ts.read_string()?,
            default_source_name: ts.read_string()?,
            cookie: ts.read_u32()?,
            ..Default::default()
        };

        if protocol_version >= 15 {
            info.channel_map = ts.read()?;
        }

        Ok(info)
    }
}

impl TagStructWrite for ServerInfo {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_string(self.server_name.as_ref())?;
        w.write_string(self.server_version.as_ref())?;
        w.write_string(self.user_name.as_ref())?;
        w.write_string(self.host_name.as_ref())?;
        w.write(self.sample_spec)?;
        w.write_string(self.default_sink_name.as_ref())?;
        w.write_string(self.default_source_name.as_ref())?;
        w.write_u32(self.cookie)?;
        w.write(self.channel_map)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::test_util::test_serde;

    use super::*;

    #[test]
    fn server_info_serde() -> anyhow::Result<()> {
        let info = ServerInfo {
            server_name: Some(CString::new("foo").unwrap()),
            server_version: Some(CString::new("bar").unwrap()),
            user_name: Some(CString::new("baz").unwrap()),
            host_name: None,
            sample_spec: SampleSpec {
                format: SampleFormat::S16Le,
                channels: 2,
                sample_rate: 44100,
            },
            default_sink_name: Some(CString::new("sink0").unwrap()),
            default_source_name: Some(CString::new("source0").unwrap()),
            cookie: 0xdeadbeef,
            channel_map: ChannelMap::default(),
        };

        test_serde(&info)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn get_server_info() -> Result<(), Box<dyn std::error::Error>> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(sock.get_mut(), 0, Command::GetServerInfo, protocol_version)?;
        let (_, info) = read_reply_message::<ServerInfo>(&mut sock, protocol_version)?;

        assert!(info.server_name.is_some());
        assert!(info.server_version.is_some());
        assert!(info.user_name.is_some());
        assert!(info.host_name.is_some());
        assert!(info.default_sink_name.is_some());
        assert!(info.default_source_name.is_some());
        assert!(info.cookie != 0);
        assert!(info.channel_map.num_channels() > 0);

        Ok(())
    }
}
