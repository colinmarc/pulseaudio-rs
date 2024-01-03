use std::ffi::CString;

use super::CommandReply;
use crate::protocol::{serde::*, ProtocolError};

/// Server state for a sink input, in response to [`super::Command::GetSinkInputInfo`].
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SinkInputInfo {
    /// ID of the sink input.
    pub index: u32,

    /// The name of the sink input.
    pub name: CString,

    /// The ID of the owning module.
    pub owner_module_index: Option<u32>,

    /// The ID of the owning client.
    pub client_index: Option<u32>,

    /// The ID of the owning sink.
    pub sink_index: u32,

    /// The formmat of samples that the input expects.
    pub sample_spec: SampleSpec,

    /// The mapping of channels to positions that the input expects.
    pub channel_map: ChannelMap,

    /// The volume of the input.
    pub cvolume: ChannelVolume,

    /// The latency due to buffering, in microseconds.
    pub buffer_latency: u64,

    /// The latency of the sink device, in microseconds.
    pub sink_latency: u64,

    /// The resampling method used, if any.
    pub resample_method: Option<CString>,

    /// The name of the driver this input belongs to.
    pub driver: Option<CString>,

    /// The properties of the input.
    pub props: Props,

    /// Whether the input is muted.
    pub muted: bool,

    /// Whether the input is corked (temporarily paused).
    pub corked: bool,

    /// If false, the volume information is considered invalid.
    pub has_volume: bool,

    /// If true, the volume can be set by the client.
    pub volume_writable: bool,

    /// The format that the input expects.
    pub format: FormatInfo,
}

impl CommandReply for SinkInputInfo {}

impl TagStructRead for SinkInputInfo {
    fn read(ts: &mut TagStructReader<'_>, protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut input_info = Self {
            index: ts.read_u32()?,
            name: ts
                .read_string()?
                .ok_or_else(|| ProtocolError::Invalid("null sink input name".into()))?,
            owner_module_index: ts.read_index()?,
            client_index: ts.read_index()?,
            sink_index: ts.read_u32()?,
            sample_spec: ts.read()?,
            channel_map: ts.read()?,
            cvolume: ts.read()?,
            buffer_latency: ts.read_usec()?,
            sink_latency: ts.read_usec()?,
            resample_method: ts.read_string()?,
            driver: ts.read_string()?,
            muted: ts.read_bool()?,
            props: ts.read()?,
            ..Default::default()
        };

        if protocol_version >= 19 {
            input_info.corked = ts.read_bool()?;
        }

        if protocol_version >= 20 {
            input_info.has_volume = ts.read_bool()?;
            input_info.volume_writable = ts.read_bool()?;
        }

        if protocol_version >= 21 {
            input_info.format = ts.read()?;
        }

        Ok(input_info)
    }
}

impl TagStructWrite for SinkInputInfo {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.index)?;
        w.write_string(Some(&self.name))?;
        w.write_index(self.owner_module_index)?;
        w.write_index(self.client_index)?;
        w.write_u32(self.sink_index)?;
        w.write(self.sample_spec)?;
        w.write(self.channel_map)?;
        w.write(self.cvolume)?;
        w.write_usec(self.buffer_latency)?;
        w.write_usec(self.sink_latency)?;
        w.write_string(self.resample_method.as_ref())?;
        w.write_string(self.driver.as_ref())?;
        w.write_bool(self.muted)?;
        w.write(&self.props)?;

        if protocol_version >= 19 {
            w.write_bool(self.corked)?;
        }

        if protocol_version >= 20 {
            w.write_bool(self.has_volume)?;
            w.write_bool(self.volume_writable)?;
        }

        if protocol_version >= 21 {
            w.write(&self.format)?;
        }

        Ok(())
    }
}

/// The server reply to [`super::Command::GetSinkInputInfoList`].
pub type SinkInputInfoList = Vec<SinkInputInfo>;

impl CommandReply for SinkInputInfoList {}

impl TagStructRead for SinkInputInfoList {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut inputs = Vec::new();
        while ts.has_data_left()? {
            inputs.push(ts.read()?);
        }

        Ok(inputs)
    }
}

impl TagStructWrite for SinkInputInfoList {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        for input in self {
            w.write(input)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn sink_input_info_list_serde() -> anyhow::Result<()> {
        let inputs = vec![
            SinkInputInfo {
                index: 1,
                name: CString::new("input 1")?,
                ..Default::default()
            },
            SinkInputInfo {
                index: 5,
                name: CString::new("input 2")?,
                ..Default::default()
            },
        ];

        test_serde(&inputs)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn get_sink_input_info() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            Command::GetSinkInputInfoList,
            protocol_version,
        )?;

        let (seq, info_list) = read_reply_message::<SinkInputInfoList>(&mut sock)?;
        assert_eq!(seq, 0);

        // The list is often empty.
        if info_list.is_empty() {
            return Ok(());
        }

        write_command_message(
            sock.get_mut(),
            1,
            Command::GetSinkInputInfo(info_list[0].index),
            protocol_version,
        )?;

        let (seq, info) = read_reply_message::<SinkInputInfo>(&mut sock)?;
        assert_eq!(seq, 1);

        assert_eq!(info, info_list[0]);

        Ok(())
    }
}
