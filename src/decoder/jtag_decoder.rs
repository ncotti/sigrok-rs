// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! JTAG decoder

use std::{ffi::c_void, ptr::null_mut};

use crate::{Decoder, SrError, sr_try};

use glib::{self};
use libsigrok_sys::sigrokdecode::{self as srd, srd_proto_data};

use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::decoder::DataSample;

/// Configuration options for JTAG decoder.
pub enum JTAGOptions {
    /// Sample rate.
    Samplerate(u64),
}

/// JTAG Channels.
pub enum JTAGChannels {
    /// Test Data In.
    TDI,
    /// Test Data Out.
    TDO,
    /// Test Clock
    TCK,
    /// Test Mode Select
    TMS,
    /// Test Reset
    TRST,
    /// System Reset
    SRST,
    /// Return Clock Signal
    RTCK,
}

impl JTAGChannels {
    fn as_str(&self) -> &str {
        match self {
            Self::TDI => "tdi",
            Self::TDO => "tdo",
            Self::TCK => "tck",
            Self::TMS => "tms",
            Self::TRST => "trst",
            Self::SRST => "srst",
            Self::RTCK => "rtck",
        }
    }
}

/// JTAG State machine's states.
#[derive(Debug, Default)]
pub enum JTAGState {
    /// Reset
    TestLogicReset,
    /// Idle
    RunTestIdle,
    /// Select DR Scan
    SelectDRScan,
    /// Capture DR
    CaptureDR,
    /// Shift DR
    ShiftDR,
    /// Exit 1 DR
    Exit1DR,
    /// Pause DR
    PauseDR,
    /// Exit 2 DR
    Exit2DR,
    /// Update DR.
    ///
    /// An update event will have attached the shifted values from TDI and TDO.
    UpdateDR(JTAGTdiTdo),
    /// Select IR Scan
    SelectIRScan,
    /// Capture IR
    CaptureIR,
    /// Shift IR
    ShiftIR,
    /// Exit 1 IR
    Exit1IR,
    /// Pause IR
    PauseIR,
    /// Exit 2 IR
    Exit2IR,
    /// Update IR
    ///
    /// An update event will have attached the shifted values from TDI and TDO.
    UpdateIR(JTAGTdiTdo),
    #[default]
    /// Unknown state
    Null,
}

/// JTAG TDI and TDO data received in the "update" states for either IR or DR.
#[derive(Debug, Default)]
pub struct JTAGTdiTdo {
    /// TDI data.
    pub tdi: Vec<u8>,
    /// TDO data.
    pub tdo: Vec<u8>,
    /// Sample information (start and end sample).
    pub sample: DataSample,
    /// How many bits were received / sent.
    ///
    /// If the received data is not a multiple of 8, then the TDI and TDO
    /// vectors will have zeros in the MSBs of the last byte.
    pub len_in_bits: usize,
}

impl From<&str> for JTAGState {
    fn from(state: &str) -> Self {
        match state {
            "TEST-LOGIC-RESET" => Self::TestLogicReset,
            "RUN-TEST/IDLE" => Self::RunTestIdle,
            "SELECT-DR-SCAN" => Self::SelectDRScan,
            "CAPTURE-DR" => Self::CaptureDR,
            "SHIFT-DR" => Self::ShiftDR,
            "EXIT1-DR" => Self::Exit1DR,
            "PAUSE-DR" => Self::PauseDR,
            "EXIT2-DR" => Self::Exit2DR,
            "UPDATE-DR" => Self::UpdateDR(JTAGTdiTdo::default()),
            "SELECT-IR-SCAN" => Self::SelectIRScan,
            "CAPTURE-IR" => Self::CaptureIR,
            "SHIFT-IR" => Self::ShiftIR,
            "EXIT1-IR" => Self::Exit1IR,
            "PAUSE-IR" => Self::PauseIR,
            "EXIT2-IR" => Self::Exit2IR,
            "UPDATE-IR" => Self::UpdateIR(JTAGTdiTdo::default()),
            _ => Self::Null,
        }
    }
}

