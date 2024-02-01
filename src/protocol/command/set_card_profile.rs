use std::ffi::CString;

use super::*;

/// Parameters for [`super::Command::SetCardProfile`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SetCardProfileParams {
    /// The index of the card.
    pub card_index: Option<u32>,

    /// The name of the card.
    pub card_name: Option<CString>,

    /// The name of the profile to set.
    pub profile_name: CString,
}

impl TagStructRead for SetCardProfileParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            card_index: ts.read_index()?,
            card_name: ts.read_string()?,
            profile_name: ts
                .read_string()?
                .ok_or_else(|| ProtocolError::Invalid("invalid profile name".into()))?,
        })
    }
}

impl TagStructWrite for SetCardProfileParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(self.card_index)?;
        ts.write_string(self.card_name.as_ref())?;
        ts.write_string(Some(&self.profile_name))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn test_set_card_profile_params_serde() -> anyhow::Result<()> {
        let params = SetCardProfileParams {
            card_index: None,
            card_name: Some(CString::new("card").unwrap()),
            profile_name: CString::new("profile").unwrap(),
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
    fn test_set_card_profile() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            Command::SetCardProfile(SetCardProfileParams {
                card_index: Some(999),
                card_name: None,
                profile_name: CString::new("profile").unwrap(),
            }),
            protocol_version,
        )?;

        let resp = read_ack_message(&mut sock);

        assert_matches!(resp, Err(ProtocolError::ServerError(PulseError::NoEntity)));

        Ok(())
    }
}
