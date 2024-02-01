use super::*;

/// A generic client event with a name and proplist.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ClientEvent {
    /// The name of the event.
    pub name: CString,

    /// The properties of the event.
    pub props: Props,
}

impl TagStructRead for ClientEvent {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            name: ts.read_string_non_null()?,
            props: ts.read()?,
        })
    }
}

impl TagStructWrite for ClientEvent {
    fn write(
        &self,
        ts: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        ts.write_string(Some(&self.name))?;
        ts.write(&self.props)?;
        Ok(())
    }
}
