// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! Protocol decoder (TODO doc)

pub mod spi_decoder;

use std::ffi::CStr;
use std::ffi::c_void;
use std::ptr::addr_of_mut;
use std::ptr::null;
use std::ptr::null_mut;
use std::str::FromStr;
use std::time::Duration;

use libsigrok_sys::sigrokdecode::srd_decoder;
use libsigrok_sys::sigrokdecode::srd_decoder_inst;
use libsigrok_sys::sigrokdecode::srd_pd_output;
use libsigrok_sys::sigrokdecode::srd_proto_data;
use libsigrok_sys::sigrokdecode::srd_session;
use libsigrok_sys::sigrokdecode as srd;

use crate::LogLevel;
use crate::sr_try;

use crate::SrError;
use crate::utils::gslist_to_vec;

pub struct Decoder {
    p_session: *mut srd_session,
    p_decoder_instance: *mut srd_decoder_inst,
}

impl Decoder {
    pub fn new(id: &str) -> Result<Self, SrError> {
        unsafe{srd::srd_init(null())};
        sr_try!(srd::srd_log_callback_set_default());
        sr_try!(srd::srd_log_loglevel_set(LogLevel::LogSpew as i32));

        sr_try!(srd::srd_decoder_load_all());

        let mut p_session: *mut srd_session = null_mut();
        sr_try!(srd::srd_session_new(addr_of_mut!(p_session)));

        sr_try!(srd::srd_pd_output_callback_add(p_session, srd::srd_output_type_SRD_OUTPUT_ANN as i32, Some(decoder_annotation_callback), null_mut()));
        sr_try!(srd::srd_pd_output_callback_add(p_session, srd::srd_output_type_SRD_OUTPUT_META as i32, Some(decoder_meta_callback), null_mut()));
        sr_try!(srd::srd_pd_output_callback_add(p_session, srd::srd_output_type_SRD_OUTPUT_BINARY as i32, Some(decoder_binary_callback), null_mut()));
        sr_try!(srd::srd_pd_output_callback_add(p_session, srd::srd_output_type_SRD_OUTPUT_PYTHON as i32, Some(decoder_python_callback), null_mut()));


        let id_cstr = std::ffi::CString::from_str(id).unwrap();

        let p_decoder_instance: *mut srd_decoder_inst = unsafe { srd::srd_inst_new(p_session, id_cstr.as_ptr().cast(), null_mut())};
        if p_decoder_instance == null_mut() {
            unsafe{srd::srd_exit()};
            return Err(SrError::SrNull);
        }

        Ok(Decoder{
            p_session: p_session,
            p_decoder_instance: p_decoder_instance,
        })
    }

    /// Returns a list with the ids of all available decoders.
    pub fn scan() -> Result<Vec<String>, SrError> {
        sr_try!(srd::srd_init(null()));
        sr_try!(srd::srd_decoder_load_all());

        let p_decoder_list: *mut srd::_GSList = unsafe{srd::srd_decoder_list()}.cast_mut();

        if p_decoder_list == null_mut() {
            return Err(SrError::SrNull);
        }

        let p_decoder_list: Vec<*mut srd_decoder> =  gslist_to_vec(p_decoder_list.cast());

        let mut ids: Vec<String> = Vec::new();
        for p_decoder in p_decoder_list {
            let decoder: srd_decoder = unsafe{*p_decoder};
            let id = unsafe { CStr::from_ptr(decoder.id) }
                .to_string_lossy()
                .to_string();

            ids.push(id);
        }

        //unsafe{srd::srd_exit()};
        Ok(ids)
    }

    pub fn write(&mut self, mut data: Vec<u8>) -> Result<(), SrError> {
        sr_try!(srd::srd_session_start(self.p_session));

        sr_try!(srd::srd_session_send(self.p_session, 0, data.len() as u64, data.as_mut_ptr(), data.len() as u64, 1));

        std::thread::sleep(Duration::from_millis(1000));

        Ok(())
    }
}

impl TryFrom<&str> for Decoder {
    type Error = SrError;

    fn try_from(id: &str) -> Result<Self, Self::Error> {
        Self::new(id)
    }
}

impl TryFrom<&String> for Decoder {
    type Error = SrError;

    fn try_from(device_id: &String) -> Result<Self, SrError> {
        Self::try_from(device_id.as_str())
    }
}

impl TryFrom<String> for Decoder {
    type Error = SrError;

    fn try_from(device_id: String) -> Result<Self, SrError> {
        Self::try_from(&device_id)
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe{srd::srd_session_destroy(self.p_session)};
        unsafe{srd::srd_exit()};
    }
}

extern "C" fn decoder_annotation_callback(
    p_data: *mut srd_proto_data,
    _cb_data: *mut c_void
) {
    if p_data == null_mut() {
        return;
    }

    let p_data: srd_proto_data = unsafe {*p_data};

    let _p_output: *mut srd_pd_output = p_data.pdo;

    println!("decoder_annotation_callback");
}

extern "C" fn decoder_binary_callback(
    _p_data: *mut srd_proto_data,
    _cb_data: *mut c_void
) {
    println!("decoder_binary_callback");
}

extern "C" fn decoder_meta_callback(
    _p_data: *mut srd_proto_data,
    _cb_data: *mut c_void
) {
    println!("decoder_meta_callback");
}

extern "C" fn decoder_python_callback(
    _p_data: *mut srd_proto_data,
    _cb_data: *mut c_void
) {
    println!("decoder_python_callback");
}