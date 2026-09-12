//! Device representation

use glib::ffi::GVariant;
use libsigrok_sys::sigrok as sr;

use sr::{GSList, sr_channel, sr_channel_group, sr_context, sr_dev_driver, sr_dev_inst};

use crate::config_option::ConfigOption;
use crate::config_option::GVariantDataType;
use crate::driver::Driver;
use crate::utils::gslist_to_vec;

use std::ffi::{CStr, CString};
use std::fmt::Display;
use std::ptr::{null, null_mut};

use crate::sr_try;
use crate::types::SrError;

/// A device can be thought as any lab instrument capable of
/// measuring something. E.g.: Logic analyzers, oscilloscopes, multimeters,
/// thermometers, etc.
///
/// A device has a driver to communicate with it, configuration options,
/// and an arbitrary number of `channel_groups` that hold `channels`
/// from where data is read.
#[derive(Debug)]
pub struct Device {
    /// Driver used to handle the device.
    driver: Driver,
    /// Raw C-FFI pointer to the device's instance.
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
    /// Device-wide configuration options.
    options: Vec<ConfigOption>,
    /// Channel groups.
    ///
    /// All channels must be within a group, whose sole purpose is to store
    /// common configuration between them.
    /// Although most devices only have a single group, some devices may have,
    /// for example, digital and analog channels.
    channel_groups: Vec<ChannelGroup>,
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Device info:")?;

        let mut device_info: String = String::new();
        if !self.get_vendor().is_empty() {
            device_info.push_str(format!("  * Vendor: \"{}\"\n", self.get_vendor()).as_str());
        }
        if !self.get_model().is_empty() {
            device_info.push_str(format!("  * Model: \"{}\"\n", self.get_model()).as_str());
        }
        if !self.get_version().is_empty() {
            device_info.push_str(format!("  * Version: \"{}\"\n", self.get_version()).as_str());
        }
        if !self.get_serial_number().is_empty() {
            device_info.push_str(
                format!("  * Serial number: \"{}\"\n", self.get_serial_number()).as_str(),
            );
        }
        if !self.get_connection_id().is_empty() {
            device_info.push_str(
                format!("  * Connection ID: \"{}\"\n", self.get_connection_id()).as_str(),
            );
        }
        write!(f, "{}", device_info)?;
        writeln!(f, "  * {}", self.driver)?;

        let mut options: String = String::new();

        for option in &self.options {
            options.push_str(format!("    - {}: \"{}\"\n", option.id, option.value).as_str());
        }

        if !options.is_empty() {
            writeln!(f, "  * Device options:")?;
            writeln!(f, "{}", options)?;
        }

        for channel_group in &self.channel_groups {
            writeln!(f, "  * Channel group: \"{}\"", channel_group.name)?;
            writeln!(
                f,
                "    | {:^10} | {:^5} | {:^7} | {:^7} |",
                "Name", "Index", "Enabled", "Type"
            )?;
            for channel in &channel_group.channels {
                writeln!(
                    f,
                    "    | {:^10} | {:^5} | {:^7} | {:^7} |",
                    channel.name,
                    channel.index,
                    channel.enabled,
                    channel.channel_type.as_str()
                )?;
            }

            let mut options: String = String::new();

            for option in &channel_group.options {
                options.push_str(format!("    - {}: \"{}\"\n", option.id, option.value).as_str());
            }

            if !options.is_empty() {
                writeln!(f, "    Channel group options:")?;
                writeln!(f, "{}", options)?;
            }
        }

        Ok(())
    }
}

impl Device {
    /// Returns all discovered devices currently plugged to the PC.
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

        let config_options: Vec<ConfigOption> =
            ConfigOption::scan(driver.get_pointer(), p_device, null())?;

        let dev = Device {
            driver: driver.clone(),
            p_device: p_device,
            vendor: vendor,
            model: model,
            version: version,
            serial_number: serial_number,
            connection_id: connection_id,
            options: config_options,
            channel_groups: ChannelGroup::scan(p_device, driver.get_pointer())?,
        };

