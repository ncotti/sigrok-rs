//! Drivers

use libsigrok_sys::sigrok as sr;

use sr::{sr_context, sr_dev_driver};

use std::ffi::CStr;
use std::ptr::{null, null_mut};

use crate::types::SrError;

/// Sigrok's drivers for all devices.
///
/// Before being able to connect to any device, a driver structure must
/// be created and used to search for the device. Only a
/// matching pair of (driver, device) can communicate with each other.
/// TODO, remove "Default"
#[derive(Clone, Default, Debug)]
pub struct Driver {
    /// Raw C pinter to the device driver.
    p_driver: *mut sr_dev_driver,
    /// Driver's name.
    name: String,
    /// Driver's long name.
    long_name: String,
}

impl TryFrom<*mut sr_dev_driver> for Driver {
    type Error = SrError;

    /// Creates a new Driver struct from a FFI C `sr_dev_driver` pointer.
    fn try_from(p_driver: *mut sr_dev_driver) -> Result<Self, SrError> {
        if p_driver == null_mut() {
            return Err(SrError::SrNull);
        }

        let driver: sr_dev_driver = unsafe { *p_driver };
        Ok(Self {
            p_driver: p_driver,
            name: unsafe { CStr::from_ptr(driver.name) }
                .to_string_lossy()
                .to_string(),
            long_name: unsafe { CStr::from_ptr(driver.longname) }
                .to_string_lossy()
                .to_string(),
        })
    }
}

impl Driver {
    /// Returns the list of available drivers for any device.
    ///
    /// * `context`: A sigrok session context. A `Session` struct must first be
    /// created, and then use its `Session.get_context()` as argument.
    pub fn list(context: *const sr_context) -> Result<Vec<Driver>, SrError> {
        if context == null() {
            return Err(SrError::SrNull);
        }

        let mut drivers: Vec<Driver> = Vec::new();

        let mut p_p_drivers: *mut *mut sr_dev_driver = unsafe { sr::sr_driver_list(context) };
        let mut p_driver: *mut sr_dev_driver = unsafe { *p_p_drivers };

        while p_driver != null_mut() {
            drivers.push(Driver::try_from(p_driver)?);
            p_p_drivers = unsafe { p_p_drivers.offset(1) };
            p_driver = unsafe { *p_p_drivers };
        }

        Ok(drivers)
    }

    /// Returns the driver's name.
    pub fn get_name(&self) -> &String {
        &self.name
    }

    /// Returns the driver's long name.
    pub fn get_long_name(&self) -> &String {
        &self.long_name
    }

    /// Returns the value of the FFI device pointer.
    pub fn get_pointer(&self) -> *mut sr_dev_driver {
        self.p_driver
    }
}

mod tests {
    use super::*;
    use crate::{driver, sr_try, types::SrError};

    #[test]
    fn test_list_drivers() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let drivers: Vec<Driver> = Driver::list(context)?;

        let demo_driver = drivers
            .into_iter()
            .find(|driver| driver.get_name() == "demo")
            .expect("Demo driver should always be found.");
        assert!(demo_driver.get_name() == "demo");
        assert!(demo_driver.get_long_name() == "Demo driver and pattern generator");

        sr_try!(sr::sr_exit(context));
        Ok(())
    }
}
