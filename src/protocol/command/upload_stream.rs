use crate::protocol::{serde::*, ProtocolError};
use crate::protocol::{ChannelMap, Props, SampleSpec};

use std::ffi::CString;

use super::CommandReply;

/// Parameters for [`super::Command::CreateUploadStream`].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct UploadStreamParams {
    /// Name of the sample.
    pub media_name: Option<CString>,

    /// Sample format for the stream.
    pub sample_spec: SampleSpec,

    /// Channel map for the stream.
    pub channel_map: ChannelMap,

    /// Length of the sample in bytes.
    pub length: u32,

    /// Additional properties for the stream.
    pub props: Props,
}

impl TagStructRead for UploadStreamParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            media_name: ts.read_string()?,
            sample_spec: ts.read()?,
            channel_map: ts.read()?,
            length: ts.read_u32()?,
            props: ts.read()?,
        })
    }
}

impl TagStructWrite for UploadStreamParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_string(self.media_name.as_ref())?;
        ts.write(self.sample_spec)?;
        ts.write(self.channel_map)?;
        ts.write_u32(self.length)?;
        ts.write(&self.props)?;
        Ok(())
    }
}

/// The server response to [`super::Command::CreateUploadStream`].
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct CreateUploadStreamReply {
    /// Channel ID, which is used in other commands to refer to this stream.
    /// Unlike the stream index, it is scoped to the connection.
    pub channel: u32,

    /// The length of the sample in bytes.
    pub length: u32,
}

impl CommandReply for CreateUploadStreamReply {}

impl TagStructRead for CreateUploadStreamReply {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let reply = Self {
            channel: ts.read_u32()?,
            length: ts.read_u32()?,
        };

        Ok(reply)
    }
}

impl TagStructWrite for CreateUploadStreamReply {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.channel)?;
        w.write_u32(self.length)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::test_util::test_serde;

    use super::*;

    #[test]
    fn params_serde() -> anyhow::Result<()> {
        let params = UploadStreamParams {
            media_name: Some(CString::new("media_name")?),
            sample_spec: SampleSpec {
                format: SampleFormat::S16Le,
                sample_rate: 44100,
                channels: 2,
            },
            channel_map: ChannelMap::stereo(),
            length: 1024,
            props: Props::new(),
        };

        test_serde(&params)
    }

    #[test]
    fn reply_serde() -> anyhow::Result<()> {
        let reply = CreateUploadStreamReply {
            channel: 0,
            length: 1024,
        };

        test_serde(&reply)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use super::*;
    use crate::integration_test_util::*;
    use crate::protocol::*;

    #[test]
    fn create_upload_stream() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            Command::CreateUploadStream(UploadStreamParams {
                media_name: Some(CString::new("media_name")?),
                sample_spec: SampleSpec {
                    format: SampleFormat::S16Le,
                    sample_rate: 44100,
                    channels: 2,
                },
                channel_map: ChannelMap::stereo(),
                length: 1024,
                ..Default::default()
            }),
            protocol_version,
        )?;

        let (_, reply) =
            read_reply_message::<CreateUploadStreamReply>(&mut sock, protocol_version)?;
        assert_eq!(reply.length, 1024);

        Ok(())
    }
}
