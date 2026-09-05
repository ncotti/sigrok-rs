//! This module includes utility functions to work with some of glib and
//! libsigrok data types more easily
//!
//!

use std::ptr::null_mut;

use libsigrok_sys::sigrok::{self as sr, GArray};

use sr::GSList;

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
    let sizeof: u32 = std::mem::size_of::<T>() as u32;
    let mut output: Vec<T> = Vec::new();

    if array == null_mut() {
        return output;
    }

    let array: GArray = unsafe { *array };

    let mut real_len: u32 = array.len / sizeof;
    let mut real_data: *mut T = array.data.cast();

    while real_len > 0 {
        output.push(unsafe { *real_data });

        real_data = unsafe { real_data.offset(1) };
        real_len -= 1;
    }

    output
}
