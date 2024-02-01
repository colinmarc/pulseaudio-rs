use std::ffi::CString;

use super::*;

/// Parameters for [`super::Command::Extension`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExtensionParams {
    /// The index of the module.
    pub index: Option<u32>,

    /// The name of the module.
    pub name: Option<CString>,
}

impl TagStructRead for ExtensionParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts.read_index()?,
            name: ts.read_string()?,
        })
    }
}

impl TagStructWrite for ExtensionParams {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_index(self.index)?;
        ts.write_string(self.name.as_ref())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn test_extension_params_serde() -> anyhow::Result<()> {
        let params = ExtensionParams {
            index: None,
            name: Some(CString::new("name").unwrap()),
        };

        test_serde(&params)
    }
}
