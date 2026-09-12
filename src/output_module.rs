//! Output module

use std::ffi::CStr;
use std::mem;
use std::ptr::{null, null_mut};

use glib::ffi::GVariant;
use libsigrok_sys::sigrok::{self as sr, sr_dev_inst, sr_option, sr_output, sr_output_module};

use crate::types::SrError;
use crate::utils::{gslist_to_vec, gvariant_type_string};

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

        let variant: glib::Variant = unsafe {
            glib::translate::from_glib_none(option.def as *mut GVariant)
        };

        let possible_values_gvar: Vec<*mut GVariant> = gslist_to_vec(option.values);

        let mut possible_values: Vec<String> = Vec::new();
        for gvar in possible_values_gvar {
            let variant: glib::Variant = unsafe {
                glib::translate::from_glib_none(gvar as *mut GVariant)
            };
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

/// TODO
#[derive(Debug, Clone)]
pub struct OutputModule {
    /// Pointer to a C-FFI output module.
    p_output: *const sr_output,
    /// Output module ID, used to uniquely identify the output module.
    id: String,
    /// Name of the output module. Just informative.
    name: String,
    /// Output module's description. Just informative.
    description: String,
    /// TODO
    options: Vec<SrOption>,
}

impl OutputModule {
    /// Returns a list of all available output modules.
    pub fn scan() -> Vec<OutputModule> {
        let mut modules: Vec<OutputModule> = Vec::new();

        let mut p_p_modules: *mut *const sr_output_module = unsafe { sr::sr_output_list() };
        let mut p_module: *const sr_output_module = unsafe { *p_p_modules };

        while p_module != null() {
            modules.push(OutputModule::new(p_module, null(), String::new()));

            p_p_modules =  unsafe{p_p_modules.offset(1)};
            p_module = unsafe { *p_p_modules };
        }

        modules
    }

    /// Creates a new output module from the raw FFI-C pointer.
    ///
    /// This function will panic! if the pointer is NULL.
    fn new(p_module: *const sr_output_module, p_device: *const sr_dev_inst, mut filename: String) -> OutputModule {
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
            tmp_p_p_options = unsafe{tmp_p_p_options.offset(1)};
            p_option = unsafe { *tmp_p_p_options };
        }

        unsafe{sr::sr_output_options_free(p_p_options)};

        let p_output: *const sr_output = if p_device != null() {
            unsafe{sr::sr_output_new(p_module, null_mut(), p_device, filename.as_mut_ptr() as *mut i8)}
        } else {
            null()
        };

        OutputModule {
            p_output: p_output,
            name: name,
            description: description,
            id: id,
            options: options,
        }
    }
}

impl Drop for OutputModule {
    fn drop(&mut self) {
        if self.p_output != null() {
            unsafe{sr::sr_output_free(self.p_output)};
        }
    }
}

impl TryFrom<String> for OutputModule {
    type Error = SrError;

    fn try_from(mut id: String) -> Result<Self, Self::Error> {
        let p_output_mod: *const sr_output_module = unsafe{sr::sr_output_find(id.as_mut_ptr() as *mut i8)};

        if p_output_mod == null() {
            Err(SrError::SrDeviceNotFound)
        } else {
            Ok(Self::new(p_output_mod, null(), String::new()))
        }
    }
}


// #[cfg(test)]
// mod tests {
//     use libsigrok_sys::sigrok::sr_context;

//     use crate::{sr_try, types::SrError};
//     use super::*;

//     #[test]
//     fn test_output_mod_scan() -> Result<(), SrError> {
//        let mut context: *mut sr_context = null_mut();
//         sr_try!(sr::sr_init(&mut context));
//         let out_modules = OutputModule::scan();
//         let ascii_out_module = out_modules.iter().find(|m| m.name == "ASCII").unwrap();

//         assert!(ascii_out_module.id == "ascii");
//         assert!(ascii_out_module.description == "ASCII art logic data");
//         assert!(ascii_out_module.options[0].id == "width");
//         assert!(ascii_out_module.options[0].default_value == "74");
//         assert!(ascii_out_module.options[1].id == "charset");

//         dbg!(out_modules);
//         panic!("hi");

//         sr_try!(sr::sr_exit(context));
//         Ok(())
//     }
// }