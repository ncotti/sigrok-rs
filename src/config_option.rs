//! Configuration options for devices and channel groups

use std::{
    ffi::{CStr, CString},
    ptr::{null, null_mut},
};

use glib::ffi::GVariant;
use libsigrok_sys::sigrok as sr;

use sr::{
    sr_channel_group, sr_configcap_SR_CONF_GET, sr_configcap_SR_CONF_LIST, sr_dev_driver,
    sr_dev_inst, sr_keytype_SR_KEY_CONFIG,
};

use crate::{
    sr_try,
    types::SrError,
    utils::{garray_to_vec, gvariant_to_string},
};

/// A configuration option is any modifiable parameter from a device or a
/// channel group.
#[derive(Debug, Clone)]
pub struct ConfigOption {
    /// Numerical value that identifies the configuration option. These
    /// values match the ones defined in the C `enum sr_configkey`, i.e., the
    /// `SR_CONF_` constants.
    key: u32,
    /// Data type reported by libsigrok. Values are stored as Strings, but
    /// are parsed to their corresponding data types, stored in this variable.
    data_type: GVariantDataType,
    /// Unique identifier for the configuration option. This values is the one
    /// used to reference it.
    pub id: String,
    /// Descriptive name, purely informative.
    pub name: String,
    /// Configuration's current value.
    ///
    /// If the option does not have a value, an empty string will be displayed.
    pub value: String,
    /// Some options have a list of possible values, or an allowed range with
    /// steps. This vector will hold those possible values, in an informative
    /// way.
    pub possible_values: Vec<String>,
}

