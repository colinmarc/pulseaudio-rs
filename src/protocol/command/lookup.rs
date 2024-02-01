use crate::protocol::{serde::*, ProtocolError};

use super::CommandReply;

/// The server response to [`super::Command::LookupSource`] and [`super::Command::LookupSink`].
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct LookupReply(u32);

impl CommandReply for LookupReply {}

impl TagStructRead for LookupReply {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self(ts.read_u32()?))
    }
}

impl TagStructWrite for LookupReply {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_util::test_serde;

    #[test]
    fn test_lookup_reply_serde() -> anyhow::Result<()> {
        let reply = LookupReply(0);

        test_serde(&reply)
    }
}

#[cfg(test)]
#[cfg(feature = "_integration-tests")]
mod integration_tests {
    use crate::{integration_test_util::connect_and_init, protocol::*};

    #[test]
    fn test_lookup_sink() -> anyhow::Result<()> {
        let (mut sock, protocol_version) = connect_and_init()?;

        write_command_message(
            sock.get_mut(),
            0,
            Command::GetSinkInfoList,
            protocol_version,
        )?;
        let (_, info) = read_reply_message::<SinkInfoList>(&mut sock, protocol_version)?;

        let sink_name = info.last().unwrap().name.clone();

        write_command_message(
            sock.get_mut(),
            1,
            Command::LookupSink(sink_name),
            protocol_version,
        )?;

        let (_, reply) = read_reply_message::<LookupReply>(&mut sock, protocol_version)?;
        assert_eq!(reply.0, info.last().unwrap().index);

        Ok(())
    }
}