/// All elements in an I2C communication frame:
///
/// A frame is composed by the following elements:
///
/// 1. A start condition.
/// 2. 7 address bits + rw bit.
/// 3. ACK (1) / NACK (0) bit.
/// 4. 8 data bits.
/// 5. ACK (1) / NACK (0) bit.
///
/// At this point, any number of data + ACK bits can be sent. Also, a new
/// repeated start condition can be received. That's why all values are
/// vectors.
///
/// 6. Single stop condition.
#[derive(Debug)]
pub struct JTAGData {
    pub state: JTAGState,
    pub tms: DataSample,
}

/// I2C Decoder struct.
pub struct JTAGDecoder {
    /// Internal decoder structure.
    decoder: Decoder,
    /// I2C transfers.
    data: Box<Vec<JTAGData>>,
}

impl JTAGDecoder {
    /// Creates a new SPIDecoder.
    ///
    /// * `options`: Configuration options for the SPI decoder.
    /// * `channels`: Which channels are read, and in which position. The
    /// order in which the channels are defined in here determine the channel
    /// number for that signal, starting from zero.
    pub fn new(
        options: Option<Vec<JTAGOptions>>,
        channels: Option<Vec<JTAGChannels>>,
    ) -> Result<Self, SrError> {
        let decoder = Decoder::new("jtag")?;

        let options = options.unwrap_or_default();

        for option in options {
            match option {
                JTAGOptions::Samplerate(samplerate) => {
                    sr_try!(srd::srd_session_metadata_set(
                        decoder.p_session,
                        srd::srd_configkey_SRD_CONF_SAMPLERATE as i32,
                        glib::ffi::g_variant_new_uint64(samplerate).cast()
                    ));
                }
            }
        }

        let channels = channels.unwrap_or_default();

        let channels_table: *mut glib::ffi::GHashTable = unsafe {
            glib::ffi::g_hash_table_new(Some(glib::ffi::g_str_hash), Some(glib::ffi::g_str_equal))
        };

        let mut counter: i32 = 0;

        for channel in channels {
            let key = std::ffi::CString::new(channel.as_str()).unwrap();

            let value = unsafe { glib::ffi::g_variant_new_int32(counter) };

            unsafe {
                glib::ffi::g_hash_table_insert(channels_table, key.into_raw().cast(), value.cast());
            }

            counter += 1;
        }

        if counter > 0 {
            sr_try!(srd::srd_inst_channel_set_all(
                decoder.p_decoder_instance,
                channels_table.cast(),
            ));
        }

        let data: Box<Vec<JTAGData>> = Box::new(Vec::new());

        let mut jtag_decoder = JTAGDecoder {
            decoder: decoder,
            data: data,
        };

        let cb_data = ((&mut *jtag_decoder.data) as *mut Vec<JTAGData>) as *mut c_void;

        sr_try!(srd::srd_pd_output_callback_add(
            jtag_decoder.decoder.p_session,
            srd::srd_output_type_SRD_OUTPUT_PYTHON as i32,
            Some(jtag_decoder_callback),
            cb_data
        ));

        Ok(jtag_decoder)
    }

    /// Writes to the JTAG Decoder and returns the decoded output.
    pub fn write(&mut self, data: Vec<u8>) -> Result<&Vec<JTAGData>, SrError> {
        self.decoder.write(data)?;
        Ok(&self.data)
    }
}

