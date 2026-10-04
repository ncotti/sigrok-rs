// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! Generic protocol decoder

pub mod spi_decoder;

use std::ffi::CStr;
use std::ptr::addr_of_mut;
use std::ptr::null;
use std::ptr::null_mut;
use std::str::FromStr;
use std::time::Duration;

use libsigrok_sys::sigrokdecode as srd;
use libsigrok_sys::sigrokdecode::srd_decoder;
use libsigrok_sys::sigrokdecode::srd_decoder_inst;
use libsigrok_sys::sigrokdecode::srd_session;

use crate::LogLevel;
use crate::sr_try;

use crate::SrError;
use crate::utils::gslist_to_vec;

/// Generic decoder struct.
///
/// The generic decoder handles basic session initialization and loads
/// the decoder modules.
///
/// Other specific decoder must implement their own callbacks with:
///
/// ```ignore
/// sr_try!(srd::srd_pd_output_callback_add(
///     decoder.p_session,
///     srd::srd_output_type_SRD_OUTPUT_PYTHON as i32,
///     Some(decoder_callback), cb_data));`
/// ```
pub struct Decoder {
    p_session: *mut srd_session,
    p_decoder_instance: *mut srd_decoder_inst,
}

impl Decoder {
    /// Creates a new generic decoder.
    pub fn new(id: &str) -> Result<Self, SrError> {
        unsafe { srd::srd_init(null()) };
        sr_try!(srd::srd_log_callback_set_default());
        sr_try!(srd::srd_log_loglevel_set(LogLevel::LogErr as i32));

        sr_try!(srd::srd_decoder_load_all());

        let mut p_session: *mut srd_session = null_mut();
        sr_try!(srd::srd_session_new(addr_of_mut!(p_session)));

        let id_cstr = std::ffi::CString::from_str(id).unwrap();

        let p_decoder_instance: *mut srd_decoder_inst =
            unsafe { srd::srd_inst_new(p_session, id_cstr.as_ptr().cast(), null_mut()) };
        if p_decoder_instance == null_mut() {
            unsafe { srd::srd_exit() };
            return Err(SrError::SrNull);
        }

        Ok(Decoder {
            p_session: p_session,
            p_decoder_instance: p_decoder_instance,
        })
    }

    /// Returns a list with the ids of all available decoders.
    pub fn scan() -> Result<Vec<String>, SrError> {
        sr_try!(srd::srd_init(null()));
        sr_try!(srd::srd_decoder_load_all());

        let p_decoder_list: *mut srd::_GSList = unsafe { srd::srd_decoder_list() }.cast_mut();

        if p_decoder_list == null_mut() {
            return Err(SrError::SrNull);
        }

        let p_decoder_list: Vec<*mut srd_decoder> = gslist_to_vec(p_decoder_list.cast());

        let mut ids: Vec<String> = Vec::new();
        for p_decoder in p_decoder_list {
            let decoder: srd_decoder = unsafe { *p_decoder };
            let id = unsafe { CStr::from_ptr(decoder.id) }
                .to_string_lossy()
                .to_string();

            ids.push(id);
        }

        // We are purposefully not unloading the decoders, since
        // it breaks the application later on;
        Ok(ids)
    }

    /// Starts a new decoding session.
    ///
    /// * `data`: A vector with samples for 8 logical channels. Ecah bit
    /// corresponds with the value for that channel, with the LSB bit
    /// corresponding to channel "D0".
    pub fn write(&mut self, mut data: Vec<u8>) -> Result<(), SrError> {
        sr_try!(srd::srd_session_start(self.p_session));
        sr_try!(srd::srd_session_send(
            self.p_session,
            0,
            data.len() as u64,
            data.as_mut_ptr(),
            data.len() as u64,
            1
        ));
        sr_try!(srd::srd_session_terminate_reset(self.p_session));
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
        unsafe { srd::srd_session_destroy(self.p_session) };
        // TODO: exiting the decoder library creates bugs with multiple
        // tests running on the same process.
        //unsafe { srd::srd_exit() };
    }
}

/// Represents a captured sample from a Logic Analyzer, including the sample
/// number and time when the data was taken.
#[derive(Debug)]
pub struct DataSample {
    /// Electrical value read by the device.
    pub value: u8,
    /// Sample number at which the value was first read.
    pub start_sample: u64,
    /// Sample number at which the value was last read.
    pub end_sample: u64,
}

impl DataSample {
    /// Returns the absolute time when the data sample was first read.
    pub fn start_time(&self, samplerate: u64) -> Duration {
        Duration::from_secs_f64((self.start_sample as f64) / (samplerate as f64))
    }

    /// Returns the absolute time when the data sample was last read.
    pub fn end_time(&self, samplerate: u64) -> Duration {
        Duration::from_secs_f64((self.end_sample as f64) / (samplerate as f64))
    }
}

impl PartialEq<u8> for DataSample {
    fn eq(&self, other: &u8) -> bool {
        self.value == *other
    }
}
