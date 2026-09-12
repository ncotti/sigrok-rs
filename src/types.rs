//! Rust's structs and enums derived from the primitive types of libsigrok.

use thiserror::Error;

/// Log level
#[derive(Debug, Clone, Copy)]
#[repr(i32)]
#[allow(missing_docs)]
pub enum LogLevel {
    LogNone = 0,
    LogErr = 1,
    LogWarn = 2,
    LogInfo = 3,
    LogDbg = 4,
    LogSpew = 5,
}

/// Sigrok error codes
#[derive(Error, Debug, PartialEq)]
#[repr(i32)]
#[allow(missing_docs)]
pub enum SrError {
    #[error("No error.")]
    SrOk = 0,
    #[error("Generic/unspecified error.")]
    SrErr = -1,
    #[error("Malloc/calloc/realloc error.")]
    SrErrMalloc = -2,
    #[error("Function argument error.")]
    SrErrArg = -3,
    #[error("Errors hinting at internal bugs.")]
    SrErrBug = -4,
    #[error("Incorrect samplerate.")]
    SrErrSampleRate = -5,
    #[error("Not applicable.")]
    SrErrNA = -6,
    #[error("Device is closed, but must be open.")]
    SrErrDevClose = -7,
    #[error("A timeout occurred.")]
    SrErrTimeout = -8,
    #[error("A channel group must be specified.")]
    SrErrChannelGroup = -9,
    #[error("Data is invalid.")]
    SrErrData = -10,
    #[error("Input/output error.")]
    SrErrIO = -11,
    // From here on, these are custom error codes
    #[error("Device not found")]
    SrDeviceNotFound = -12,
    #[error("Pointer was NULL")]
    SrNull = -13,
    #[error("Configuration option does not exist")]
    SrOptionNotExist = -14,
    #[error("Invalid value for configuration option")]
    SrInvalidOptionValue = -15,
    #[error("Channel name of index not found")]
    SrChannelNotFound = -16,
}

impl From<i32> for SrError {
    fn from(value: i32) -> Self {
        match value {
            0 => SrError::SrOk,
            -1 => SrError::SrErr,
            -2 => SrError::SrErrMalloc,
            -3 => SrError::SrErrArg,
            -4 => SrError::SrErrBug,
            -5 => SrError::SrErrSampleRate,
            -6 => SrError::SrErrNA,
            -7 => SrError::SrErrDevClose,
            -8 => SrError::SrErrTimeout,
            -9 => SrError::SrErrChannelGroup,
            -10 => SrError::SrErrData,
            -11 => SrError::SrErrIO,
            -12 => SrError::SrDeviceNotFound,
            -13 => SrError::SrNull,
            -14 => SrError::SrOptionNotExist,
            -15 => SrError::SrInvalidOptionValue,
            -16 => SrError::SrChannelNotFound,
            _ => SrError::SrErrNA,
        }
    }
}

/// Device type
/// TODO, currently not used
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum DeviceType {
    /// The device can act as logic analyzer.
    LogicAnalyzer = 10000,
    /// The device can act as an oscilloscope.
    Oscilloscope = 10001,
    /// The device can act as a multimeter.
    Multimeter = 10002,
    /// The device is a demo device.
    DemoDev = 10003,
    /// The device can act as a sound level meter.
    SoundLevelMeter = 10004,
    /// The device can measure temperature.
    Thermometer = 10005,
    /// The device can measure humidity.
    Hygrometer = 10006,
    /// The device can measure energy consumption.
    EnergyMeter = 10007,
    /// The device can act as a signal demodulator.
    Demodulator = 10008,
    /// The device can act as a programmable power supply.
    PowerSupply = 10009,
    /// The device can act as an LCR meter.
    LCRMeter = 10010,
    /// The device can act as an electronic load.
    ElectronicLoad = 10011,
    /// The device can act as a scale.
    Scale = 10012,
    /// The device can act as a function generator.
    SignalGenerator = 10013,
    /// The device can measure power.
    PowerMeter = 10014,
}

/// Packet Type
#[derive(Debug, Clone, Copy)]
pub enum PacketType {
    /// Payload is sr_datafeed_header.
    Header = 10000,
    /// End of stream (no further data).
    End = 10001,
    /// Payload is struct sr_datafeed_meta
    Meta = 10002,
    /// The trigger matched at this point in the data feed. No payload.
    Trigger = 10003,
    /// Payload is struct sr_datafeed_logic.
    Logic = 10004,
    /// Beginning of frame. No payload.
    FrameBegin = 10005,
    /// End of frame. No payload.
    FrameEnd = 10006,
    /// Payload is struct sr_datafeed_analog.
    Analog = 10007,
}

impl TryFrom<u16> for PacketType {
    type Error = SrError;

    fn try_from(value: u16) -> Result<Self, SrError> {
        match value {
            10000 => Ok(PacketType::Header),
            10001 => Ok(PacketType::End),
            10002 => Ok(PacketType::Meta),
            10003 => Ok(PacketType::Trigger),
            10004 => Ok(PacketType::Logic),
            10005 => Ok(PacketType::FrameBegin),
            10006 => Ok(PacketType::FrameEnd),
            10007 => Ok(PacketType::Analog),
            _ => Err(SrError::SrErrNA),
        }
    }
}
