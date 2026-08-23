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
#[derive(Error, Debug)]
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
}

impl TryFrom<i32> for SrError {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Err("SrOk is not an error"),
            -1 => Ok(SrError::SrErr),
            -2 => Ok(SrError::SrErrMalloc),
            -3 => Ok(SrError::SrErrArg),
            -4 => Ok(SrError::SrErrBug),
            -5 => Ok(SrError::SrErrSampleRate),
            -6 => Ok(SrError::SrErrNA),
            -7 => Ok(SrError::SrErrDevClose),
            -8 => Ok(SrError::SrErrTimeout),
            -9 => Ok(SrError::SrErrChannelGroup),
            -10 => Ok(SrError::SrErrData),
            -11 => Ok(SrError::SrErrIO),
            -12 => Ok(SrError::SrDeviceNotFound),
            _ => Err("Unknown SR_ERROR code value"),
        }
    }
}
