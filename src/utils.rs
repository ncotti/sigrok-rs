//! This module includes utility functions to work with some of glib and
//! libsigrok data types more easily
//!
//!

use std::{ffi::CStr, ptr::null_mut};

use glib::ffi::GVariant;
use libsigrok_sys::sigrok as sr;

use sr::{GArray, GSList};

use crate::{
    config_option::{GVariantDataType, MeasuredQuantity, MeasuredQuantityFlag},
    types::SrError,
};

/// Converts a `GSList` type to a Rust `Vec<*mut T>` type.
/// The returned vector may be empty.
///
/// `data` is originally represented as a void pointer, while this function
/// converts it to a pointer of the given `T` type. It does not clone nor
/// copy the contents of the data that is being pointed to.
///
/// The `GSList` struct is defined as follows:
/// ```rs
/// pub struct GSList {
///     pub data: *mut c_void,
///     pub next: *mut GSList,
/// }
/// ```

pub fn gslist_to_vec<T>(list: *mut GSList) -> Vec<*mut T> {
    let mut data: Vec<*mut T> = Vec::new();
    let mut p_list_node: *mut GSList = list;

    while p_list_node != null_mut() {
        let p_data: *mut T = unsafe { *p_list_node }.data.cast();

        data.push(p_data);
        p_list_node = unsafe { *p_list_node }.next;
    }

    data
}

/// Converts a `GArray` type into a Rust `Vec<T>` type.
/// The returned vector may be empty.
///
/// The `GArray` struct is defined as follows:
/// pub struct GArray {
///    pub data: *mut i8,
///    pub len: u32,
/// }
///
/// Although `data` is defined as a char pointer, the size of the data can
/// be of any length, that is why the `<T>` is provided as an argument.
pub fn garray_to_vec<T: Copy>(array: *mut GArray) -> Vec<T> {
    let mut output: Vec<T> = Vec::new();

    if array == null_mut() {
        return output;
    }

    let array: GArray = unsafe { *array };

    let mut elements: u32 = array.len;
    let mut real_data: *mut T = array.data.cast();

    while elements > 0 {
        output.push(unsafe { *real_data });

        real_data = unsafe { real_data.offset(1) };
        elements -= 1;
    }

    output
}

/// Extract the value from a GVariant to a String
///
/// GVariant is a variable that can hold any type. This function will try to
/// read the value within and return it as a String.
pub fn gvariant_to_string(
    data_type: GVariantDataType,
    gvar: *mut GVariant,
) -> Result<String, SrError> {
    Ok(match data_type {
        GVariantDataType::Bool => unsafe { glib::ffi::g_variant_get_boolean(gvar) }.to_string(),
        GVariantDataType::Float => unsafe { glib::ffi::g_variant_get_double(gvar) }.to_string(),
        GVariantDataType::DoubleRange => {
            todo!()
        }
        GVariantDataType::Int32 => unsafe { glib::ffi::g_variant_get_int32(gvar) }.to_string(),
        GVariantDataType::KeyValue => {
            todo!()
        }
        GVariantDataType::MQ => {
            let mq_type: *mut GVariant = unsafe { glib::ffi::g_variant_get_child_value(gvar, 0) };
            let mq_flag: *mut GVariant = unsafe { glib::ffi::g_variant_get_child_value(gvar, 1) };

            let mq_type = unsafe { glib::ffi::g_variant_get_uint32(mq_type) };
            let mq_flag = unsafe { glib::ffi::g_variant_get_uint64(mq_flag) };

            let mq_type = MeasuredQuantity::try_from(mq_type as i32)?.as_str();
            let mq_flag = MeasuredQuantityFlag::try_from(mq_flag)?.as_str();
            format!("{}, {}", mq_type, mq_flag)
        }
        GVariantDataType::RationalPeriod | GVariantDataType::RationalVolt => {
            todo!()
        }
        GVariantDataType::String => {
            unsafe { CStr::from_ptr(glib::ffi::g_variant_get_string(gvar, null_mut())) }
                .to_string_lossy()
                .to_string()
        }
        GVariantDataType::Uint64 => {
            let gvar_type = gvariant_type_string(gvar);
            if gvar_type == "t" {
                unsafe { glib::ffi::g_variant_get_uint64(gvar) }.to_string()
            } else if gvar_type == "{sv}" {
                // The "s" stands for string
                let gvar_string = unsafe { glib::ffi::g_variant_get_child_value(gvar, 0) };
                let gvar_string = unsafe {
                    CStr::from_ptr(glib::ffi::g_variant_get_string(gvar_string, null_mut()))
                }
                .to_string_lossy()
                .to_string();

                // The "v" stands for another GVariant
                let gvar_values = unsafe { glib::ffi::g_variant_get_child_value(gvar, 1) };
                let gvar_values = unsafe { glib::ffi::g_variant_get_variant(gvar_values) };

                // The inner value is a "at", i.e., an array of uint64_t
                let child_qtty = unsafe { glib::ffi::g_variant_n_children(gvar_values) };
                let mut values = String::from(gvar_string);
                for i in 0..child_qtty {
                    let gvar_value =
                        unsafe { glib::ffi::g_variant_get_child_value(gvar_values, i) };
                    let gvar_value =
                        unsafe { glib::ffi::g_variant_get_uint64(gvar_value) }.to_string();

                    values = format!("{}, {}", values, gvar_value);
                }

                values
            } else {
                String::new()
            }
        }
        GVariantDataType::Uint64Range => {
            todo!()
        }
    })
}

/// Returns the type of a GVariant type, as a string.
pub fn gvariant_type_string(gvar: *mut GVariant) -> String {
    let out = unsafe { glib::ffi::g_variant_get_type_string(gvar) };
    unsafe { CStr::from_ptr(out) }.to_string_lossy().to_string()
}
