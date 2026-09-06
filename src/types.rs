//! Rust's structs and enums derived from the primitive types of libsigrok.

use std::ptr::null;
use std::{ffi::CStr, ptr::null_mut};

use glib::ffi::GVariant;
use libsigrok_sys::sigrok::{self as sr, _GVariant, sr_keytype_SR_KEY_CONFIG};

use libsigrok_sys::sigrok::{sr_configkey_SR_CONF_LOGIC_ANALYZER, sr_key_info};
use thiserror::Error;

use crate::sr_try;
use crate::utils::garray_to_vec;

/// Log level
#[derive(Debug, Clone, Copy)]
#[repr(i32)]
#[allow(missing_docs)]
pub enum LogLevel {
    LogNone = 0,
    LogErr = 1,
    LogWarn = 2,
    LogInfo = 3,
    LogDbg = 4,
    LogSpew = 5,
}

/// Sigrok error codes
#[derive(Error, Debug)]
#[repr(i32)]
#[allow(missing_docs)]
pub enum SrError {
    #[error("No error.")]
    SrOk = 0,
    #[error("Generic/unspecified error.")]
    SrErr = -1,
    #[error("Malloc/calloc/realloc error.")]
    SrErrMalloc = -2,
    #[error("Function argument error.")]
    SrErrArg = -3,
    #[error("Errors hinting at internal bugs.")]
    SrErrBug = -4,
    #[error("Incorrect samplerate.")]
    SrErrSampleRate = -5,
    #[error("Not applicable.")]
    SrErrNA = -6,
    #[error("Device is closed, but must be open.")]
    SrErrDevClose = -7,
    #[error("A timeout occurred.")]
    SrErrTimeout = -8,
    #[error("A channel group must be specified.")]
    SrErrChannelGroup = -9,
    #[error("Data is invalid.")]
    SrErrData = -10,
    #[error("Input/output error.")]
    SrErrIO = -11,
    // From here on, these are custom error codes
    #[error("Device not found")]
    SrDeviceNotFound = -12,
    #[error("Pointer was NULL")]
    SrNull = -13,
}

impl TryFrom<i32> for SrError {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Err("SrOk is not an error"),
            -1 => Ok(SrError::SrErr),
            -2 => Ok(SrError::SrErrMalloc),
            -3 => Ok(SrError::SrErrArg),
            -4 => Ok(SrError::SrErrBug),
            -5 => Ok(SrError::SrErrSampleRate),
            -6 => Ok(SrError::SrErrNA),
            -7 => Ok(SrError::SrErrDevClose),
            -8 => Ok(SrError::SrErrTimeout),
            -9 => Ok(SrError::SrErrChannelGroup),
            -10 => Ok(SrError::SrErrData),
            -11 => Ok(SrError::SrErrIO),
            -12 => Ok(SrError::SrDeviceNotFound),
            _ => Err("Unknown SR_ERROR code value"),
        }
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

#[derive(Debug, Clone)]
pub struct ConfigOption {
    pub key: u32,
    pub data_type: GVariantDataType,
    pub id: String,
    pub name: String,
    pub value: String,
    pub possible_values: Vec<String>,
}

impl ConfigOption {
    /// Returns a vector with all the configuration options for the given device.
    ///
    /// If `p_group == null()`, then the configuration options returned will be
    /// device-wide. Otherwise, they will be specific to the channel group.
    pub fn scan(
        p_driver: *const sr::sr_dev_driver,
        p_device: *const sr::sr_dev_inst,
        p_group: *const sr::sr_channel_group,
    ) -> Result<Vec<ConfigOption>, SrError> {
        let keys: *mut sr::_GArray = unsafe { sr::sr_dev_options(p_driver, p_device, p_group) };
        let keys: Vec<u32> = garray_to_vec(keys);

        let mut options: Vec<ConfigOption> = Vec::new();
        for key in keys {
            let key_info: *const sr::sr_key_info =
                unsafe { sr::sr_key_info_get(sr_keytype_SR_KEY_CONFIG as i32, key) };
            if key_info == null() {
                continue;
            }

            let key_info = unsafe { *key_info };

            let id: String = if key_info.id == null_mut() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(key_info.id) }
                    .to_string_lossy()
                    .to_string()
            };

