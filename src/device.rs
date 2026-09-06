//! Device representation

use libsigrok_sys::sigrok as sr;
use libsigrok_sys::sigrok::sr_channel_group;
use libsigrok_sys::sigrok::sr_dev_driver;
use libsigrok_sys::sigrok::sr_keytype_SR_KEY_CONFIG;

use crate::driver::Driver;
use crate::types::ConfigOption;
use crate::utils::garray_to_vec;
use crate::utils::gslist_to_vec;

use std::ffi::CStr;
use std::ptr::null;
use std::ptr::null_mut;

use sr::{GSList, sr_channel, sr_context, sr_dev_inst};

use crate::sr_try;
use crate::types::ChannelType;
use crate::types::SrError;

/// A device can be though as any lab instrument which is capable of
/// measuring something. E.g.: Logic analyzers, oscilloscopes, multimeters,
/// thermometers, etc.
///
/// A device has a driver to communicate with it, configuration options,
/// and an arbitrary number of `channel_groups`, with `channels`
/// from where data is read.
#[derive(Debug, Default)]
pub struct Device {
    /// Driver used to handle with the device.
    driver: Driver,
    /// Raw C FFI pointer to the device's instance.
    p_device: *mut sr_dev_inst,
    /// Vendor string. May be empty.
    vendor: String,
    /// Model string. May be empty
    model: String,
    /// Version string. May be empty.
    version: String,
    /// Serial number. May be empty.
    serial_number: String,
    /// Connection ID, as detected by the operating system. May be empty.
    ///
    /// A typical value would be something like "usb/5-1.2.2", which
    /// corresponds to the "sysfs" path at `/sys/bus/usb/devices/5-1.2.2`.
    connection_id: String,
    /// Device-wide configuration options
    config_options: Vec<ConfigOption>,
    /// channel groups
    channel_groups: Vec<ChannelGroup>,
}

impl Device {
    /// Returns the list of all discovered devices currently plugged to the PC.
    ///
    /// The "demo" device is always discovered, so the returned vector will
    /// never be empty.
    pub fn scan(context: *mut sr::sr_context) -> Result<Vec<Device>, SrError> {
        let mut devices: Vec<Device> = Vec::new();

        for driver in Driver::list(context)? {
            let p_devices: Vec<*mut sr_dev_inst> = driver.scan_for_devices(context)?;

            for p_device in p_devices {
                devices.push(Device::new(p_device, &driver)?);
            }
        }

        Ok(devices)
    }

