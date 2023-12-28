use std::time::SystemTime;

use crate::protocol::{serde::*, ProtocolError};

use super::CommandReply;

/// Parameters for [`super::Command::GetPlaybackLatency`] and [`super::Command::GetRecordLatency`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatencyParams {
    /// The channel to get latency for.
    pub channel: u32,
    /// The local system time.
    pub now: SystemTime,
}

impl TagStructRead for LatencyParams {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            channel: ts.read_u32()?,
            now: ts.read_timeval()?,
        })
    }
}

impl TagStructWrite for LatencyParams {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_u32(self.channel)?;
        w.write_timeval(self.now)?;
        Ok(())
    }
}

/// The server reply to [`super::Command::GetPlaybackLatency`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackLatency {
    /// The latency of the sink.
    pub sink_usec: u64,

    /// The latency of the source.
    pub source_usec: u64,

    /// Whether the stream is currently playing.
    pub playing: bool,

    /// The local system time.
    pub local_time: SystemTime,

    /// The remote system time.
    pub remote_time: SystemTime,

    /// The client's offset in the shared buffer.
    pub write_offset: i64,

    /// The sink's offset in the shared buffer.
    pub read_offset: i64,

    /// The number of bytes the sink has underrun for.
    pub underrun_for: u64,

    /// The number of bytes the sink has been playing for.
    pub playing_for: u64,
}

impl CommandReply for PlaybackLatency {}

impl TagStructRead for PlaybackLatency {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            sink_usec: ts.read_usec()?,
            source_usec: ts.read_usec()?,
            playing: ts.read_bool()?,
            local_time: ts.read_timeval()?,
            remote_time: ts.read_timeval()?,
            write_offset: ts.read_i64()?,
            read_offset: ts.read_i64()?,
            underrun_for: ts.read_u64()?,
            playing_for: ts.read_u64()?,
        })
    }
}

impl TagStructWrite for PlaybackLatency {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_usec(self.sink_usec)?;
        w.write_usec(self.source_usec)?;
        w.write_bool(self.playing)?;
        w.write_timeval(self.local_time)?;
        w.write_timeval(self.remote_time)?;
        w.write_i64(self.write_offset)?;
        w.write_i64(self.read_offset)?;
        Ok(())
    }
}

/// The server reply to [`super::Command::GetRecordLatency`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordLatency {
    /// The latency of the sink.
    pub sink_usec: u64,

    /// The latency of the source.
    pub source_usec: u64,

    /// Whether the stream is currently running.
    pub playing: bool,

    /// The local system time.
    pub local_time: SystemTime,

    /// The remote system time.
    pub remote_time: SystemTime,

    /// The client's offset in the shared buffer.
    pub write_offset: i64,

    /// The sink's offset in the shared buffer.
    pub read_offset: i64,
}

impl CommandReply for RecordLatency {}

impl TagStructRead for RecordLatency {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        Ok(Self {
            sink_usec: ts.read_usec()?,
            source_usec: ts.read_usec()?,
            playing: ts.read_bool()?,
            local_time: ts.read_timeval()?,
            remote_time: ts.read_timeval()?,
            write_offset: ts.read_i64()?,
            read_offset: ts.read_i64()?,
        })
    }
}

impl TagStructWrite for RecordLatency {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.write_usec(self.sink_usec)?;
        w.write_usec(self.source_usec)?;
        w.write_bool(self.playing)?;
        w.write_timeval(self.local_time)?;
        w.write_timeval(self.remote_time)?;
        w.write_i64(self.write_offset)?;
        w.write_i64(self.read_offset)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{test_util::test_serde_version, MAX_VERSION};

    #[test]
    fn playback_timing_info_serde() -> anyhow::Result<()> {
        let timing_info = PlaybackLatency {
            sink_usec: 1,
            source_usec: 2,
            playing: true,
            local_time: SystemTime::UNIX_EPOCH,
            remote_time: SystemTime::UNIX_EPOCH,
            write_offset: 3,
            read_offset: 4,
            underrun_for: 5,
            playing_for: 6,
        };

        test_serde_version(&timing_info, MAX_VERSION)
    }

    #[test]
    fn record_timing_info_serde() -> anyhow::Result<()> {
        let timing_info = RecordLatency {
            sink_usec: 1,
            source_usec: 2,
            playing: true,
            local_time: SystemTime::UNIX_EPOCH,
            remote_time: SystemTime::UNIX_EPOCH,
            write_offset: 3,
            read_offset: 4,
        };

        test_serde_version(&timing_info, MAX_VERSION)
    }
}
