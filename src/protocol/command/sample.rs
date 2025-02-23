use super::*;

/// Parameters for [`super::Command::PlaySample`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlaySampleParams {
    /// The index of the sink to play the sample on.
    pub sink_index: Option<u32>,

    /// The name of the sink to play the sample on.
    pub sink_name: Option<CString>,

    /// The volume to play the sample at.
    pub volume: u32,

    /// The name of the sample to play.
    pub name: CString,

    /// Additional properties for the sample.
    pub props: Props,
}

impl TagStructRead for PlaySampleParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            sink_index: ts.read_index()?,
            sink_name: ts.read_string()?,
            volume: ts.read_u32()?,
            name: ts.read_string_non_null()?,
            props: ts.read()?,
        })
    }
}

impl TagStructWrite for PlaySampleParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(self.sink_index)?;
        ts.write_string(self.sink_name.as_ref())?;
        ts.write_u32(self.volume)?;
        ts.write_string(Some(&self.name))?;
        ts.write(&self.props)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use self::test_util::test_serde;

    use super::*;

    #[test]
    fn test_play_sample_params_serde() -> anyhow::Result<()> {
        let params = PlaySampleParams {
            sink_index: None,
            sink_name: Some(CString::new("sink").unwrap()),
            volume: 0,
            name: CString::new("name").unwrap(),
            props: Props::new(),
        };

        test_serde(&params)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use assert_matches::assert_matches;
    use std::ffi::CString;

    use anyhow::Ok;

    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn test_play_sample() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            &Command::PlaySample(PlaySampleParams {
                sink_index: Some(999),
                sink_name: None,
                volume: 0,
                name: CString::new("bell").unwrap(),
                props: Props::new(),
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }
}