    /// Given a non-null pointer to a device and its driver, this function
    /// scouts for the its related information and returns the device.
    ///
    /// * `p_device`: Pointer to a device instance.
    /// * `driver`: A driver obtained from `Driver::list()`.
    ///
    /// This function will panic! if `p_device` is NULL.
    fn new(p_device: *mut sr_dev_inst, driver: &Driver) -> Result<Self, SrError> {
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

        let config_options = ConfigOption::scan(driver.get_pointer(), p_device, null())?;

        Ok(Device {
            driver: driver.clone(),
            p_device: p_device,
            vendor: vendor,
            model: model,
            version: version,
            serial_number: serial_number,
            connection_id: connection_id,
            config_options: config_options,
            channel_groups: ChannelGroup::scan(p_device, driver.get_pointer())?,
        })
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

    /// Returns the driver's name associated with the device.
    pub fn get_driver_name(&self) -> &String {
        self.driver.get_name()
    }

    /// Returns the device's channel whose index matches the argument.
    pub fn get_channel_by_index(&self, index: i32) -> Option<&Channel> {
        self.channel_groups[0]
            .channels
            .iter()
            .find(|channel| channel.index == index)
    }

    /// Returns the device's channel whose name matches the argument.
    pub fn get_channel_by_name(&self, name: String) -> Option<&Channel> {
        self.channel_groups[0]
            .channels
            .iter()
            .find(|channel| channel.name == name)
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

impl TryFrom<(&str, *mut sr_context)> for Device {
    type Error = SrError;

    fn try_from(value: (&str, *mut sr_context)) -> Result<Self, Self::Error> {
        let devices = Device::scan(value.1)?;
        let expected_device = devices.into_iter().find(|dev| {
            (dev.get_vendor() == value.0)
                || (dev.get_model() == value.0)
                || (dev.get_serial_number() == value.0 || (dev.get_driver_name() == value.0))
        });

        if expected_device.is_none() {
            return Err(SrError::SrDeviceNotFound);
        }

        Ok(expected_device.unwrap())
    }
}

/// A channel group holds an arbitrary amount data channels from a
/// device, and groups common characteristics between them.
///
/// A typical separation comes from having digital and analog channel
/// groups in the same device.
#[derive(Debug)]
pub struct ChannelGroup {
    /// Arbitrary name given to the channel group.
    pub name: String,
    /// Channels that form part of the given group.
    pub channels: Vec<Channel>,
    /// Configuration options that only apply to this group of channels, not
    /// the whole device. It may be empty.
    pub config_options: Vec<ConfigOption>,
}

impl ChannelGroup {
    /// Returns all channel groups from a device.
    pub fn scan(
        p_device: *mut sr_dev_inst,
        p_driver: *const sr_dev_driver,
    ) -> Result<Vec<ChannelGroup>, SrError> {
        let p_channel_groups: *mut GSList = unsafe { sr::sr_dev_inst_channel_groups_get(p_device) };
        let p_channel_groups: Vec<*mut sr_channel_group> = gslist_to_vec(p_channel_groups);

        let mut channel_groups: Vec<ChannelGroup> = Vec::new();
        for p_group in p_channel_groups {
            if p_group != null_mut() {
                let mut channels: Vec<Channel> = Vec::new();

                let group: sr_channel_group = unsafe { *p_group };

                let name: String = unsafe { CStr::from_ptr(group.name) }
                    .to_string_lossy()
                    .to_string();

                let p_channels: Vec<*mut sr_channel> = gslist_to_vec(group.channels);

                for p_channel in p_channels {
                    channels.push(Channel::new(p_channel));
                }

                let channel_options = ConfigOption::scan(p_driver, p_device, p_group)?;

                let group = ChannelGroup {
                    name: name,
                    channels: channels,
                    config_options: channel_options,
                };

                channel_groups.push(group);
            }
        }

        Ok(channel_groups)
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
    use crate::types::LogLevel;

    use super::*;

    #[test]
    fn test_device_scan() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));
        //sr_try!(sr::sr_log_loglevel_set(LogLevel::LogSpew as i32));

        let devices: Vec<Device> = Device::scan(context)?;
        assert!(!devices.is_empty());

        dbg!(&devices);
        panic!("hi");

        let demo_device = devices
            .into_iter()
            .find(|dev| dev.get_model() == "Demo device")
            .expect("Demo device should exist");
        assert!(demo_device.driver.get_name() == "demo");

        sr_try!(sr::sr_exit(context));
        Ok(())
    }

    #[test]
    fn test_device_from_driver_name() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let demo_device = Device::try_from(("demo", context))?;
        assert!(demo_device.get_model() == "Demo device");

        sr_try!(sr::sr_exit(context));
        Ok(())
    }

    #[test]
    fn test_device_from_model_name() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let demo_device = Device::try_from(("Demo device", context))?;
        assert!(demo_device.get_driver_name() == "demo");

        sr_try!(sr::sr_exit(context));
        Ok(())
    }

    #[test]
    fn test_device_config_options() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let demo_device = Device::try_from(("Demo device", context))?;
        assert!(!demo_device.config_options.is_empty());
        //todo!();

        sr_try!(sr::sr_exit(context));
        Ok(())
    }

    #[test]
    fn test_device_channel_groups() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let demo_device = Device::try_from(("Demo device", context))?;
        //todo!();

        sr_try!(sr::sr_exit(context));
        Ok(())
    }
}
