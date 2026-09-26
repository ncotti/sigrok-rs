//! Output module

use std::ffi::CStr;
use std::ptr::{null, null_mut};

use glib::ffi::GVariant;
use libsigrok_sys::sigrok::{
    self as sr, sr_dev_inst, sr_option, sr_output, sr_output_module, sr_session,
};

use crate::types::SrError;
use crate::utils::gslist_to_vec;

use std::fmt;
use std::path::Path;

/// Generic option struct used by various subsystems, equivalent to `sr_option`.
#[derive(Debug, Clone)]
pub struct SrOption {
    /// Option ID, used to uniquely identify the output module.
    id: String,
    /// Option's name. Just informative.
    name: String,
    /// Option's description. Just informative.
    description: String,
    // Default value for this option
    default_value: String,
    // List of possible values, if this is an option with few values.
    possible_values: Vec<String>,
}

impl SrOption {
    /// Builds a new SrOption from a C-FFI pointer.
    ///
    /// This function will panic!() if the pointer is null().
    pub fn new(p_option: *const sr_option) -> SrOption {
        if p_option == null() {
            panic!("SrOption::new(), p_option was NULL");
        }
        let option: sr_option = unsafe { *p_option };
        let name: String = unsafe { CStr::from_ptr(option.name) }
            .to_string_lossy()
            .to_string();
        let id: String = unsafe { CStr::from_ptr(option.id) }
            .to_string_lossy()
            .to_string();
        let description: String = unsafe { CStr::from_ptr(option.desc) }
            .to_string_lossy()
            .to_string();

        let variant: glib::Variant =
            unsafe { glib::translate::from_glib_none(option.def as *mut GVariant) };

        let possible_values_gvar: Vec<*mut GVariant> = gslist_to_vec(option.values);

        let mut possible_values: Vec<String> = Vec::new();
        for gvar in possible_values_gvar {
            let variant: glib::Variant =
                unsafe { glib::translate::from_glib_none(gvar as *mut GVariant) };
            possible_values.push(variant.print(false).to_string());
        }

        SrOption {
            name: name,
            id: id,
            description: description,
            default_value: variant.print(false).to_string(),
            possible_values: possible_values,
        }
    }
}

impl fmt::Display for SrOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Option: {} ({})", self.name, self.id)?;
        writeln!(f, "  {}", self.description)?;
        writeln!(f, "  Default value: {}", self.default_value)?;
        if !self.possible_values.is_empty() {
            writeln!(f, "  Possible values: {}", self.possible_values.join(" "))?;
        }
        Ok(())
    }
}

/// The output module dictates how the information retrieved from the device's
/// channels will be stored.
pub struct OutputModule {
    /// Pointer to a C-FFI output module.
    p_output: *const sr_output,
    /// A copy of the C-FFI session pointer.
    ///
    /// This pointer is required so that the the callback may stop the session
    /// on a different thread.
    pub p_session: *mut sr_session,
    /// All info related to the output module
    info: OutputModuleInfo,
    /// Output will be written to this file
    filename: String,
    /// Last batch of samples that has been read from the device's
    /// Logic channel.
    ///
    /// Each bit represents the value from the logic channel, from
    /// D7 (MSB) to D0 (LSB).
    pub data: Vec<u8>,
}

impl OutputModule {
    /// Creates a new output module.
    ///
    /// * `id`: Which output module to select. The list of possible output
    /// modules can be obtained from the `OutputModule::scan()` method.
    ///
    /// * `filename`: Output file where information will be stored.
    ///
    /// * `p_device`: C-FFI pointer to the device connected to the output
    /// module. Although the device and the output format are independent, from
    /// the device details like the sample rate and the number of channels are
    /// extracted.
    pub fn new(
        id: &str,
        filename: impl AsRef<Path>,
        p_device: *const sr_dev_inst,
        p_session: *mut sr_session,
    ) -> Result<OutputModule, SrError> {
        // The filename requested by Sigrok's API seems to not be used, so
        // just give it a "mock name" just in case.
        if filename.as_ref().is_file() {
            std::fs::remove_file(&filename).unwrap();
        }

        if let Some(parent) = filename.as_ref().parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        let filename = filename.as_ref().to_string_lossy().to_string();
        let sigrok_filename = format!("sr_{}", &filename);

        let sigrok_filename: std::ffi::CString =
            std::ffi::CString::new(sigrok_filename).map_err(|_| SrError::SrNull)?;
        let info: OutputModuleInfo = OutputModuleInfo::try_from(id)?;

        let p_output: *const sr_output = unsafe {
            sr::sr_output_new(
                info.p_module,
                null_mut(),
                p_device,
                sigrok_filename.as_ptr().cast_mut(),
            )
        };

        Ok(OutputModule {
            p_output: p_output,
            p_session: p_session,
            info: info,
            filename: filename,
            data: Vec::new(),
        })
    }

    /// Returns the output module pointer
    pub fn get_pointer(&self) -> *const sr_output {
        self.p_output
    }

    /// Returns the name of the file where data will be stored.
    pub fn get_filename(&self) -> &String {
        &self.filename
    }

