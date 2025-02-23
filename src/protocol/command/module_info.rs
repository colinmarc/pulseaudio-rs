use std::ffi::CString;

use crate::protocol::{serde::*, ProtocolError};

use super::CommandReply;

/// Server state for a module, in response to [`super::Command::GetModuleInfo`].
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct ModuleInfo {
    /// ID of the module.
    pub index: u32,

    /// The name of the module.
    pub name: CString,

    /// Argument string passed to the module.
    pub argument: Option<CString>,

    /// Usage counter.
    pub n_used: Option<u32>,

    /// Whether the module is automatically unloaded when unused. Deprecated.
    #[deprecated]
    pub auto_unload: bool,

    /// Module properties.
    pub props: Props,
}

impl CommandReply for ModuleInfo {}

impl TagStructRead for ModuleInfo {
    #[allow(deprecated)]
    fn read(ts: &mut TagStructReader<'_>, protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            index: ts.read_u32()?,
            name: ts.read_string_non_null()?,
            argument: ts.read_string()?,
            n_used: ts.read_index()?,
            auto_unload: if protocol_version < 15 {
                ts.read_bool()?
            } else {
                false
            },
            props: if protocol_version >= 15 {
                ts.read()?
            } else {
                Default::default()
            },
        })
    }
}

impl TagStructWrite for ModuleInfo {
    #[allow(deprecated)]
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.index)?;
        w.write_string(Some(&self.name))?;
        w.write_string(self.argument.as_ref())?;
        w.write_index(self.n_used)?;

        if protocol_version < 15 {
            w.write_bool(self.auto_unload)?;
        } else {
            w.write(&self.props)?;
        }

        Ok(())
    }
}

/// The server reply to [`super::Command::GetModuleInfoList`].
pub type ModuleInfoList = Vec<ModuleInfo>;

impl CommandReply for ModuleInfoList {}

impl TagStructRead for ModuleInfoList {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        let mut modules = Vec::new();
        while ts.has_data_left()? {
            modules.push(ts.read()?);
        }
        Ok(modules)
    }
}

impl TagStructWrite for ModuleInfoList {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        for module in self {
            w.write(module)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::serde::test_util::test_serde;

    use super::*;

    #[test]
    fn module_info_list_serde() -> anyhow::Result<()> {
        let modules = vec![
            ModuleInfo {
                index: 0,
                name: CString::new("test").unwrap(),
                ..Default::default()
            },
            ModuleInfo {
                index: 1,
                name: CString::new("test2").unwrap(),
                ..Default::default()
            },
        ];

        test_serde(&modules)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use super::*;
    use crate::{
        integration_test_util::connect_and_init,
        protocol::{read_reply_message, write_command_message, Command},
    };

    use pretty_assertions::assert_eq;

    #[test]
    fn list_modules() -> Result<(), Box<dyn std::error::Error>> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            &Command::GetModuleInfoList,
            protocol_version,
        )?;

        let (seq, info_list) = read_reply_message::<ModuleInfoList>(&mut sock, protocol_version)?;
        assert_eq!(seq, 0);
        assert!(!info_list.is_empty());

        write_command_message(
            sock.get_mut(),
            1,
            &Command::GetModuleInfo(info_list[0].index),
            protocol_version,
        )?;

        let (seq, info) = read_reply_message::<ModuleInfo>(&mut sock, protocol_version)?;
        assert_eq!(seq, 1);

        assert_eq!(info, info_list[0]);

        Ok(())
    }
}
