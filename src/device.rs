//! Device representation

use libsigrok_sys::sigrok as sr;

use crate::driver::Driver;
use sr::sr_dev_inst;

use std::ffi::CStr;
use std::ptr::null_mut;

use sr::GSList;
use sr::sr_channel;

use crate::sr_try;
use crate::types::ChannelType;
use crate::types::SrError;

/// A device can be though as any lab instrument which is capable of
/// measuring something. E.g.: Logic analyzers, oscilloscopes, multimeters,
/// thermometers, etc.
///
/// A device has a driver to communicate with it, and an arbitrary number of
/// `channels` from where data is read.
#[derive(Debug, Default)]
pub struct Device {
    /// Driver used to handle with the device.
    driver: Driver,
    /// Raw C FFI pointer to the device's instance.
    p_device: *mut sr_dev_inst,
    /// Vendor string, may be empty.
    vendor: String,
    /// Model string, may be empty
    model: String,
    /// Version string, may be empty.
    version: String,
    /// Serial number, may be empty.
    serial_number: String,
    /// Connection ID, may be empty.
    connection_id: String,
    /// Device's channels, from where data will be read.
    channels: Vec<Channel>,
}

impl Device {
    /// Returns the list of all discovered devices currently plugged to the PC.
    ///
    /// The "demo" device is always discovered, so the returned vector will
    /// never be empty.
    pub fn scan(context: *mut sr::sr_context) -> Result<Vec<Device>, SrError> {
        let mut devices: Vec<Device> = Vec::new();

        // The driver is the one that scans for devices, so you have to
        // initialize each one and do a search.
        for driver in Driver::list(context)? {
            sr_try!(sr::sr_driver_init(context, driver.get_pointer()));

            let device_list: *mut GSList =
                unsafe { sr::sr_driver_scan(driver.get_pointer(), 0x00 as *mut GSList) };
            let mut device_node: *mut GSList = device_list;

            while device_node != null_mut() {
                let p_device: *mut sr_dev_inst = unsafe { *device_node }.data.cast();

                devices.push(Device::new(p_device, driver.clone()));
                device_node = unsafe { *device_node }.next;
            }

            // unsafe { glib::ffi::g_slist_free(device_list.cast()) };
        }

        Ok(devices)
    }

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
            let out: String = unsafe { CStr::from_ptr(connection_id) }
                .to_string_lossy()
                .to_string()
                .clone();

            //unsafe{glib::ffi::g_free(connection_id.cast_mut().cast())};
            out
        };

        Device {
            driver: driver,
            p_device: p_device,
            vendor: vendor,
            model: model,
            version: version,
            serial_number: serial_number,
            connection_id: connection_id,
            channels: Channel::get_channels(p_device),
        }
    }

    pub fn open(&self) -> Result<(), SrError> {
        sr_try!(sr::sr_dev_open(self.p_device));
        Ok(())
    }

    /// Returns the vendor string.
    pub fn get_vendor(&self) -> &String {
        &self.vendor
    }

    /// Returns the model string.
    pub fn get_model(&self) -> &String {
        &self.model
    }

    /// Returns the version string.
    pub fn get_version(&self) -> &String {
        &self.version
    }

    /// Returns the serial string.
    pub fn get_serial_number(&self) -> &String {
        &self.serial_number
    }

    /// Returns the connection ID string.
    pub fn get_connection_id(&self) -> &String {
        &self.connection_id
    }

    /// Returns the pointer to the device structure.
    pub fn get_pointer(&self) -> *mut sr_dev_inst {
        self.p_device
    }

    /// Returns the device's channel whose index matches the argument.
    pub fn get_channel_by_index(&self, index: i32) -> Option<&Channel> {
        self.channels.iter().find(|channel| channel.index == index)
    }

    /// Returns the device's channel whose name matches the argument.
    pub fn get_channel_by_name(&self, name: String) -> Option<&Channel> {
        self.channels.iter().find(|channel| channel.name == name)
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
            || (value == self.driver.get_name())
            || (value == self.driver.get_long_name())
    }
}

/// A Channel represents a reading stream.
#[derive(Debug)]
pub struct Channel {
    /// Raw FFI C pointer to the channel struct
    p_channel: *mut sr_channel,
    /// Name of the channel. E.g. "D0", "D1", "A0", etc.
    name: String,
    /// Whether the channel is enabled, i.e., will read data when the session
    /// starts, or not.
    enabled: bool,
    /// Index of the channel. This value is used to reference it if needed.
    index: i32,
    /// Channel type, either digital or analog.
    channel_type: ChannelType,
}

impl Channel {
    /// Creates a new Channel struct from a raw FFI C `sr_channel` pointer.
    ///
    /// This function will panic! if `p_channel` is NULL.
    pub fn new(p_channel: *mut sr_channel) -> Self {
        if p_channel == null_mut() {
            panic!("Channel::new(), p_channel was NULL");
        }
        let channel: sr_channel = unsafe { *p_channel };
        Self {
            p_channel: p_channel,
            name: unsafe { CStr::from_ptr(channel.name) }
                .to_string_lossy()
                .to_string(),
            enabled: channel.enabled != 0,
            index: channel.index,
            channel_type: ChannelType::from(channel.type_),
        }
    }

    /// Returns a vector holding all the listed channels for the given device.
    pub fn get_channels(p_device: *const sr_dev_inst) -> Vec<Self> {
        let mut channels: Vec<Channel> = Vec::new();

        let channel_list: *mut GSList = unsafe { sr::sr_dev_inst_channels_get(p_device) };
        let mut channel_node: *mut GSList = channel_list;

        while channel_node != null_mut() {
            let p_channel: *mut sr_channel = unsafe { *channel_node }.data.cast();

            channels.push(Channel::new(p_channel));
            channel_node = unsafe { *channel_node }.next;
        }

        // unsafe { glib::ffi::g_slist_free(channel_list.cast()) };

        channels
    }

    /// Returns the raw FFI C pointer to the channel struct.
    pub fn get_pointer(&self) -> *mut sr_channel {
        self.p_channel
    }

    /// Returns the name of the channel
    pub fn get_name(&self) -> &String {
        &self.name
    }

    /// Returns the index of the channel
    pub fn get_index(&self) -> i32 {
        self.index
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_scan() {}

    #[test]
    fn test_demo_driver_detection() {}

    ///
    #[test]
    fn test_demo_device_detection() {}
}
