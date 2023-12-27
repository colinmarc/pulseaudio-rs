use std::ffi::CString;

use crate::protocol::{serde::*, ProtocolError};

use super::CommandReply;

/// Represents a single sample cache entry.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct SampleInfo {
    /// The ID of the sample cache entry.
    pub index: u32,

    /// The name of the sample cache entry.
    pub name: CString,

    /// The default volume of the entry.
    pub cvolume: ChannelVolume,

    /// The format of the entry.
    pub sample_spec: SampleSpec,

    /// The mapping of channels to positions in the entry.
    pub channel_map: ChannelMap,

    /// Duration of the sample, in microseconds.
    pub duration: u64,

    /// The length in bytes.
    pub length: u32,

    /// If set, this points to a file for the sound data to be loaded from on
    /// demand.
    pub lazy: Option<CString>,

    /// Properties of the entry.
    pub props: Props,
}

impl CommandReply for SampleInfo {}

impl TagStructRead for SampleInfo {
    fn read(ts: &mut TagStructReader, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let index = ts.read_u32()?;
        let name = ts
            .read_string()?
            .ok_or_else(|| ProtocolError::Invalid("null sample name".into()))?;
        let cvolume = ts.read()?;
        let duration = ts.read_usec()?;
        let sample_spec = ts.read()?;
        let channel_map = ts.read()?;
        let length = ts.read_u32()?;
        let lazy = ts.read_bool()?;
        let lazy_filename = ts.read_string()?;
        let props = ts.read()?;

        Ok(Self {
            index,
            name,
            cvolume,
            duration,
            sample_spec,
            channel_map,
            length,
            lazy: if lazy { lazy_filename } else { None },
            props,
        })
    }
}

impl TagStructWrite for SampleInfo {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        w.write_u32(self.index)?;
        w.write_string(Some(&self.name))?;
        w.write(&self.cvolume)?;
        w.write_usec(self.duration)?;
        w.write(self.sample_spec)?;
        w.write(&self.channel_map)?;
        w.write_u32(self.length)?;
        w.write_bool(self.lazy.is_some())?;
        w.write_string(self.lazy.as_ref())?;
        w.write(&self.props)?;
        Ok(())
    }
}

pub type SampleInfoList = Vec<SampleInfo>;

impl CommandReply for SampleInfoList {}

impl TagStructRead for SampleInfoList {
    fn read(ts: &mut TagStructReader, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut samples = Vec::new();
        while ts.has_data_left()? {
            samples.push(ts.read()?);
        }
        Ok(samples)
    }
}

impl TagStructWrite for SampleInfoList {
    fn write(&self, w: &mut TagStructWriter, _protocol_version: u16) -> Result<(), ProtocolError> {
        for sample in self {
            w.write(sample)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn sample_info_list_serde() -> anyhow::Result<()> {
        let samples = vec![
            SampleInfo {
                index: 0,
                name: CString::new("test1").unwrap(),
                ..Default::default()
            },
            SampleInfo {
                index: 1,
                name: CString::new("test2").unwrap(),
                ..Default::default()
            },
        ];

        test_serde(&samples)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn get_samples() -> anyhow::Result<()> {
        let mut sock = connect_and_init()?;

        write_command_message(sock.get_mut(), 0, Command::GetSampleInfoList)?;
        let (seq, info_list) = read_reply_message::<SampleInfoList>(&mut sock)?;
        assert_eq!(seq, 0);

        // The list is often empty.
        if info_list.is_empty() {
            return Ok(());
        }

        write_command_message(
            sock.get_mut(),
            1,
            Command::GetSampleInfo(info_list[0].index),
        )?;

        let (seq, info) = read_reply_message::<SampleInfo>(&mut sock)?;
        assert_eq!(seq, 1);

        assert_eq!(info, info_list[0]);

        Ok(())
    }
}