            let name: String = if key_info.name == null_mut() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(key_info.name) }
                    .to_string_lossy()
                    .to_string()
            };

            let data_type = GVariantDataType::try_from(key_info.datatype)?;
            let mut possible_values: Vec<String> = Vec::new();

            if data_type == GVariantDataType::STRING {
                let mut p_g_variant: *mut GVariant = null_mut();
                let status = unsafe {
                    sr::sr_config_list(
                        p_driver,
                        p_device,
                        p_group,
                        key,
                        std::ptr::addr_of_mut!(p_g_variant).cast(),
                    )
                };
                let qtty = if status == SrError::SrOk as i32 {
                    unsafe { glib::ffi::g_variant_n_children(p_g_variant) }
                } else if status == SrError::SrErrArg as i32 {
                    0
                } else {
                    return Err(SrError::try_from(status)).unwrap();
                };

                for i in 0..qtty {
                    let child = unsafe { glib::ffi::g_variant_get_child_value(p_g_variant, i) };
                    let value = unsafe {
                        CStr::from_ptr(glib::ffi::g_variant_get_string(child, null_mut()))
                            .to_string_lossy()
                            .to_string()
                    };
                    possible_values.push(value);
                }
            }

            let mut gvar_value: *mut GVariant = null_mut();
            let status = unsafe {
                sr::sr_config_get(
                    p_driver,
                    p_device,
                    p_group,
                    key,
                    std::ptr::addr_of_mut!(gvar_value).cast(),
                )
            };
            let value: String = if (status == SrError::SrOk as i32) {
                unsafe {
                    match data_type {
                        GVariantDataType::BOOL => {
                            glib::ffi::g_variant_get_boolean(gvar_value).to_string()
                        }
                        GVariantDataType::DOUBLE_RANGE | GVariantDataType::FLOAT => {
                            glib::ffi::g_variant_get_double(gvar_value).to_string()
                        }
                        GVariantDataType::INT32 => {
                            glib::ffi::g_variant_get_int32(gvar_value).to_string()
                        }
                        GVariantDataType::KEYVALUE | GVariantDataType::MQ => String::new(),
                        GVariantDataType::RATIONAL_PERIOD | GVariantDataType::RATIONAL_VOLT => {
                            String::new()
                        }
                        GVariantDataType::STRING => {
                            CStr::from_ptr(glib::ffi::g_variant_get_string(gvar_value, null_mut()))
                                .to_string_lossy()
                                .to_string()
                        }
                        GVariantDataType::UINT64 | GVariantDataType::UINT64_RANGE => {
                            glib::ffi::g_variant_get_uint64(gvar_value).to_string()
                        }
                    }
                }
            } else {
                println!("Error for key {}", name);
                String::new()
            };

            let config_option = ConfigOption {
                key: key_info.key,
                data_type: data_type,
                id: id,
                name: name,
                value: value,
                possible_values: possible_values,
            };

            options.push(config_option);
        }

        Ok(options)
    }
}

/// pub const sr_datatype_SR_T_UINT64: sr_datatype = 10000;
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum GVariantDataType {
    UINT64 = 10000,
    STRING = 10001,
    BOOL = 10002,
    FLOAT = 10003,
    RATIONAL_PERIOD = 10004,
    RATIONAL_VOLT = 10005,
    KEYVALUE = 10006,
    UINT64_RANGE = 10007,
    DOUBLE_RANGE = 10008,
    INT32 = 10009,
    MQ = 10010,
}

impl TryFrom<i32> for GVariantDataType {
    type Error = SrError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            10000 => Ok(GVariantDataType::UINT64),
            10001 => Ok(GVariantDataType::STRING),
            10002 => Ok(GVariantDataType::BOOL),
            10003 => Ok(GVariantDataType::FLOAT),
            10004 => Ok(GVariantDataType::RATIONAL_PERIOD),
            10005 => Ok(GVariantDataType::RATIONAL_VOLT),
            10006 => Ok(GVariantDataType::KEYVALUE),
            10007 => Ok(GVariantDataType::UINT64_RANGE),
            10008 => Ok(GVariantDataType::DOUBLE_RANGE),
            10009 => Ok(GVariantDataType::INT32),
            10010 => Ok(GVariantDataType::MQ),
            _ => Err(SrError::SrErrNA),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum DeviceType {
    /// The device can act as logic analyzer.
    LogicAnalyzer = 10000,
    /// The device can act as an oscilloscope.
    Oscilloscope = 10001,
    /// The device can act as a multimeter.
    Multimeter = 10002,
    /// The device is a demo device.
    DemoDev = 10003,
    /// The device can act as a sound level meter.
    SoundLevelMeter = 10004,
    /// The device can measure temperature.
    Thermometer = 10005,
    /// The device can measure humidity.
    Hygrometer = 10006,
    /// The device can measure energy consumption.
    EnergyMeter = 10007,
    /// The device can act as a signal demodulator.
    Demodulator = 10008,
    /// The device can act as a programmable power supply.
    PowerSupply = 10009,
    /// The device can act as an LCR meter.
    LCRMeter = 10010,
    /// The device can act as an electronic load.
    ElectronicLoad = 10011,
    /// The device can act as a scale.
    Scale = 10012,
    /// The device can act as a function generator.
    SignalGenerator = 10013,
    /// The device can measure power.
    PowerMeter = 10014,
}
