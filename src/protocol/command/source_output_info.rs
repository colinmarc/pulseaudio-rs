use std::ffi::CString;

use super::CommandReply;
use crate::protocol::{serde::*, ProtocolError};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceOutputInfo {
    /// ID of the source output.
    pub index: u32,

    /// The name of the source output.
    pub name: CString,

    /// The ID of the owning module.
    pub owner_module_index: Option<u32>,

    /// The ID of the owning client.
    pub client_index: Option<u32>,

    /// The ID of the owning source.
    pub source_index: u32,

    /// The formmat of samples that the output expects.
    pub sample_spec: SampleSpec,

    /// The mapping of channels to positions that the output expects.
    pub channel_map: ChannelMap,

    /// The volume of the output.
    pub cvolume: ChannelVolume,

    /// The latency due to buffering, in microseconds.
    pub buffer_latency: u64,

    /// The latency of the source device, in microseconds.
    pub source_latency: u64,

    /// The resampling method used, if any.
    pub resample_method: Option<CString>,

    /// The name of the driver this output belongs to.
    pub driver: Option<CString>,

    /// The properties of the output.
    pub props: Props,

    /// Whether the output is muted.
    pub muted: bool,

    /// Whether the output is corked (temporarily paused).
    pub corked: bool,

    /// If false, the volume information is considered invalid.
    pub has_volume: bool,

    /// If true, the volume can be set by the client.
    pub volume_writable: bool,

    /// The format that the output expects.
    pub format: FormatInfo,
}

impl CommandReply for SourceOutputInfo {}

impl TagStructRead for SourceOutputInfo {
    fn read(ts: &mut TagStructReader, protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut output_info = Self {
            index: ts.read_u32()?,
            name: ts
                .read_string()?
                .ok_or_else(|| ProtocolError::Invalid("null source output name".into()))?,
            owner_module_index: ts.read_index()?,
            client_index: ts.read_index()?,
            source_index: ts.read_u32()?,
            sample_spec: ts.read()?,
            channel_map: ts.read()?,
            buffer_latency: ts.read_usec()?,
            source_latency: ts.read_usec()?,
            resample_method: ts.read_string()?,
            driver: ts.read_string()?,
            props: ts.read()?,
            ..Default::default()
        };

        if protocol_version >= 19 {
            output_info.corked = ts.read_bool()?;
        }

        if protocol_version >= 22 {
            output_info.cvolume = ts.read()?;
            output_info.muted = ts.read_bool()?;
            output_info.has_volume = ts.read_bool()?;
            output_info.volume_writable = ts.read_bool()?;
            output_info.format = ts.read()?;
        }

        Ok(output_info)
    }
}

impl TagStructWrite for SourceOutputInfo {
    fn write(&self, w: &mut TagStructWriter, protocol_version: u16) -> Result<(), ProtocolError> {
        w.write_u32(self.index)?;
        w.write_string(Some(&self.name))?;
        w.write_index(self.owner_module_index)?;
        w.write_index(self.client_index)?;
        w.write_u32(self.source_index)?;
        w.write(&self.sample_spec)?;
        w.write(&self.channel_map)?;
        w.write_usec(self.buffer_latency)?;
        w.write_usec(self.source_latency)?;
        w.write_string(self.resample_method.as_ref())?;
        w.write_string(self.driver.as_ref())?;
        w.write(&self.props)?;

        if protocol_version >= 19 {
            w.write_bool(self.corked)?;
        }

        if protocol_version >= 22 {
            w.write(&self.cvolume)?;
            w.write_bool(self.muted)?;
            w.write_bool(self.has_volume)?;
            w.write_bool(self.volume_writable)?;
            w.write(&self.format)?;
        }

        Ok(())
    }
}

pub type SourceOutputInfoList = Vec<SourceOutputInfo>;

impl CommandReply for SourceOutputInfoList {}

impl TagStructRead for SourceOutputInfoList {
    fn read(ts: &mut TagStructReader, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut outputs = Vec::new();
        while ts.has_data_left()? {
            outputs.push(ts.read()?);
        }

        Ok(outputs)
    }
}

impl TagStructWrite for SourceOutputInfoList {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        for output in self {
            w.write(output)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn source_output_info_list_serde() -> anyhow::Result<()> {
        let outputs = vec![
            SourceOutputInfo {
                index: 1,
                name: CString::new("output 1")?,
                ..Default::default()
            },
            SourceOutputInfo {
                index: 5,
                name: CString::new("output 2")?,
                ..Default::default()
            },
        ];

        test_serde(&outputs)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn get_source_input_info() -> anyhow::Result<()> {
        let mut sock = connect_and_init()?;

        write_command_message(sock.get_mut(), 0, Command::GetSourceOutputInfoList)?;
        let (seq, info_list) = read_reply_message::<SourceOutputInfoList>(&mut sock)?;
        assert_eq!(seq, 0);

        // The list is often empty.
        if info_list.len() == 0 {
            return Ok(());
        }

        write_command_message(
            sock.get_mut(),
            1,
            Command::GetSourceOutputInfo(info_list[0].index),
        )?;

        let (seq, info) = read_reply_message::<SourceOutputInfo>(&mut sock)?;
        assert_eq!(seq, 1);

        assert_eq!(info, info_list[0]);

        Ok(())
    }
}
