use std::ffi::CStr;
use std::mem;
use std::ptr::{null, null_mut};

use libsigrok_sys::sigrok::{self as sr, sr_option, sr_output_module};

#[derive(Debug, Clone)]
pub struct SrOption {
    name: String,
    description: String,
    id: String,
}

impl SrOption {
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

        SrOption {
            name: name,
            id: id,
            description: description,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutputModule {
    name: String,
    description: String,
    id: String,
    file_extensions: Vec<String>,
    options: Vec<SrOption>,
}

impl OutputModule {
    /// Returns a list of all possible output modules there are.
    pub fn scan() -> Vec<OutputModule> {
        let mut modules: Vec<OutputModule> = Vec::new();

        let mut p_p_modules: *mut *const sr_output_module = unsafe { sr::sr_output_list() };
        let mut p_module: *const sr_output_module = unsafe { *p_p_modules };

        while p_module != null() {
            modules.push(OutputModule::new(p_module));

            // Point to the next driver by moving the numerical value of the
            // memory address
            p_p_modules = ((p_p_modules as usize) + mem::size_of::<*const sr_output_module>())
                as *mut *const sr_output_module;
            p_module = unsafe { *p_p_modules };
        }

        modules
    }

    /// Creates a new output module from the raw FFI-C pointer.
    ///
    /// This function will panic! if the pointer is NULL.
    fn new(p_module: *const sr_output_module) -> OutputModule {
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

        let mut file_extensions: Vec<String> = Vec::new();

        let mut p_p_extension: *const *const i8 = unsafe { sr::sr_output_extensions_get(p_module) };
        let mut p_extension: *const i8 = if p_p_extension == null() {
            null()
        } else {
            unsafe { *p_p_extension }
        };

        while p_extension != null() {
            let extension: String = unsafe { CStr::from_ptr(p_extension) }
                .to_string_lossy()
                .to_string();
            file_extensions.push(extension);

            p_p_extension =
                ((p_p_extension as usize) + mem::size_of::<*const i8>()) as *const *const i8;
            p_extension = unsafe { *p_p_extension };
        }

        let mut p_p_options: *mut *const sr_option = unsafe { sr::sr_output_options_get(p_module) };
        let mut p_option: *const sr_option = if p_p_options == null_mut() {
            null()
        } else {
            unsafe { *p_p_options }
        };

        let mut options: Vec<SrOption> = Vec::new();

        while p_option != null() {
            options.push(SrOption::new(p_option));

            p_p_options = ((p_p_options as usize) + mem::size_of::<*const sr_option>())
                as *mut *const sr_option;
            p_option = unsafe { *p_p_options };
        }

        OutputModule {
            name: name,
            description: description,
            id: id,
            file_extensions: file_extensions,
            options: options,
        }
    }
}

// impl TryFrom<String> for OutputModule {
//     type Error = &'static str;

//     fn try_from(value: String) -> Result<Self, Self::Error> {
//         // scan, see if any of the output modules matches
//         // the name, description, id or file_extension
//         // Return that output module
//     }
// }