        dev.open()?;
        Ok(dev)
    }

    /// Open the device. Most operations, like changing a configuration or
    /// reading data, can't be done if the device is not opened.
    ///
    /// This functions will not return an error if the device was already
    /// opened.
    fn open(&self) -> Result<(), SrError> {
        let status = unsafe { sr::sr_dev_open(self.p_device) };
        let status = SrError::from(status);
        match status {
            SrError::SrOk | SrError::SrErr => Ok(()),
            _ => Err(status),
        }
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

    /// Returns the device's channel whose name or index matches the argument `id`.
    ///
    /// * `id`: Either the channel's name, or the channel index, as a String.
    pub fn get_channel(&self, id: &str) -> Result<&Channel, SrError> {
        for group in &self.channel_groups {
            let channel = group
                .channels
                .iter()
                .find(|channel| channel.name == id || channel.index.to_string() == id);

            if channel.is_some() {
                return Ok(channel.unwrap());
            }
        }
        Err(SrError::SrChannelNotFound)
    }

    /// Returns a mutable reference to the device's channel whose
    /// name or index matches the argument `id`.
    ///
    /// * `id`: Either the channel's name, or the channel index, as a String.
    pub fn get_channel_mut(&mut self, id: &str) -> Result<&mut Channel, SrError> {
        for group in &mut self.channel_groups {
            let channel = group
                .channels
                .iter_mut()
                .find(|channel| channel.name == id || channel.index.to_string() == id);

            if channel.is_some() {
                return Ok(channel.unwrap());
            }
        }
        Err(SrError::SrChannelNotFound)
    }

    /// Compares the given value with the device's vendor, model, version,
    /// serial number and connection ID, and also its driver name.
    ///
    /// Returns `true` if any of them match.
    /// TODO
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

    /// Sets the option `id` for the given `channel_group_name` to the given
    /// `value`.
    ///
    /// * `channel_group_name`: Channel group name. This function will return
    /// the error `SrError::SrErrChannelGroup` if it can't be found.
    ///
    /// * `id`: The option's ID.
    ///
    /// * "value": The value for the given `id`. Although this functions
    /// receives a string as argument, the value must be parseable to the
    /// option's datatype, or an error `SrError::SrInvalidOptionValue` will be
    /// returned.
    pub fn set_channel_option(
        &mut self,
        channel_group_name: &str,
        id: &str,
        value: &str,
    ) -> Result<(), SrError> {
        self.set_option(format!("{} {}", channel_group_name, id).as_str(), value)
    }

    /// Sets the option `id` to the given `value`.
    ///
    /// * `id`: If the string `"<option_id>"` is given, this function assumes
    /// that it is a device option or it may be channel group option if the
    /// device has a single channel group. If the device has multiple channel
    /// groups, their options can be set by passing a two word string formed
    /// by the channel group name and the option's id as such:
    /// `"<channel_group_name> <option_id>"`.
    ///
    /// * "value": The value for the given `id`. Although this functions
    /// receives a string as argument, the value must be parseable to the
    /// option's datatype, or an error `SrError::SrInvalidOptionValue` will be
    /// returned.
    pub fn set_option(&mut self, id: &str, value: &str) -> Result<(), SrError> {
        let option = self.options.iter_mut().find(|o| o.id == id);

        let (option, p_group): (&mut ConfigOption, *const sr_channel_group) = if option.is_some() {
            (option.unwrap(), null())
        } else {
            let whitespace_separated_id: Vec<&str> = id.split(" ").collect();

            let (group, option_id): (&mut ChannelGroup, &str) =
                if whitespace_separated_id.len() == 1 {
                    (&mut self.channel_groups[0], id)
                } else if whitespace_separated_id.len() == 2 {
                    let group_name = whitespace_separated_id[0];
                    let option_id = whitespace_separated_id[1];

                    let group = self
                        .channel_groups
                        .iter_mut()
                        .find(|g| g.name == group_name);

                    if group.is_none() {
                        return Err(SrError::SrErrChannelGroup);
                    }

                    (group.expect("Group is Some()"), option_id)
                } else {
                    return Err(SrError::SrOptionNotExist);
                };

            let option = group.options.iter_mut().find(|o| o.id == option_id);

            if option.is_none() {
                return Err(SrError::SrOptionNotExist);
            }

            (option.unwrap(), group.p_group)
        };

        option.set(value, self.p_device, p_group)
    }

    /// Returns the current value of the given option `id`, which belongs to
    /// the `channel_group_name`.
    ///
    /// * `channel_group_name`: Channel group name. This function will return
    /// the error `SrError::SrErrChannelGroup` if it can't be found.
    ///
    /// * `id`: The option's ID.
    ///
    /// The returned value will always be a String, and its the user's
    /// responsibility to parse it to the correct data type.
    pub fn get_channel_option(
        &self,
        channel_group_name: &str,
        id: &str,
    ) -> Result<&String, SrError> {
        self.get_option(format!("{} {}", channel_group_name, id).as_str())
    }

    /// Returns the current value of the given option `id`.
    ///
    /// * `id`: If the string `"<option_id>"` is given, this function assumes
    /// that it is a device option or it may be channel group option if the
    /// device has a single channel group. If the device has multiple channel
    /// groups, their options can be set by passing a two word string formed
    /// by the channel group name and the option's id as such:
    /// `"<channel_group_name> <option_id>"`.
    ///
    /// The returned value will always be a String, and its the user's
    /// responsibility to parse it to the correct data type.
    pub fn get_option(&self, id: &str) -> Result<&String, SrError> {
        let option = self.options.iter().find(|o| o.id == id);

        let option = if option.is_some() {
            option.unwrap()
        } else {
            let whitespace_separated_id: Vec<&str> = id.split(" ").collect();

            let (group, option_id): (&ChannelGroup, &str) = if whitespace_separated_id.len() == 1 {
                (&self.channel_groups[0], id)
            } else if whitespace_separated_id.len() == 2 {
                let group_name = whitespace_separated_id[0];
                let option_id = whitespace_separated_id[1];

                let group = self.channel_groups.iter().find(|g| g.name == group_name);

                if group.is_none() {
                    return Err(SrError::SrErrChannelGroup);
                }

                (group.expect("Group is Some()"), option_id)
            } else {
                return Err(SrError::SrOptionNotExist);
            };

            let option = group.options.iter().find(|o| o.id == option_id);

            if option.is_none() {
                return Err(SrError::SrOptionNotExist);
            }

            option.unwrap()
        };

        Ok(&option.value)
    }

    /// Enables or disables the given channel.
    pub fn enable_channel(&mut self, name: &str, enable: bool) -> Result<(), SrError> {
        let channel = self.get_channel_mut(name)?;
        sr_try!(sr::sr_dev_channel_enable(
            channel.get_pointer(),
            enable as i32
        ));
        channel.enabled = enable;
        Ok(())
    }

    /// Returns "true" if the channel is enabled.
    pub fn is_channel_enabled(&self, name: &str) -> Result<bool, SrError> {
        let channel = self.get_channel(name)?;
        Ok(channel.enabled)
    }

    /// Changes the channel's name from `old_name` to `new_name`.
    ///
    /// * `old_name`: The channel's current name or index number, as a string.
    /// * `new_name`: The channel's new name.
    pub fn set_channel_name(&mut self, old_name: &str, new_name: &str) -> Result<(), SrError> {
        let channel = self.get_channel_mut(old_name)?;
        sr_try!(sr::sr_dev_channel_name_set(
            channel.get_pointer(),
            new_name.as_ptr().cast()
        ));
        channel.name = new_name.to_string();
        Ok(())
    }
}

