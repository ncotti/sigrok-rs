//! SIGROK-RS

#![warn(missing_docs)]

pub mod device;
pub mod driver;
pub mod input_module;
pub mod output_module;
pub mod trigger;
pub mod types;
pub mod version;

mod utils;

use crate::device::Device;
use crate::driver::Driver;
use crate::input_module::InputModule;
use crate::output_module::OutputModule;
use crate::trigger::{Trigger, TriggerEvent};
use crate::types::{LogLevel, SrError};

use std::os::raw::c_void;
use std::ptr::{null, null_mut};
use std::thread::{sleep, sleep_ms};
use std::time::Duration;

use libsigrok_sys::sigrok::GSList;
use libsigrok_sys::sigrok::sr_context;
use libsigrok_sys::sigrok::sr_dev_driver;
use libsigrok_sys::sigrok::sr_dev_inst;
use libsigrok_sys::sigrok::sr_session;
use libsigrok_sys::sigrok::{self as sr, sr_datafeed_packet};
use std::mem;
pub use version::*;

use glib;

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

/// Logic analyzer session.
pub struct Session {
    context: *mut sr_context,
    session: *mut sr_session,
    device: Device,
    // input: InputModule,
    // output: OutputModule,
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { sr::sr_session_stop(self.session) };
        unsafe { sr::sr_dev_close(self.device.get_pointer()) };
        unsafe { sr::sr_session_dev_remove_all(self.session) };
        unsafe { sr::sr_session_destroy(self.session) };
        // TODO, this line generates a Segmentation Fault
        unsafe { sr::sr_exit(self.context) };
    }
}

impl TryFrom<&str> for Session {
    type Error = SrError;

    fn try_from(value: &str) -> Result<Self, SrError> {
        let mut session = Self::new()?;
        let devices = session.scan()?;

        dbg!(&devices);

        let device = devices.into_iter().find(|dev| dev.find(value));

        if device.is_none() {
            return Err(SrError::SrDeviceNotFound);
        }

        session.device = device.unwrap();

        sr_try!(sr::sr_dev_open(session.device.get_pointer()));
        sr_try!(sr::sr_session_dev_add(
            session.session,
            session.device.get_pointer()
        ));

        Ok(session)
    }
}

impl Session {
    /// Creates a new "empty" session.
    ///
    /// The returned Session object does not have any device attached yet, so
    /// its device pointer is NULL. Therefore, it is the caller's
    /// responsibility to scan for devices an attach one.
    fn new() -> Result<Self, SrError> {
        sr_try!(sr::sr_log_callback_set_default());

        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let mut session: *mut sr_session = null_mut();
        sr_try!(sr::sr_session_new(context, &mut session));

        let session = Self {
            context: context,
            session: session,
            device: Device::default(),
        };

        Ok(session)
    }

    /// Sets the log level for the libsigrok functions.
    pub fn set_log_level(&self, level: LogLevel) -> Result<(), SrError> {
        sr_try!(sr::sr_log_loglevel_set(level as i32));
        Ok(())
    }

    /// Returns a list of all discovered devices currently plugged to the PC.
    ///
    /// The device "demo" is always discovered, so the returned vector will
    /// never be empty.
    pub fn scan(&self) -> Result<Vec<Device>, SrError> {
        Ok(Device::scan(self.context)?)
    }

    pub fn set_trigger(&self, event: TriggerEvent) -> Result<(), SrError> {
        let trigger: Trigger = Trigger::new(
            String::from("name"),
            self.device.get_channel_by_index(0).unwrap(),
            event,
        )?;
        sr_try!(sr::sr_session_trigger_set(
            self.session,
            trigger.get_pointer()
        ));
        Ok(())
    }

    /// Runs until the trigger or "ms" have passed
    /// Blocking
    pub fn run(&self, timeout_ms: u64) -> Result<(), SrError> {
        sr_try!(sr::sr_session_datafeed_callback_add(
            self.session,
            Some(Session::my_callback),
            null_mut()
        ));
        self.device.open().unwrap_or_else(|e| {});
        sr_try!(sr::sr_session_start(self.session));
        unsafe {
            let main_loop = glib::ffi::g_main_loop_new(0x0 as *mut _, 0);
            glib::ffi::g_main_loop_run(main_loop);
        };
        unsafe { assert!(sr::sr_session_is_running(self.session) == 1) }
        sr_try!(sr::sr_session_stop(self.session));
        Ok(())
    }

    unsafe extern "C" fn my_callback(
        sdi: *const sr_dev_inst,
        packet: *const sr_datafeed_packet,
        cb_data: *mut std::ffi::c_void,
    ) {
        if packet == null() {
            return;
        }

        let packet = unsafe { *packet };
        let packet_type = PacketType::try_from(packet.type_).unwrap();

        match packet_type {
            PacketType::Header => {
                let p = HeaderPacket::from(packet.payload);
            }
            PacketType::End => {}
            PacketType::Meta => {}
            PacketType::Trigger => {}
            PacketType::Logic => {
                let p = LogicPacket::from(packet.payload);
            }
            PacketType::FrameBegin => {}
            PacketType::FrameEnd => {}
            PacketType::Analog => {}
        };
    }
}

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
