//! SIGROK-RS

#![warn(missing_docs)]

pub mod config_option;
pub mod device;
pub mod driver;
pub mod input_module;
pub mod output_module;
pub mod packets;
pub mod session;
pub mod trigger;
pub mod types;
pub mod version;

mod utils;

use crate::device::Device;

use crate::types::SrError;

pub use version::*;

/// This macro will try to execute the sigrok function inside an `unsafe{}`
/// statement. If it returns anything other than a SrOk status code, it will
/// return from the function it was called with an `Err(SrError)`.
macro_rules! sr_try {
    ($expr:expr) => {{
        let status = unsafe { $expr };
        if status != SrError::SrOk as i32 {
            return Err(SrError::try_from(status).unwrap());
        }
    }};
}

pub(crate) use sr_try;