impl TryFrom<(&str, *mut sr_context)> for Device {
    type Error = SrError;

    /// Connects a `Device` from a `value` string, which may match the device's
    /// vendor, model, serial number or driver name.
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

/// A channel group holds an arbitrary amount of data channels from a
/// device and their characteristics.
#[derive(Debug)]
struct ChannelGroup {
    /// Raw C-FFI pointer.
    p_group: *mut sr_channel_group,
    /// Arbitrary name given to the channel group.
    name: String,
    /// Channels that form part of the given group.
    channels: Vec<Channel>,
    /// Configuration options that only apply to this group of channels, not
    /// the whole device. It may be empty.
    options: Vec<ConfigOption>,
}

impl ChannelGroup {
    /// Returns all channel groups from a device.
    fn scan(
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

                let options = ConfigOption::scan(p_driver, p_device, p_group)?;

                let group = ChannelGroup {
                    p_group: p_group,
                    name: name,
                    channels: channels,
                    options: options,
                };

                channel_groups.push(group);
            }
        }

        Ok(channel_groups)
    }
}

/// Hardware data channel.
///
/// A channel represents a reading stream, from where data is continuously
/// available.
#[derive(Debug)]
pub struct Channel {
    /// Raw C-FFI pointer
    p_channel: *mut sr_channel,
    /// Name of the channel. E.g. "D0", "D1", "A0", etc. It may be used to
    /// reference it.
    name: String,
    /// Index of the channel. It may be used to reference it.
    index: i32,
    /// Whether the channel is enabled, i.e., will read data when the session
    /// starts, or not.
    enabled: bool,
    /// Channel type, either digital or analog.
    channel_type: ChannelType,
}

impl Channel {
    /// Creates a new Channel struct from a raw C-FFI `sr_channel` pointer.
    ///
    /// This function will panic! if `p_channel` is NULL.
    fn new(p_channel: *mut sr_channel) -> Self {
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

/// Channel types
#[derive(Debug, Clone, Copy)]
pub enum ChannelType {
    /// Digital channel, a.k.a "logic" channel.
    Digital = 10000,
    /// Analog channel.
    Analog = 10001,
}

impl ChannelType {
    /// Returns the name of the enum as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelType::Digital => "Digital",
            ChannelType::Analog => "Analog",
        }
    }
}