impl ConfigOption {
    /// Returns a vector with all the configuration options for the given device
    /// or the channel group.
    ///
    /// * `p_driver`: C-FFI pointer to a driver.
    /// * `p_device`: C-FFI pointer to a device.
    /// * `p_group`: C-FFI pointer to a device, or `null()`.
    ///
    /// If `p_group == null()`, then the configuration options returned will be
    /// device-wide. Otherwise, they will be specific to the channel group.
    pub fn scan(
        p_driver: *const sr_dev_driver,
        p_device: *const sr_dev_inst,
        p_group: *const sr_channel_group,
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

            // All config options may have three capabilities:
            // `sr_configcap_SR_CONF_GET`: The value can be retrieved
            // `sr_configcap_SR_CONF_SET`: A value can be set.
            // sr_configcap_SR_CONF_LIST`: It has a list of possible values.
            let config_capabilities =
                unsafe { sr::sr_dev_config_capabilities_list(p_device, p_group, key as i32) };

            let mut possible_values: Vec<String> = Vec::new();
            if (config_capabilities & sr_configcap_SR_CONF_LIST as i32) != 0 {
                let mut p_g_variant: *mut GVariant = null_mut();

                sr_try!(sr::sr_config_list(
                    p_driver,
                    p_device,
                    p_group,
                    key,
                    std::ptr::addr_of_mut!(p_g_variant).cast(),
                ));

                let qtty = unsafe { glib::ffi::g_variant_n_children(p_g_variant) };

                for i in 0..qtty {
                    let child = unsafe { glib::ffi::g_variant_get_child_value(p_g_variant, i) };
                    let value = gvariant_to_string(data_type, child)?;
                    possible_values.push(value);
                }
            }

            let value: String = if (config_capabilities & sr_configcap_SR_CONF_GET as i32) != 0 {
                let mut gvar: *mut GVariant = null_mut();
                sr_try!(sr::sr_config_get(
                    p_driver,
                    p_device,
                    p_group,
                    key,
                    std::ptr::addr_of_mut!(gvar).cast(),
                ));
                gvariant_to_string(data_type, gvar)?
            } else if (config_capabilities & sr_configcap_SR_CONF_LIST as i32) != 0 {
                possible_values.join(" ")
            } else {
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

    /// Sets a configuration option to a given value.
    pub fn set(
        &mut self,
        value: &str,
        p_device: *const sr_dev_inst,
        p_group: *const sr_channel_group,
    ) -> Result<(), SrError> {
        let data: *mut GVariant = match self.data_type {
            GVariantDataType::Bool => {
                let possible_true_values: Vec<&str> = vec!["true", "1", "on", "ok", "t"];
                let possible_false_values: Vec<&str> = vec!["false", "0", "off", "f"];
                let value: i32 = if possible_true_values.contains(&value.to_lowercase().as_str()) {
                    1
                } else if possible_false_values.contains(&value.to_lowercase().as_str()) {
                    0
                } else {
                    return Err(SrError::SrInvalidOptionValue);
                };

                unsafe { glib::ffi::g_variant_new_boolean(value) }
            }
            GVariantDataType::DoubleRange | GVariantDataType::Float => {
                let value: Result<f64, std::num::ParseFloatError> = value.parse();
                if value.is_err() {
                    return Err(SrError::SrInvalidOptionValue);
                }
                let value = value.expect("Value is not error");
                unsafe { glib::ffi::g_variant_new_double(value) }
            }
            GVariantDataType::Int32 => {
                let value: Result<i32, std::num::ParseIntError> = value.parse();
                if value.is_err() {
                    return Err(SrError::SrInvalidOptionValue);
                }
                let value = value.expect("Value is not error");
                unsafe { glib::ffi::g_variant_new_int32(value) }
            }
            GVariantDataType::KeyValue => {
                todo!()
            }
            GVariantDataType::MQ => {
                todo!()
            }
            GVariantDataType::RationalPeriod | GVariantDataType::RationalVolt => {
                todo!()
            }
            GVariantDataType::String => unsafe {
                glib::ffi::g_variant_new_string(CString::new(value.as_bytes()).unwrap().as_ptr())
            },
            GVariantDataType::Uint64 | GVariantDataType::Uint64Range => {
                let value: Result<u64, std::num::ParseIntError> = value.parse();
                if value.is_err() {
                    return Err(SrError::SrInvalidOptionValue);
                }
                let value = value.expect("Value is not error");
                unsafe { glib::ffi::g_variant_new_uint64(value) }
            }
        };

        sr_try!(sr::sr_config_set(p_device, p_group, self.key, data.cast()));
        sr_try!(sr::sr_config_commit(p_device));

        self.value = String::from(value);
        Ok(())
    }
}

/// All possible types for a glib GVariant returned by Sigrok
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum GVariantDataType {
    /// u64. It is possible for it to hold multiple u64 presented as a "{sv}",
    /// i.e., a string (s) and another GVariant (s), which holds inside a "at",
    /// an array (a) of u64 (t).
    Uint64 = 10000,
    /// Single String
    String = 10001,
    /// Single bool
    Bool = 10002,
    /// Single f32
    Float = 10003,
    /// TODO
    RationalPeriod = 10004,
    /// TODO
    RationalVolt = 10005,
    /// TODO
    KeyValue = 10006,
    /// TODO
    Uint64Range = 10007,
    /// TODO
    DoubleRange = 10008,
    /// Single i32
    Int32 = 10009,
    /// Measured quantity. The value usually has the type {ut}, i.e., a tuple
    /// with a u32 (u) and a u64 (t), which hold a MeasuredQuantity enum value
    /// and a MeasuredQuantityFlag enum.
    MQ = 10010,
}

/// Measured quantity, returned from a ConfigOption.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasuredQuantity {
    /// Voltage [V]
    Voltage = 10000,
    /// Current [A]
    Current,
    /// Resistance [Ohm]
    Resistance,
    /// Capacitance [F]
    Capacitance,
    /// Temperature [°C]
    Temperature,
    /// Frequency [Hz]
    Frequency,
    /// Duty cycle, e.g. on/off ratio.
    DutyCycle,
    /// Continuity test.
    Continuity,
    /// Pulse width [s]
    PulseWidth,
    /// Conductance [Siemens]
    Conductance,
    /// Electrical power, usually in W, or dBm.
    Power,
    /// Gain (a transistor's gain, or hFE, for example).
    Gain,
    /// Logarithmic representation of sound pressure relative to a reference value.
    SoundPressureLevel,
    /// Carbon monoxide level.
    CarbonMonoxide,
    /// Humidity.
    RelativeHumidity,
    /// Time.
    Time,
    /// Wind speed.
    WindSpeed,
    /// Pressure.
    Pressure,
    /// Parallel inductance (LCR meter model).
    ParallelInductance,
    /// Parallel capacitance (LCR meter model).
    ParallelCapacitance,
    /// Parallel resistance (LCR meter model).
    ParallelResistance,
    /// Series inductance (LCR meter model).
    SeriesInductance,
    /// Series capacitance (LCR meter model).
    SeriesCapacitance,
    /// Series resistance (LCR meter model).
    SeriesResistance,
    /// Dissipation factor.
    DissipationFactor,
    /// Quality factor.
    QualityFactor,
    /// Phase angle.
    PhaseAngle,
    /// Difference from reference value.
    Difference,
    /// Count.
    Count,
    /// Power factor.
    PowerFactor,
    /// Apparent power.
    ApparentPower,
    /// Mass.
    Mass,
    /// Harmonic ratio.
    HarmonicRatio,
}

impl TryFrom<i32> for MeasuredQuantity {
    type Error = SrError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            10000 => Ok(Self::Voltage),
            10001 => Ok(Self::Current),
            10002 => Ok(Self::Resistance),
            10003 => Ok(Self::Capacitance),
            10004 => Ok(Self::Temperature),
            10005 => Ok(Self::Frequency),
            10006 => Ok(Self::DutyCycle),
            10007 => Ok(Self::Continuity),
            10008 => Ok(Self::PulseWidth),
            10009 => Ok(Self::Conductance),
            10010 => Ok(Self::Power),
            10011 => Ok(Self::Gain),
            10012 => Ok(Self::SoundPressureLevel),
            10013 => Ok(Self::CarbonMonoxide),
            10014 => Ok(Self::RelativeHumidity),
            10015 => Ok(Self::Time),
            10016 => Ok(Self::WindSpeed),
            10017 => Ok(Self::Pressure),
            10018 => Ok(Self::ParallelInductance),
            10019 => Ok(Self::ParallelCapacitance),
            10020 => Ok(Self::ParallelResistance),
            10021 => Ok(Self::SeriesInductance),
            10022 => Ok(Self::SeriesCapacitance),
            10023 => Ok(Self::SeriesResistance),
            10024 => Ok(Self::DissipationFactor),
            10025 => Ok(Self::QualityFactor),
            10026 => Ok(Self::PhaseAngle),
            10027 => Ok(Self::Difference),
            10028 => Ok(Self::Count),
            10029 => Ok(Self::PowerFactor),
            10030 => Ok(Self::ApparentPower),
            10031 => Ok(Self::Mass),
            10032 => Ok(Self::HarmonicRatio),
            _ => Err(SrError::SrErrNA),
        }
    }
}

