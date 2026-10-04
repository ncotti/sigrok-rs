// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! SPI decoder

use std::{ffi::c_void, ptr::null_mut};

use crate::{Decoder, SrError, sr_try};

use glib::{self, ffi::GHashTable};
use libsigrok_sys::sigrokdecode::{self as srd, srd_proto_data};

use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::decoder::DataSample;

/// Configuration options for SPI decoder.
pub enum SPIOptions {
    /// Sample rate.
    Samplerate(u64),

    /// Chip select polarity. Default is `0`.
    ///
    /// * `0`: Chip is selected when the signal is electrical low.
    /// * `1`: Chip is selected when the signal is electrical high.
    CSPolarity(u8),

    /// Clock polarity. Default is `0`.
    ClockPolarity(u8),

    /// Clock phase. Default is `0`.
    ClockPhase(u8),

    /// MSB (0) or LSB(1) bit order. Default is MSB.
    BitOrder(u8),
}

/// SPI Channels.
pub enum SPIChannels {
    /// Clock (mandatory).
    CLK,
    /// Master In, Slave Out (optional).
    MISO,
    /// Master Out, Slave In (optional).
    MOSI,
    /// Chip Select (optional).
    CS,
}

impl SPIChannels {
    fn as_str(&self) -> &str {
        match self {
            Self::CLK => "clk",
            Self::MISO => "miso",
            Self::MOSI => "mosi",
            Self::CS => "cs",
        }
    }
}

/// Contains the data samples from the three channels for any SPI transaction.
#[derive(Debug)]
pub struct SPIData {
    /// Chip Select (CS) pin data readings. This is a single bit channel.
    pub cs: Vec<DataSample>,
    /// MOSI channel data samples. Only "valid" samples will be stored, i.e.,
    /// 8-bit transactions when the CS pin was active.
    pub mosi: Vec<DataSample>,
    /// MISO channel data samples. Only "valid" samples will be stored, i.e.,
    /// 8-bit transactions when the CS pin was active.
    pub miso: Vec<DataSample>,
}

/// SPI Decoder struct.
pub struct SPIDecoder {
    /// Internal decoder structure.
    decoder: Decoder,
    /// Data read from the SPI channels (MOSI, MISO, CS).
    data: Box<SPIData>,
}