impl From<i32> for ChannelType {
    fn from(value: i32) -> Self {
        match value {
            10000 => ChannelType::Digital,
            10001 => ChannelType::Analog,
            _ => ChannelType::Digital,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_scan() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let devices: Vec<Device> = Device::scan(context)?;
        assert!(!devices.is_empty());

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
    fn test_device_options() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let mut demo_device = Device::try_from(("Demo device", context))?;
        assert!(!demo_device.options.is_empty());

        demo_device.set_option("limit_samples", "50")?;
        assert!(demo_device.get_option("limit_samples")? == "50");

        let result = demo_device.set_option("non_existent_option", "100");
        assert!(result.is_err());
        assert!(result.unwrap_err() == SrError::SrOptionNotExist);

        let result = demo_device.set_option("limit_samples", "not_a_valid_value");
        assert!(result.is_err());
        assert!(result.unwrap_err() == SrError::SrInvalidOptionValue);

        let result = demo_device.get_option("non_existent_option");
        assert!(result.is_err());
        assert!(result.unwrap_err() == SrError::SrOptionNotExist);

        sr_try!(sr::sr_exit(context));
        Ok(())
    }

    #[test]
    fn test_device_illegal_options() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let mut demo_device = Device::try_from(("Demo device", context))?;
        assert!(!demo_device.options.is_empty());

        // "continuous" option can't be set
        let result = demo_device.set_option("continuous", "true");
        assert!(result.is_err());
        assert!(result.unwrap_err() == SrError::SrErrArg);

        sr_try!(sr::sr_exit(context));
        Ok(())
    }

    #[test]
    fn test_device_channel_enable() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let mut demo_device = Device::try_from(("Demo device", context))?;

        demo_device.enable_channel("3", true)?;
        assert!(demo_device.is_channel_enabled("3")?);
        demo_device.enable_channel("3", false)?;
        assert!(!demo_device.is_channel_enabled("D3")?);
        demo_device.enable_channel("D3", true)?;
        assert!(demo_device.is_channel_enabled("3")?);

        let result = demo_device.is_channel_enabled("abcdef");
        assert!(result.is_err());
        assert!(result.unwrap_err() == SrError::SrChannelNotFound);

        sr_try!(sr::sr_exit(context));
        Ok(())
    }

    #[test]
    fn test_device_channel_change_name() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let mut demo_device = Device::try_from(("Demo device", context))?;

        demo_device.set_channel_name("3", "XD")?;
        demo_device.set_channel_name("D4", "YY")?;
        assert!(demo_device.is_channel_enabled("XD").is_ok());
        assert!(demo_device.is_channel_enabled("YY").is_ok());
        assert!(demo_device.is_channel_enabled("D3").is_err());
        assert!(demo_device.is_channel_enabled("D4").is_err());

        sr_try!(sr::sr_exit(context));
        Ok(())
    }

    #[test]
    fn test_device_channel_group_options() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let mut demo_device = Device::try_from(("Demo device", context))?;

        let result = demo_device.set_channel_option("xdd", "amplitude", "5");
        assert!(result.is_err());
        assert!(result.unwrap_err() == SrError::SrErrChannelGroup);

        let result = demo_device.get_channel_option("xdd", "amplitude");
        assert!(result.is_err());
        assert!(result.unwrap_err() == SrError::SrErrChannelGroup);

        demo_device.set_channel_option("A0", "amplitude", "5")?;
        demo_device.set_channel_option("A0", "offset", "1")?;
        demo_device.set_channel_option("A0", "pattern", "triangle")?;

        assert!(demo_device.get_channel_option("A0", "amplitude")? == "5");
        assert!(demo_device.get_channel_option("A0", "offset")? == "1");
        assert!(demo_device.get_channel_option("A0", "pattern")? == "triangle");

        // "pattern" option has a list of valid values.
        let result = demo_device.set_channel_option("A0", "pattern", "invalid_pattern");
        assert!(result.is_err());
        assert!(result.unwrap_err() == SrError::SrErrArg);

        sr_try!(sr::sr_exit(context));
        Ok(())
    }

    #[test]
    fn test_device_option_with_list() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));

        let mut demo_device = Device::try_from(("Demo device", context))?;
        assert!(!demo_device.options.is_empty());

        // For the demo device, "samplerate" is defined as a list that goes
        // from 1Hz to 1GHz, in jumps of 1Hz.
        // Let's try setting the frequency to "0 Hz", i.e., an unsupported value.
        let result = demo_device.set_option("samplerate", "0");
        assert!(result.is_err());
        assert!(result.unwrap_err() == SrError::SrErrArg);

        // Setting to 500MHz should be ok
        demo_device.set_option("samplerate", "500000000")?;
        assert!(demo_device.get_option("samplerate")? == "500000000");

        sr_try!(sr::sr_exit(context));
        Ok(())
    }
}
