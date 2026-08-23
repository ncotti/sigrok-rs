//! Device representation

use libsigrok_sys::sigrok::sr_dev_driver;
use libsigrok_sys::sigrok::sr_dev_inst;
use std::fmt;

use std::ffi::CStr;
use std::ptr::null_mut;

use libsigrok_sys::sigrok as sr;

/// This struct represents any device recognizable by libsigrok.
#[derive(Debug, Default)]
pub struct Device {
    /// Driver used to communicate with the device.
    driver: Driver,
    /// Raw C FFI pointer to the device instance.
    p_device: *mut sr_dev_inst,
    /// Vendor string, if any, or "".
    vendor: String,
    /// Model string, if any, or "".
    model: String,
    /// Version string, if any, or "".
    version: String,
    /// Serial number, if any, or "".
    serial_number: String,
    /// Connection ID, if any, or "".
    connection_id: String,
}

impl Device {
    /// Given a non-null pointer to a device and its driver, this function
    /// scouts for the device's related information and fills the structure.
    ///
    /// This function will panic! if `p_device` is NULL.
    pub fn new(p_device: *mut sr_dev_inst, driver: Driver) -> Self {
        let vendor = unsafe { sr::sr_dev_inst_vendor_get(p_device) };
        let vendor: String = if vendor == null_mut() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(vendor) }
                .to_string_lossy()
                .to_string()
        };

        let model = unsafe { sr::sr_dev_inst_model_get(p_device) };
        let model: String = if model == null_mut() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(model) }
                .to_string_lossy()
                .to_string()
        };

        let version = unsafe { sr::sr_dev_inst_version_get(p_device) };
        let version: String = if version == null_mut() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(version) }
                .to_string_lossy()
                .to_string()
        };

        let serial_number = unsafe { sr::sr_dev_inst_sernum_get(p_device) };
        let serial_number: String = if serial_number == null_mut() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(serial_number) }
                .to_string_lossy()
                .to_string()
        };

        let connection_id = unsafe { sr::sr_dev_inst_connid_get(p_device) };
        let connection_id: String = if connection_id == null_mut() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(connection_id) }
                .to_string_lossy()
                .to_string()
        };

        Device {
            driver: driver,
            p_device: p_device,
            vendor: vendor,
            model: model,
            version: version,
            serial_number: serial_number,
            connection_id: connection_id,
        }
    }

    /// Returns the vendor string.
    ///
    /// If it could be detected, it returns an empty string "".
    pub fn get_vendor(&self) -> &String {
        &self.vendor
    }

    /// Returns the model string.
    ///
    /// If it could be detected, it returns an empty string "".
    pub fn get_model(&self) -> &String {
        &self.model
    }

    /// Returns the version string.
    ///
    /// If it could be detected, it returns an empty string "".
    pub fn get_version(&self) -> &String {
        &self.version
    }

    /// Returns the serial string.
    ///
    /// If it could be detected, it returns an empty string "".
    pub fn get_serial_number(&self) -> &String {
        &self.serial_number
    }

    /// Returns the connection ID string.
    ///
    /// If it could be detected, it returns an empty string "".
    pub fn get_connection_id(&self) -> &String {
        &self.connection_id
    }

    /// Returns the pointer to the device structure.
    pub fn get_device(&self) -> *mut sr_dev_inst {
        self.p_device
    }

    /// Compares the given value with the device's vendor, model, version,
    /// serial number and connection ID, and also its driver name.
    ///
    /// Returns `true` if any of them match.
    pub fn find(&self, value: impl AsRef<str>) -> bool {
        let value: &str = value.as_ref();

        if value.is_empty() {
            return false;
        }

        (value == self.vendor)
            || (value == self.model)
            || (value == self.version)
            || (value == self.serial_number)
            || (value == self.connection_id)
            || (value == self.driver.name)
            || (value == self.driver.long_name)
    }
}

/// Sigrok's drivers for different types of devices.
///
/// Before being able to connect to any device, a driver structure must
/// be created and used to search for the device. The idea is that only a
/// matching pair of (driver, device) can communicate with each other.
#[derive(Clone, Default)]
pub struct Driver {
    /// Raw C pinter to the device driver.
    p_driver: *mut sr_dev_driver,
    /// Driver's name.
    name: String,
    /// Driver's long name.
    long_name: String,
}

impl fmt::Debug for Driver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Driver")
            .field("name", &self.name)
            .field("longname", &self.long_name)
            .finish()
    }
}

impl Driver {
    /// Creates a new Driver struct from a FFI C `sr_dev_driver` pointer.
    ///
    /// This function will panic! if the argument, `p_driver`, is NULL.
    pub fn new(p_driver: *mut sr_dev_driver) -> Self {
        if p_driver == null_mut() {
            panic!("Driver::new(), p_driver argument is a NULL pointer.");
        }

        let driver: sr_dev_driver = unsafe { *p_driver };
        Self {
            p_driver: p_driver,
            name: unsafe { CStr::from_ptr(driver.name) }
                .to_string_lossy()
                .to_string(),
            long_name: unsafe { CStr::from_ptr(driver.longname) }
                .to_string_lossy()
                .to_string(),
        }
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
    pub fn get_driver(&self) -> *mut sr_dev_driver {
        self.p_driver
    }
}
