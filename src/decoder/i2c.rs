// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! # I2C decoder

use std::{ffi::c_void, ptr::null_mut};

use crate::{SrError, sr_try};

use glib::{self};
use libsigrok_sys::sigrokdecode::{self as srd, srd_proto_data};

use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::decoder::{DataSample, Decoder};

/// Configuration options for I2C decoder.
pub enum I2COptions {
    /// Todo check.
    AddressShifted(u8),
}

/// I2C Channels.
pub enum I2CChannels {
    /// Serial clock.
    SCL,
    /// Serial Data line.
    SDA,
}

impl I2CChannels {
    fn as_str(&self) -> &str {
        match self {
            Self::SCL => "scl",
            Self::SDA => "sda",
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
#[derive(Debug, Default)]
pub struct I2CData {
    /// Start
    pub start: Vec<DataSample>,
    /// Address
    pub address: Vec<DataSample>,
    /// RW address
    pub rw_address: Vec<bool>,
    /// Address ACK
    pub address_ack: Vec<DataSample>,
    /// Data
    pub data: Vec<DataSample>,
    /// Data ACK
    pub data_ack: Vec<DataSample>,
    /// RW data
    pub rw_data: Vec<bool>,
    /// Stop
    pub stop: Option<DataSample>,
}

/// I2C Decoder struct.
pub struct I2CDecoder {
    /// Internal decoder structure.
    decoder: Decoder,
    /// I2C transfers.
    data: Box<Vec<I2CData>>,
}

impl I2CDecoder {
    /// Creates a new SPIDecoder.
    ///
    /// * `options`: Configuration options for the SPI decoder.
    /// * `channels`: Which channels are read, and in which position. The
    /// order in which the channels are defined in here determine the channel
    /// number for that signal, starting from zero.
    pub fn new(
        options: Option<Vec<I2COptions>>,
        channels: Option<Vec<I2CChannels>>,
    ) -> Result<Self, SrError> {
        let decoder = Decoder::new("i2c")?;

        let options = options.unwrap_or_else(|| vec![I2COptions::AddressShifted(0)]);

        for option in options {
            match option {
                I2COptions::AddressShifted(shifted) => {
                    let table: *mut glib::ffi::GHashTable = unsafe {
                        glib::ffi::g_hash_table_new(
                            Some(glib::ffi::g_str_hash),
                            Some(glib::ffi::g_str_equal),
                        )
                    };

                    let key = std::ffi::CString::new("shifted").unwrap();

                    let value = if shifted == 0 {
                        std::ffi::CString::new("unshifted").unwrap()
                    } else {
                        std::ffi::CString::new("shifted").unwrap()
                    };

                    let variant = unsafe { glib::ffi::g_variant_new_string(value.as_ptr()) };

                    unsafe {
                        glib::ffi::g_hash_table_insert(
                            table,
                            key.into_raw().cast(),
                            variant.cast(),
                        );
                    }

                    sr_try!(srd::srd_inst_option_set(
                        decoder.p_decoder_instance,
                        table.cast(),
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

        let data: Box<Vec<I2CData>> = Box::new(Vec::new());

        let mut i2c_decoder = I2CDecoder {
            decoder: decoder,
            data: data,
        };

        let cb_data = ((&mut *i2c_decoder.data) as *mut Vec<I2CData>) as *mut c_void;

        sr_try!(srd::srd_pd_output_callback_add(
            i2c_decoder.decoder.p_session,
            srd::srd_output_type_SRD_OUTPUT_PYTHON as i32,
            Some(spi_decoder_callback),
            cb_data
        ));

        Ok(i2c_decoder)
    }

    /// Writes to the SPI Decoder and returns the decoded output.
    pub fn write(&mut self, data: Vec<u8>) -> Result<&Vec<I2CData>, SrError> {
        self.decoder.write(data)?;
        Ok(&self.data)
    }
}

/// SPI decoder callback.
///
/// It reads the values from the decoder and stores them in the SPIData
/// variable.
extern "C" fn spi_decoder_callback(p_data: *mut srd_proto_data, cb_data: *mut c_void) {
    if p_data == null_mut() {
        return;
    }
    let data: srd_proto_data = unsafe { *p_data };

    if data.data == null_mut() {
        return;
    }

    let output = unsafe { &mut *cb_data.cast::<Vec<I2CData>>() };

    Python::attach(|py| {
        // Converts the void* to a Python object
        let obj = unsafe { Bound::from_borrowed_ptr(py, data.data as *mut pyo3::ffi::PyObject) };

        // Python object to list.
        let list: &Bound<'_, PyList> = obj.cast::<PyList>().expect("It should only receive a list");

        // Get the first element of the Python list.
        // It should be a string with the packet type.
        let kind: String = list.get_item(0).unwrap().extract().unwrap();

        // Types of SPI packets
        match kind.as_ref() {
            "START" => {
                // Received packet is a Python List with two values:
                // ['START', None]
                //
                // The start condition will always create a new I2CData value.

                output.push(I2CData::default());

                let data_sample = DataSample {
                    start_sample: data.start_sample,
                    end_sample: data.end_sample,
                    value: 1,
                };
                output.last_mut().unwrap().start.push(data_sample);
            }
            "START REPEAT" => {
                // Received packet is a Python List with two values:
                // ['START REPEAT', None]

                let data_sample = DataSample {
                    start_sample: data.start_sample,
                    end_sample: data.end_sample,
                    value: 1,
                };
                output.last_mut().unwrap().start.push(data_sample);
            }
            "STOP" => {
                let data_sample = DataSample {
                    start_sample: data.start_sample,
                    end_sample: data.end_sample,
                    value: 1,
                };
                output.last_mut().unwrap().stop = Some(data_sample);
            }
            "ACK" => {
                let data_sample = DataSample {
                    start_sample: data.start_sample,
                    end_sample: data.end_sample,
                    value: 1,
                };

                // Check how many addresses have been received.
                // If the number of addresses is higher than the number of
                // address ACK, then it is an address ACK.
                // of addresses and
                if output.last().unwrap().address.len() > output.last().unwrap().address_ack.len() {
                    output.last_mut().unwrap().address_ack.push(data_sample);
                } else {
                    output.last_mut().unwrap().data_ack.push(data_sample);
                }
            }
            "NACK" => {
                let data_sample = DataSample {
                    start_sample: data.start_sample,
                    end_sample: data.end_sample,
                    value: 0,
                };

                // Check how many addresses have been received.
                // If the number of addresses is higher than the number of
                // address ACK, then it is an address ACK.
                // of addresses and
                if output.last().unwrap().address.len() > output.last().unwrap().address_ack.len() {
                    output.last_mut().unwrap().address_ack.push(data_sample);
                } else {
                    output.last_mut().unwrap().data_ack.push(data_sample);
                }
            }
            "ADDRESS READ" => {
                let value: u8 = list.get_item(1).unwrap().extract().unwrap();
                let data_sample = DataSample {
                    start_sample: data.start_sample,
                    end_sample: data.end_sample,
                    value: value,
                };

                output.last_mut().unwrap().address.push(data_sample);
                output.last_mut().unwrap().rw_address.push(true);
            }
            "ADDRESS WRITE" => {
                let value: u8 = list.get_item(1).unwrap().extract().unwrap();
                let data_sample = DataSample {
                    start_sample: data.start_sample,
                    end_sample: data.end_sample,
                    value: value,
                };

                output.last_mut().unwrap().address.push(data_sample);
                output.last_mut().unwrap().rw_address.push(false);
            }
            "DATA READ" => {
                let value: u8 = list.get_item(1).unwrap().extract().unwrap();
                let data_sample = DataSample {
                    start_sample: data.start_sample,
                    end_sample: data.end_sample,
                    value: value,
                };

                output.last_mut().unwrap().data.push(data_sample);
                output.last_mut().unwrap().rw_data.push(true);
            }
            "DATA WRITE" => {
                let value: u8 = list.get_item(1).unwrap().extract().unwrap();
                let data_sample = DataSample {
                    start_sample: data.start_sample,
                    end_sample: data.end_sample,
                    value: value,
                };

                output.last_mut().unwrap().data.push(data_sample);
                output.last_mut().unwrap().rw_data.push(false);
            }
            "BITS" => {
                // ignore
            }
            _ => {
                eprintln!("Unexpected I2C event: {kind}");
                return;
            }
        }
    });
}