impl SPIDecoder {
    /// Creates a new SPIDecoder.
    ///
    /// * `options`: Configuration options for the SPI decoder.
    /// * `channels`: Which channels are read, and in which position. The
    /// order in which the channels are defined in here determine the channel
    /// number for that signal, starting from zero.
    pub fn new(
        options: Option<Vec<SPIOptions>>,
        channels: Option<Vec<SPIChannels>>,
    ) -> Result<Self, SrError> {
        let decoder = Decoder::new("spi")?;

        let options = options.unwrap_or_default();

        for option in options {
            match option {
                SPIOptions::Samplerate(samplerate) => {
                    sr_try!(srd::srd_session_metadata_set(
                        decoder.p_session,
                        srd::srd_configkey_SRD_CONF_SAMPLERATE as i32,
                        glib::ffi::g_variant_new_uint64(samplerate).cast()
                    ));
                }

                SPIOptions::CSPolarity(pol) => {
                    let table: *mut glib::ffi::GHashTable = unsafe {
                        glib::ffi::g_hash_table_new(
                            Some(glib::ffi::g_str_hash),
                            Some(glib::ffi::g_str_equal),
                        )
                    };

                    let key = std::ffi::CString::new("cs_polarity").unwrap();

                    let value = if pol == 0 {
                        std::ffi::CString::new("active-low").unwrap()
                    } else {
                        std::ffi::CString::new("active-high").unwrap()
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

                SPIOptions::ClockPolarity(pol) => {
                    let table: *mut GHashTable = unsafe {
                        glib::ffi::g_hash_table_new(
                            Some(glib::ffi::g_str_hash),
                            Some(glib::ffi::g_str_equal),
                        )
                    };

                    let key = std::ffi::CString::new("cpol").unwrap();

                    let value =
                        unsafe { glib::ffi::g_variant_new_int64(if pol == 0 { 0 } else { 1 }) };

                    unsafe {
                        glib::ffi::g_hash_table_insert(table, key.into_raw().cast(), value.cast());
                    }

                    sr_try!(srd::srd_inst_option_set(
                        decoder.p_decoder_instance,
                        table.cast(),
                    ));
                }

                SPIOptions::ClockPhase(cpha) => {
                    let table: *mut GHashTable = unsafe {
                        glib::ffi::g_hash_table_new(
                            Some(glib::ffi::g_str_hash),
                            Some(glib::ffi::g_str_equal),
                        )
                    };

                    let key = std::ffi::CString::new("cpha").unwrap();

                    let value =
                        unsafe { glib::ffi::g_variant_new_int64(if cpha == 0 { 0 } else { 1 }) };

                    unsafe {
                        glib::ffi::g_hash_table_insert(table, key.into_raw().cast(), value.cast());
                    }

                    sr_try!(srd::srd_inst_option_set(
                        decoder.p_decoder_instance,
                        table.cast(),
                    ));
                }

                SPIOptions::BitOrder(order) => {
                    let table: *mut glib::ffi::GHashTable = unsafe {
                        glib::ffi::g_hash_table_new(
                            Some(glib::ffi::g_str_hash),
                            Some(glib::ffi::g_str_equal),
                        )
                    };

                    let key = std::ffi::CString::new("bitorder").unwrap();

                    let value = if order == 0 {
                        std::ffi::CString::new("msb-first").unwrap()
                    } else {
                        std::ffi::CString::new("lsb-first").unwrap()
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

        let data: Box<SPIData> = Box::new(SPIData {
            cs: Vec::new(),
            mosi: Vec::new(),
            miso: Vec::new(),
        });

        let mut spi_decoder = SPIDecoder {
            decoder: decoder,
            data: data,
        };

        let cb_data = ((&mut *spi_decoder.data) as *mut SPIData) as *mut c_void;

        sr_try!(srd::srd_pd_output_callback_add(
            spi_decoder.decoder.p_session,
            srd::srd_output_type_SRD_OUTPUT_PYTHON as i32,
            Some(spi_decoder_callback),
            cb_data
        ));

        Ok(spi_decoder)
    }

    /// Writes to the SPI Decoder and returns the decoded output.
    pub fn write(&mut self, data: Vec<u8>) -> Result<&SPIData, SrError> {
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

    let output = unsafe { &mut *cb_data.cast::<SPIData>() };

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
            "DATA" => {
                // Received packet is a Python List with three values:
                // ['DATA', mosi: Option<u8>, miso: Option<u8>]
                // "mosi" and "miso" might be None if their respective channels
                // were not configured.
                let mosi: Option<u8> = list.get_item(1).unwrap().extract().unwrap_or_default();
                let miso: Option<u8> = list.get_item(2).unwrap().extract().unwrap_or_default();

                if mosi.is_some() {
                    let data_sample = DataSample {
                        start_sample: data.start_sample,
                        end_sample: data.end_sample,
                        value: mosi.unwrap(),
                    };
                    output.mosi.push(data_sample);
                }

                if miso.is_some() {
                    let data_sample = DataSample {
                        start_sample: data.start_sample,
                        end_sample: data.end_sample,
                        value: miso.unwrap(),
                    };
                    output.miso.push(data_sample);
                }
            }
            "BITS" => {
                // Ignored
            }
            "CS-CHANGE" => {
                // Received packet is a Python List with three values:
                // ['CS-CHANGE', cs_old: Option<u8>, cs_new: Option<u8>]
                // `cs_old` is the previous value of the chip select pin, or
                // None if it is the first packet ever.
                // `cs_new` is the current value of the chip select pin, or
                // None if the channel was not configured.
                let new_cs: Option<u8> = list.get_item(2).unwrap().extract().unwrap_or_default();

                if new_cs.is_some() {
                    let data_sample = DataSample {
                        start_sample: data.start_sample,
                        end_sample: data.end_sample,
                        value: new_cs.unwrap(),
                    };
                    output.cs.push(data_sample);
                }
            }
            "TRANSFER" => {
                // Ignored
            }
            _ => {
                eprintln!("Unexpected SPI event: {kind}");
                return;
            }
        }
    });
}
