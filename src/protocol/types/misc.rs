/*
/// Direction bitfield - while we currently do not expose anything bidirectional,
/// one should test against the bit instead of the value (e.g.\ `if (d & PA_DIRECTION_OUTPUT)`),
/// because we might add bidirectional stuff in the future. \since 2.0
pub enum Direction {
    /// Output direction
    Output = 0x01,
    /// Input direction
    Input = 0x02,
    /// Both input and output.
    Bidirectional = 0x03,
}

/// The type of device we are dealing with.
pub enum DeviceType {
    /// Playback device.
    Sink,
    /// Recording device.
    Source,
}
*/
