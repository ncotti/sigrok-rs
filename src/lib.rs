//! SIGROK-RS

#![warn(missing_docs)]

pub mod device;
pub mod driver;
pub mod input_module;
pub mod output_module;
pub mod session;
pub mod trigger;
pub mod types;
pub mod version;

mod utils;

use crate::device::Device;

use crate::types::SrError;

use std::os::raw::c_void;
use std::time::Duration;

use libsigrok_sys::sigrok::{self as sr};
pub use version::*;

pub struct HeaderPacket {
    feed_version: u32,
    time: Duration,
}

impl From<*const c_void> for HeaderPacket {
    fn from(payload: *const c_void) -> Self {
        let p_feed_version: *const u32 = payload.cast();
        let feed_version: u32 = unsafe { p_feed_version.read() };

        let p_time_seconds: *const u32 = unsafe { payload.byte_offset(4).cast() };
        let time_seconds: u64 = unsafe { p_time_seconds.read() } as u64;

        let p_time_us: *const u32 = unsafe { payload.byte_offset(8).cast() };
        let time_us: u64 = unsafe { p_time_us.read() } as u64;

        dbg!(feed_version);
        dbg!(time_seconds);
        dbg!(time_us);
        HeaderPacket {
            feed_version: feed_version,
            time: Duration::from_micros(time_us + time_seconds * 1_000_000),
        }
    }
}

pub struct MetaPacket {}

/// Given "N" logic channels, where `1 <= N <= 16`, the `data` vector holds in
/// each bit the logical state of each channel, ordered by channel index,
/// as such:
///
/// D15 D14 D13 D12 D11 D10 D9 D8 D7 D6 D5 D4 D3 D2 D1 D0
///              CH7 CH6 CH5 CH4 CH3 CH2 CH1 CH0
// 0x0E          0   0   0   0   1   1   1   0
// 0xF6          1   1   1   1   0   1   1   0
// 0xD2          1   1   0   1   0   0   1   0
pub struct LogicPacket {
    data: Vec<u16>,
}

impl From<*const c_void> for LogicPacket {
    fn from(payload: *const c_void) -> Self {
        // let p_length: *const u64 = payload.cast();
        // let length: u64 = unsafe{p_length.read()};

        // let p_unit_size: *const u16 = unsafe{payload.byte_offset(8).cast()};
        // let unit_size: u16 = unsafe{p_unit_size.read()};

        // let p_p_data: *const *const u8 = unsafe{payload.byte_offset(10).cast()};
        // dbg!(p_p_data);
        // let mut p_data: *const u8 = unsafe{*p_p_data};

        let logic = unsafe { &*(payload as *const sr::sr_datafeed_logic) };

        let length = logic.length;
        let unit_size = logic.unitsize;
        let mut p_data = logic.data as *const u8;

        let mut data: Vec<u16> = Vec::new();

        if unit_size != 1 && unit_size != 2 {
            // TODO
            panic!("Unit size wrong value");
        }

        for _i in 0..length {
            let channel_values: u16 = if unit_size == 1 {
                (unsafe { p_data.read() } as u16)
            } else {
                ((unsafe { p_data.offset(1).read() } as u16) << 8)
                    | (unsafe { p_data.read() } as u16)
            };
            data.push(channel_values);
            p_data = unsafe { p_data.byte_offset(unit_size as isize) };
        }

        dbg!(length);
        dbg!(unit_size);
        dbg!(data[0]);
        dbg!(data[1]);
        dbg!(data[2]);

        LogicPacket { data: data }
    }
}

pub struct AnalogPacket {}

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
