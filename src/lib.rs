//! SIGROK-RS

#![warn(missing_docs)]

pub mod device;
pub mod types;
pub mod version;

use crate::device::{Device, Driver};
use crate::types::{LogLevel, SrError};

use std::ptr::null_mut;

use libsigrok_sys::sigrok as sr;
use libsigrok_sys::sigrok::GSList;
use libsigrok_sys::sigrok::sr_context;
use libsigrok_sys::sigrok::sr_dev_driver;
use libsigrok_sys::sigrok::sr_dev_inst;
use libsigrok_sys::sigrok::sr_session;
use std::mem;
pub use version::*;

use glib;

/// Logic analyzer session.
pub struct Session {
    context: *mut sr_context,
    session: *mut sr_session,
    device: Device,
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { sr::sr_session_destroy(self.session) };
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
        Ok(session)
    }
}

impl Session {
    /// Creates a new "empty" session.
    ///
    /// The returned Session object does not have any device attached yet, so
    /// its device pointer is NULL. Therefore, it is the caller's
    /// responsibility to scan for devices an attach one.
    pub fn new() -> Result<Self, SrError> {
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

    /// Returns the list of available drivers for all devices that could be
    /// recognized.
    fn driver_list(&self) -> Vec<Driver> {
        let mut drivers: Vec<Driver> = Vec::new();

        let mut p_p_drivers: *mut *mut sr_dev_driver = unsafe { sr::sr_driver_list(self.context) };
        let mut p_driver: *mut sr_dev_driver = unsafe { *p_p_drivers };

        while p_driver != null_mut() {
            drivers.push(Driver::new(p_driver));

            // Point to the next driver by moving the numerical value of the
            // memory address
            p_p_drivers = ((p_p_drivers as usize) + mem::size_of::<*mut sr_dev_driver>())
                as *mut *mut sr_dev_driver;
            p_driver = unsafe { *p_p_drivers };
        }

        drivers
    }

    /// Returns a list of all discovered devices currently plugged to the PC.
    ///
    /// The device "demo" is always discovered, so the returned vector will
    /// never be empty.
    pub fn scan(&mut self) -> Result<Vec<Device>, SrError> {
        // For each driver, scan if there are any devices connected
        let mut devices: Vec<Device> = Vec::new();
        for driver in self.driver_list() {
            sr_try!(sr::sr_driver_init(self.context, driver.get_driver()));

            let device_list: *mut GSList =
                unsafe { sr::sr_driver_scan(driver.get_driver(), 0x00 as *mut GSList) };
            let mut device_node: *mut GSList = device_list;

            while device_node != null_mut() {
                let p_device: *mut sr_dev_inst = unsafe { *device_node }.data.cast();

                devices.push(Device::new(p_device, driver.clone()));
                device_node = unsafe { *device_node }.next;
            }

            unsafe { glib::ffi::g_slist_free(device_list.cast()) };
        }

        Ok(devices)
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