/// JTAG decoder callback.
extern "C" fn jtag_decoder_callback(p_data: *mut srd_proto_data, cb_data: *mut c_void) {
    if p_data == null_mut() {
        return;
    }
    let data: srd_proto_data = unsafe { *p_data };

    if data.data == null_mut() {
        return;
    }

    let output = unsafe { &mut *cb_data.cast::<Vec<JTAGData>>() };

    Python::attach(|py| {
        // Converts the void* to a Python object
        let obj: Bound<'_, PyAny> =
            unsafe { Bound::from_borrowed_ptr(py, data.data as *mut pyo3::ffi::PyObject) };

        // Python object to list.
        let list: &Bound<'_, PyList> = obj.cast::<PyList>().expect("It should only receive a list");

        // Get the first element of the Python list.
        // It should be a string with the packet type.
        let kind: String = list.get_item(0).unwrap().extract().unwrap();

        // Types of SPI packets
        match kind.as_ref() {
            "NEW STATE" => {
                // Received packet is a Python List with two values:
                // ['NEW STATE', 'STATE NAME']

                let state: String = list.get_item(1).unwrap().extract().unwrap();
                let state: JTAGState = JTAGState::from(state.as_ref());

                // TODO, get TMS value
                let jtag_data = JTAGData {
                    state: state,
                    tms: DataSample {
                        value: 0,
                        start_sample: data.start_sample,
                        end_sample: data.end_sample,
                    },
                };

                output.push(jtag_data);
            }
            "IR TDI" | "IR TDO" | "DR TDI" | "DR TDO" => {
                // Received packet is a Python List with two values:
                // ['NEW STATE', ["0011", x]]
                //
                // Where the bits are sent as the first element of a list as
                // a string of chars "1" and "0".
                let obj: Bound<'_, PyAny> = list.get_item(1).unwrap();
                let list: &Bound<'_, PyList> =
                    obj.cast::<PyList>().expect("It should only receive a list");

                // Bits are a string of "1100110"
                let bits: String = list.get_item(0).unwrap().extract().unwrap();
                let len_in_bits = bits.len();

                let last_state: &mut JTAGState = &mut output.last_mut().unwrap().state;

                match last_state {
                    JTAGState::UpdateIR(tdi_tdo) | JTAGState::UpdateDR(tdi_tdo) => {
                        if kind == "IR TDI" || kind == "DR TDI" {
                            tdi_tdo.tdi.extend(bit_string_to_vec(bits));
                        } else {
                            tdi_tdo.tdo.extend(bit_string_to_vec(bits));
                        }

                        tdi_tdo.len_in_bits = len_in_bits;
                        tdi_tdo.sample = DataSample {
                            value: 0,
                            start_sample: data.start_sample,
                            end_sample: data.end_sample,
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });
}

/// Converts a string like "000100010101" into a Vec<u8>.
///
/// If the data size is not a multiple of 8, data will be prepended with zeros.
fn bit_string_to_vec(mut bits: String) -> Vec<u8> {
    let len_in_bits = bits.len();
    let mut out: Vec<u8> = Vec::new();

    // Extend the string with zeros
    if len_in_bits % 8 != 0 {
        for _i in 0..(8 - (len_in_bits % 8)) {
            bits.insert(0, '0');
        }
    }

    let bits: Vec<u8> = unsafe { bits.as_bytes_mut() }
        .iter_mut()
        .map(|x| *x - ('0' as u8))
        .collect();

    // Separate into slices of 8 chars each
    for byte in bits.chunks(8) {
        let num_byte: u8 = byte[0] << 7
            | byte[1] << 6
            | byte[2] << 5
            | byte[3] << 4
            | byte[4] << 3
            | byte[5] << 2
            | byte[6] << 1
            | byte[7];

        out.push(num_byte);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_string_to_vec() {
        let input = "0000111110101010";
        let expected_output: Vec<u8> = vec![0b00001111, 0b10101010];

        let out = bit_string_to_vec(String::from(input));
        assert!(expected_output == out);
    }

    #[test]
    fn test_bit_string_not_multiple_of_8() {
        let input = "111";
        let expected_output: Vec<u8> = vec![0b00000111];

        let out: Vec<u8> = bit_string_to_vec(String::from(input));

        dbg!(&expected_output);
        dbg!(&out);
        assert!(expected_output == out);
    }
}
