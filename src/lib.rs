// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! # Sigrok-rs
//!
//! Programmatically manage any logic analyzer. Capture samples from your
//! device to files or Rust variables, and automate measuring and testing
//! of electrical signals from real hardware.
//!
//! Provides a Rust-friendly implementation for [libsigrok](https://sigrok.org/wiki/Libsigrok), using the
//! C-FFI [libsigrok-sys](https://crates.io/crates/libsigrok-sys).
//!
//! ## Example
//!
//! The following example connects to the "demo" device, and captures ten
//! samples; storing them in the file "data_capture.txt" and in a `Vec<u8>`.
//!
//! ```rust
//! use sigrok_rs::Session;
//!
//! let mut session: Session = Session::try_from("demo").unwrap();
//! session.set_output("bits", "data_capture.txt").unwrap();
//! let capture_data: Vec<u8> = session.run_samples(10).unwrap();
//!
//! assert!(capture_data.len() == 10);
//! ```
//!
//! If a new device is plugged to the PC and its information is unknown, you
//! can *scan* for connected devices and print their information:
//!
//! ```rust
//! use sigrok_rs::{Session, Device};
//!
//! let mut devices: Vec<Device> = Session::scan().unwrap();
//! for device in &devices {
//!     println!("{}", device);
//! }
//! ```
//!
//! ## Library architecture
//!
//! Any data capture starts by creating a [session], which must always be linked
//! to a [device]. You may modify the session parameters or the device's
//! configuration, and then *run* a data capture session.
//!
#![doc = mermaid!("../docs/architecture.mmd")]
//!
//!

#![warn(missing_docs)]

use simple_mermaid::mermaid;

pub mod device;

pub mod session;
pub mod version;

mod config_option;
mod decoder;
mod driver;
mod input_module;
mod output_module;
mod packets;
mod trigger;
mod types;

pub use crate::decoder::i2c_decoder::{I2CChannels, I2CDecoder, I2COptions};
pub use crate::decoder::jtag_decoder::{
    JTAGChannels, JTAGDecoder, JTAGOptions, JTAGState, JTAGTdiTdo,
};
pub use crate::decoder::spi_decoder::{SPIChannels, SPIDecoder, SPIOptions};
pub use crate::decoder::{DataSample, Decoder};

pub use crate::device::Device;
pub use crate::session::Session;
pub use crate::trigger::TriggerEvent;
pub use crate::types::LogLevel;
pub use crate::types::SrError;

mod utils;

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
