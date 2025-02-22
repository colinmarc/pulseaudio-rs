use super::*;

/// Parameters for [`super::Command::UpdateClientProplist`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UpdateClientProplistParams {
    /// The mode of the update.
    pub mode: props::PropsUpdateMode,

    /// The new props.
    pub props: Props,
}

impl TagStructRead for UpdateClientProplistParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            mode: ts.read_enum()?,
            props: ts.read()?,
        })
    }
}

impl TagStructWrite for UpdateClientProplistParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_u32(self.mode as u32)?;
        ts.write(&self.props)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn test_update_client_props_params_serde() -> anyhow::Result<()> {
        let params = UpdateClientProplistParams {
            mode: props::PropsUpdateMode::Merge,
            props: Props::new(),
        };

        test_serde(&params)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use std::ffi::CString;

    use anyhow::Ok;

    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn test_play_sample() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        let mut props = Props::new();
        props.set(Prop::ApplicationName, CString::new("test")?);

        write_command_message(
            sock.get_mut(),
            0,
            Command::UpdateClientProplist(UpdateClientProplistParams {
                mode: props::PropsUpdateMode::Replace,
                props,
            }),
            protocol_version,
        )?;

        read_ack_message(&mut sock)?;
        Ok(())
    }
}
