use super::*;

/// Parameters for [`super::Command::LoadModule`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LoadModuleParams {
    /// The name of the module to load.
    pub name: CString,

    /// The arguments to pass to the module.
    pub arguments: Option<CString>,
}

impl TagStructRead for LoadModuleParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            name: ts
                .read_string()?
                .ok_or_else(|| ProtocolError::Invalid("invalid module name".into()))?,
            arguments: ts.read_string()?,
        })
    }
}

impl TagStructWrite for LoadModuleParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_string(Some(&self.name))?;
        ts.write_string(self.arguments.as_ref())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn test_load_module_params_serde() -> anyhow::Result<()> {
        let params = LoadModuleParams {
            name: CString::new("name").unwrap(),
            arguments: Some(CString::new("args").unwrap()),
        };

        test_serde(&params)
    }
}