impl MeasuredQuantity {
    /// Converts the enum to a string value.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Voltage => "Voltage",
            Self::Current => "Current",
            Self::Resistance => "Resistance",
            Self::Capacitance => "Capacitance",
            Self::Temperature => "Temperature",
            Self::Frequency => "Frequency",
            Self::DutyCycle => "Duty cycle",
            Self::Continuity => "Continuity",
            Self::PulseWidth => "Pulse width",
            Self::Conductance => "Conductance",
            Self::Power => "Power",
            Self::Gain => "Gain",
            Self::SoundPressureLevel => "Sound pressure level",
            Self::CarbonMonoxide => "Carbon monoxide",
            Self::RelativeHumidity => "Relative humidity",
            Self::Time => "Time",
            Self::WindSpeed => "Wind speed",
            Self::Pressure => "Pressure",
            Self::ParallelInductance => "Parallel inductance",
            Self::ParallelCapacitance => "Parallel capacitance",
            Self::ParallelResistance => "Parallel resistance",
            Self::SeriesInductance => "Series inductance",
            Self::SeriesCapacitance => "Series capacitance",
            Self::SeriesResistance => "Series resistance",
            Self::DissipationFactor => "Dissipation factor",
            Self::QualityFactor => "Quality factor",
            Self::PhaseAngle => "Phase angle",
            Self::Difference => "Difference",
            Self::Count => "Count",
            Self::PowerFactor => "Power factor",
            Self::ApparentPower => "Apparent power",
            Self::Mass => "Mass",
            Self::HarmonicRatio => "Harmonic ratio",
        }
    }
}