    /// Returns the associated "info" struct for the output module.
    pub fn get_info(&self) -> &OutputModuleInfo {
        &self.info
    }
}

impl fmt::Display for OutputModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_info())
    }
}

/// Stores all information related to an output module, but without
/// explicitly creating one.
///
/// This distinction between `OutputModule` and `OutputModuleInfo` comes from
/// the fact that, to create a new `OutputModule`, a device is required.
/// The separation between the two allows to present all available output
/// structures to the user without them having an instantiated device.
#[derive(Debug, Clone)]
pub struct OutputModuleInfo {
    /// Raw C-FFI pointer to the output module info.
    p_module: *const sr_output_module,
    /// Output module ID, used to uniquely identify the output module.
    pub id: String,
    /// Name of the output module. Just informative.
    pub name: String,
    /// Output module's description. Just informative.
    pub description: String,
    /// Configuration options.
    pub options: Vec<SrOption>,
}

impl Drop for OutputModule {
    fn drop(&mut self) {
        if self.p_output != null() {
            unsafe { sr::sr_output_free(self.p_output) };
        }
    }
}

impl OutputModuleInfo {
    /// Returns information about all available output modules.
    pub fn scan() -> Vec<OutputModuleInfo> {
        let mut modules: Vec<OutputModuleInfo> = Vec::new();

        let mut p_p_modules: *mut *const sr_output_module = unsafe { sr::sr_output_list() };
        let mut p_module: *const sr_output_module = unsafe { *p_p_modules };

        while p_module != null() {
            modules.push(OutputModuleInfo::new(p_module));

            p_p_modules = unsafe { p_p_modules.offset(1) };
            p_module = unsafe { *p_p_modules };
        }

        modules
    }

    /// Creates a new output module from the raw FFI-C pointer.
    ///
    /// This function will panic! if the pointer is NULL.
    fn new(p_module: *const sr_output_module) -> OutputModuleInfo {
        if p_module == null() {
            panic!("OutputModule::new(). p_module should not be NULL");
        }

        let name: String = unsafe { CStr::from_ptr(sr::sr_output_name_get(p_module)) }
            .to_string_lossy()
            .to_string();
        let description: String =
            unsafe { CStr::from_ptr(sr::sr_output_description_get(p_module)) }
                .to_string_lossy()
                .to_string();
        let id: String = unsafe { CStr::from_ptr(sr::sr_output_id_get(p_module)) }
            .to_string_lossy()
            .to_string();

        let p_p_options: *mut *const sr_option = unsafe { sr::sr_output_options_get(p_module) };
        let mut p_option: *const sr_option = if p_p_options == null_mut() {
            null()
        } else {
            unsafe { *p_p_options }
        };

        let mut options: Vec<SrOption> = Vec::new();
        let mut tmp_p_p_options: *mut *const sr_option = p_p_options;

        while p_option != null() {
            options.push(SrOption::new(p_option));
            tmp_p_p_options = unsafe { tmp_p_p_options.offset(1) };
            p_option = unsafe { *tmp_p_p_options };
        }

        unsafe { sr::sr_output_options_free(p_p_options) };

        OutputModuleInfo {
            p_module: p_module,
            name: name,
            description: description,
            id: id,
            options: options,
        }
    }
}

impl TryFrom<&str> for OutputModuleInfo {
    type Error = SrError;

    fn try_from(id: &str) -> Result<Self, Self::Error> {
        let id = std::ffi::CString::new(id).map_err(|_| SrError::SrNotFound)?;
        let p_output_mod: *const sr_output_module =
            unsafe { sr::sr_output_find(id.as_ptr().cast_mut()) };

        if p_output_mod == null() {
            Err(SrError::SrNotFound)
        } else {
            Ok(Self::new(p_output_mod))
        }
    }
}

impl fmt::Display for OutputModuleInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Output module: {} ({})", self.name, self.id)?;
        writeln!(f, "  {}", self.description)?;

        for option in &self.options {
            write!(f, "  {}", option)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use libsigrok_sys::sigrok::sr_context;

    use super::*;
    use crate::{sr_try, types::SrError};

    #[test]
    fn test_output_module_info_scan() -> Result<(), SrError> {
        let mut context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut context));
        let out_infos: Vec<OutputModuleInfo> = OutputModuleInfo::scan();
        let ascii_info = out_infos.iter().find(|m| m.name == "ASCII").unwrap();

        assert!(ascii_info.id == "ascii");
        assert!(ascii_info.description == "ASCII art logic data");
        assert!(ascii_info.options[0].id == "width");
        assert!(ascii_info.options[0].default_value == "74");
        assert!(ascii_info.options[1].id == "charset");

        sr_try!(sr::sr_exit(context));
        Ok(())
    }

    #[test]
    fn test_output_module_info_tryfrom() -> Result<(), SrError> {
        let result = OutputModuleInfo::try_from("xxxx");
        assert!(result.is_err());
        assert!(result.unwrap_err() == SrError::SrNotFound);

        let info = OutputModuleInfo::try_from("null")?;
        assert!(info.id == "null");
        Ok(())
    }
}
