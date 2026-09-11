//! Session

use std::ptr::{null, null_mut};

use glib::LogLevel;
use libsigrok_sys::sigrok::{self as sr, sr_datafeed_packet, sr_dev_inst};

use crate::{
    Device, HeaderPacket, LogicPacket, SrError, sr_try,
    trigger::{Trigger, TriggerEvent},
    types::PacketType,
};
use sr::{sr_context, sr_session};

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
            device: Device::try_from(("demo", context))?,
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
            self.device.get_channel("0").unwrap(),
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
    pub fn run(&self) -> Result<(), SrError> {
        sr_try!(sr::sr_session_datafeed_callback_add(
            self.session,
            Some(Session::my_callback),
            null_mut()
        ));
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