/// Flags associated with a MeasuredQuantity
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasuredQuantityFlag {
    /// Voltage measurement is alternating current (AC).
    Ac = 0x01,
    /// Voltage measurement is direct current (DC).
    Dc = 0x02,
    /// This is a true RMS measurement.
    Rms = 0x04,
    /// Value is voltage drop across a diode, or NAN.
    Diode = 0x08,
    /// Device is in "hold" mode (repeating the last measurement).
    Hold = 0x10,
    /// Device is in "max" mode, only updating upon a new max value.
    Max = 0x20,
    /// Device is in "min" mode, only updating upon a new min value.
    Min = 0x40,
    /// Device is in autoranging mode.
    Autorange = 0x80,
    /// Device is in relative mode.
    Relative = 0x100,
    /// Sound pressure level is A-weighted in the frequency domain,
    /// according to IEC 61672:2003.
    SplFreqWeightA = 0x200,
    /// Sound pressure level is C-weighted in the frequency domain,
    /// according to IEC 61672:2003.
    SplFreqWeightC = 0x400,
    /// Sound pressure level is Z-weighted (i.e. not at all) in the
    /// frequency domain, according to IEC 61672:2003.
    SplFreqWeightZ = 0x800,
    /// Sound pressure level is not weighted in the frequency domain,
    /// albeit without standards-defined low and high frequency limits.
    SplFreqWeightFlat = 0x1000,
    /// Sound pressure level measurement is S-weighted (1s) in the
    /// time domain
    SplTimeWeightS = 0x2000,
    /// Sound pressure level measurement is F-weighted (125ms) in the
    /// time domain.
    SplTimeWeightF = 0x4000,
    /// Sound pressure level is time-averaged (LAT), also known as
    /// Equivalent Continuous A-weighted Sound Level (LEQ).
    SplLat = 0x8000,
    /// Sound pressure level represented as a percentage of measurements
    /// that were over a preset alarm level.
    SplPctOverAlarm = 0x10000,
    /// Time is duration (as opposed to epoch, ...).
    Duration = 0x20000,
    /// Device is in "avg" mode, averaging upon each new value.
    Avg = 0x40000,
    /// Reference value shown.
    Reference = 0x80000,
    /// Unstable value (hasn't settled yet).
    Unstable = 0x100000,
    /// Measurement is four wire (e.g. Kelvin connection).
    FourWire = 0x200000,
}

impl TryFrom<u64> for MeasuredQuantityFlag {
    type Error = SrError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Ac),
            0x02 => Ok(Self::Dc),
            0x04 => Ok(Self::Rms),
            0x08 => Ok(Self::Diode),
            0x10 => Ok(Self::Hold),
            0x20 => Ok(Self::Max),
            0x40 => Ok(Self::Min),
            0x80 => Ok(Self::Autorange),
            0x100 => Ok(Self::Relative),
            0x200 => Ok(Self::SplFreqWeightA),
            0x400 => Ok(Self::SplFreqWeightC),
            0x800 => Ok(Self::SplFreqWeightZ),
            0x1000 => Ok(Self::SplFreqWeightFlat),
            0x2000 => Ok(Self::SplTimeWeightS),
            0x4000 => Ok(Self::SplTimeWeightF),
            0x8000 => Ok(Self::SplLat),
            0x10000 => Ok(Self::SplPctOverAlarm),
            0x20000 => Ok(Self::Duration),
            0x40000 => Ok(Self::Avg),
            0x80000 => Ok(Self::Reference),
            0x100000 => Ok(Self::Unstable),
            0x200000 => Ok(Self::FourWire),
            _ => Err(SrError::SrErrNA),
        }
    }
}

impl MeasuredQuantityFlag {
    /// Converts the enum into a String value
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ac => "AC",
            Self::Dc => "DC",
            Self::Rms => "RMS",
            Self::Diode => "Diode",
            Self::Hold => "Hold",
            Self::Max => "Max",
            Self::Min => "Min",
            Self::Autorange => "Autorange",
            Self::Relative => "Relative",
            Self::SplFreqWeightA => "SPL frequency weight A",
            Self::SplFreqWeightC => "SPL frequency weight C",
            Self::SplFreqWeightZ => "SPL frequency weight Z",
            Self::SplFreqWeightFlat => "SPL frequency weight flat",
            Self::SplTimeWeightS => "SPL time weight S",
            Self::SplTimeWeightF => "SPL time weight F",
            Self::SplLat => "SPL LAT",
            Self::SplPctOverAlarm => "SPL percentage over alarm",
            Self::Duration => "Duration",
            Self::Avg => "Average",
            Self::Reference => "Reference",
            Self::Unstable => "Unstable",
            Self::FourWire => "Four wire",
        }
    }
}

impl TryFrom<i32> for GVariantDataType {
    type Error = SrError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            10000 => Ok(GVariantDataType::Uint64),
            10001 => Ok(GVariantDataType::String),
            10002 => Ok(GVariantDataType::Bool),
            10003 => Ok(GVariantDataType::Float),
            10004 => Ok(GVariantDataType::RationalPeriod),
            10005 => Ok(GVariantDataType::RationalVolt),
            10006 => Ok(GVariantDataType::KeyValue),
            10007 => Ok(GVariantDataType::Uint64Range),
            10008 => Ok(GVariantDataType::DoubleRange),
            10009 => Ok(GVariantDataType::Int32),
            10010 => Ok(GVariantDataType::MQ),
            _ => Err(SrError::SrErrNA),
        }
    }
}
