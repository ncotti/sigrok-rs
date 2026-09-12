//! Session

use std::ptr::{null, null_mut};

use glib::LogLevel;
use libsigrok_sys::sigrok::{self as sr, sr_datafeed_packet, sr_dev_inst};

use crate::{
    Device, SrError,
    packets::{HeaderPacket, LogicPacket},
    sr_try,
    trigger::{Trigger, TriggerEvent},
    types::PacketType,
};
use sr::{sr_context, sr_session};

/// A Sigrok session
pub struct Session {
    /// Sigrok library context. This is the value returned when calling
    /// `sr_init()` and freed by `sr_exit()`. It is mandatory for the
    /// library to work.
    p_context: *mut sr_context,
    /// Raw C-FFI to the session
    p_session: *mut sr_session,
    /// Device associated with the session. There can only be one device
    /// per session.
    device: Device,
    // input: InputModule,
    // output: OutputModule,
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { sr::sr_session_stop(self.p_session) };
        unsafe { sr::sr_dev_close(self.device.get_pointer()) };
        unsafe { sr::sr_session_dev_remove_all(self.p_session) };
        unsafe { sr::sr_session_destroy(self.p_session) };
        unsafe { sr::sr_exit(self.p_context) };
    }
}

impl TryFrom<&str> for Session {
    type Error = SrError;

    /// Builds a sessions from a device ID, which may be its name, serial
    /// number, driver name, connection ID or model.
    ///
    /// The device will be opened and attached to the session.
    fn try_from(device_id: &str) -> Result<Self, SrError> {
        let mut session = Self::new()?;
        let devices = Device::scan(session.p_context)?;

        let device = devices.into_iter().find(|dev| dev == &device_id);

        if device.is_none() {
            return Err(SrError::SrDeviceNotFound);
        }

        session.device = device.unwrap();
        session.device.open()?;

        sr_try!(sr::sr_session_dev_add(
            session.p_session,
            session.device.get_pointer()
        ));

        Ok(session)
    }
}

impl TryFrom<&String> for Session {
    type Error = SrError;

    fn try_from(device_id: &String) -> Result<Self, SrError> {
        Self::try_from(device_id.as_str())
    }
}

impl TryFrom<String> for Session {
    type Error = SrError;

    fn try_from(device_id: String) -> Result<Self, SrError> {
        Self::try_from(&device_id)
    }
}

impl Session {
    /// Creates a new "empty" session.
    ///
    /// The returned Session object will be plugged to the "demo" device.
    fn new() -> Result<Self, SrError> {
        sr_try!(sr::sr_log_callback_set_default());

        let mut p_context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut p_context));

        let mut p_session: *mut sr_session = null_mut();
        sr_try!(sr::sr_session_new(p_context, &mut p_session));

        let session = Self {
            p_context: p_context,
            p_session: p_session,
            device: Device::try_from(("demo", p_context))?,
        };

        Ok(session)
    }

    /// Sets the log level for the libsigrok functions.
    pub fn set_log_level(&self, level: LogLevel) -> Result<(), SrError> {
        sr_try!(sr::sr_log_loglevel_set(level as i32));
        Ok(())
    }

    /// Returns a list with the driver names of all discovered devices
    /// currently plugged to the PC.
    ///
    /// The driver name can be used to start a new session.
    ///
    /// The device "demo" is always discovered, so the returned vector will
    /// never be empty.
    pub fn scan() -> Result<Vec<String>, SrError> {
        let mut p_context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut p_context));

        let devices = Device::scan(p_context)?;
        let mut names: Vec<String> = Vec::new();
        for device in devices {
            names.push(device.get_driver_name().clone())
        }

        unsafe { sr::sr_exit(p_context) };
        Ok(names)
    }

    pub fn set_trigger(&self, event: TriggerEvent) -> Result<(), SrError> {
        let trigger: Trigger = Trigger::new(
            String::from("name"),
            self.device.get_channel("0").unwrap(),
            event,
        )?;
        sr_try!(sr::sr_session_trigger_set(
            self.p_session,
            trigger.get_pointer()
        ));
        Ok(())
    }

    /// Runs until the trigger or "ms" have passed
    /// Blocking
    pub fn run(&self) -> Result<(), SrError> {
        sr_try!(sr::sr_session_datafeed_callback_add(
            self.p_session,
            Some(Session::my_callback),
            null_mut()
        ));
        sr_try!(sr::sr_session_start(self.p_session));
        sr_try!(sr::sr_session_run(self.p_session));
        // unsafe {
        //     let main_loop = glib::ffi::g_main_loop_new(0x0 as *mut _, 0);
        //     glib::ffi::g_main_loop_run(main_loop);
        // };
        unsafe { assert!(sr::sr_session_is_running(self.p_session) == 1) }
        sr_try!(sr::sr_session_stop(self.p_session));
        Ok(())
    }

    unsafe extern "C" fn my_callback(
        p_device: *const sr_dev_inst,
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
                println!("{}", HeaderPacket::from(packet.payload));
            }
            PacketType::End => {}
            PacketType::Meta => {}
            PacketType::Trigger => {}
            PacketType::Logic => {
                println!("{}", LogicPacket::from(packet.payload));
            }
            PacketType::FrameBegin => {}
            PacketType::FrameEnd => {}
            PacketType::Analog => {}
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_scan() -> Result<(), SrError> {
        let device_names = Session::scan().unwrap();
        let demo_device = device_names.into_iter().find(|dev| dev == "demo").unwrap();

        let _session = Session::try_from(&demo_device)?;

        Ok(())
    }
}
