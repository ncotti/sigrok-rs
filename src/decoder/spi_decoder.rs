// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! SPI decoder

use std::{ffi::CString, ptr::addr_of_mut, str::FromStr};

use crate::{Decoder, SrError, sr_try};

use libsigrok_sys::sigrokdecode as srd;
use glib::{self, ffi::GHashTable};


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
            Self::CLK => {"clk"},
            Self::MISO => {"miso"},
            Self::MOSI => {"mosi"},
            Self::CS => {"cs"},
        }
    }
}

pub struct SPIDecoder {
    pub decoder: Decoder,
}

impl SPIDecoder {
    pub fn new(options: Option<Vec<SPIOptions>>, channels: Option<Vec<SPIChannels>>) -> Result<Self, SrError> {
        let decoder = Decoder::new("spi")?;
        let options = options.unwrap_or_default();

        for option in options {
            match option {
                SPIOptions::Samplerate(samplerate) => {
                    sr_try!(srd::srd_session_metadata_set(decoder.p_session, srd::srd_configkey_SRD_CONF_SAMPLERATE as i32, glib::ffi::g_variant_new_uint64(samplerate).cast()));
                },

                SPIOptions::CSPolarity(pol) => {
                    // let table: *mut GHashTable = unsafe{glib::ffi::g_hash_table_new(Some(glib::ffi::g_str_hash), Some(glib::ffi::g_str_equal))};
                    // let cstring_key = Box::new(std::ffi::CString::new("cs_polarity").unwrap());
                    // let cstring_value = if pol == 0 {
                    //     Box::new(std::ffi::CString::new("active-low").unwrap())
                    // } else {
                    //     Box::new(std::ffi::CString::new("active-high").unwrap())
                    // };

                    // unsafe{glib::ffi::g_hash_table_insert(table, Box::into_raw(cstring_key).cast() ,Box::into_raw(cstring_value).cast())};

                    // sr_try!(srd::srd_inst_option_set(decoder.p_decoder_instance, table.cast()));

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

                    let variant = unsafe {
                        glib::ffi::g_variant_new_string(value.as_ptr())
                    };

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
                },

                SPIOptions::ClockPolarity(pol) => {
                    let table: *mut GHashTable = unsafe {
                        glib::ffi::g_hash_table_new(
                            Some(glib::ffi::g_str_hash),
                            Some(glib::ffi::g_str_equal),
                        )
                    };

                    let key = std::ffi::CString::new("cpol").unwrap();

                    let value = unsafe {
                        glib::ffi::g_variant_new_int64(if pol == 0 { 0 } else { 1 })
                    };

                    unsafe {
                        glib::ffi::g_hash_table_insert(
                            table,
                            key.into_raw().cast(),
                            value.cast(),
                        );
                    }

                    sr_try!(srd::srd_inst_option_set(
                        decoder.p_decoder_instance,
                        table.cast(),
                    ));
                },

                SPIOptions::ClockPhase(cpha) => {
                    let table: *mut GHashTable = unsafe {
                        glib::ffi::g_hash_table_new(
                            Some(glib::ffi::g_str_hash),
                            Some(glib::ffi::g_str_equal),
                        )
                    };

                    let key = std::ffi::CString::new("cpha").unwrap();

                    let value = unsafe {
                        glib::ffi::g_variant_new_int64(if cpha == 0 { 0 } else { 1 })
                    };

                    unsafe {
                        glib::ffi::g_hash_table_insert(
                            table,
                            key.into_raw().cast(),
                            value.cast(),
                        );
                    }

                    sr_try!(srd::srd_inst_option_set(
                        decoder.p_decoder_instance,
                        table.cast(),
                    ));
                },

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

                    let variant = unsafe {
                        glib::ffi::g_variant_new_string(value.as_ptr())
                    };

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
                },
            }
        }

        let channels = channels.unwrap_or_default();

        let channels_table: *mut glib::ffi::GHashTable = unsafe {
            glib::ffi::g_hash_table_new(
                Some(glib::ffi::g_str_hash),
                Some(glib::ffi::g_str_equal),
            )
        };

        let mut counter: i32 = 0;

        for channel in channels {
            let key = std::ffi::CString::new(channel.as_str()).unwrap();

            let value = unsafe{glib::ffi::g_variant_new_int32(counter)};

            unsafe {
                glib::ffi::g_hash_table_insert(
                    channels_table,
                    key.into_raw().cast(),
                    value.cast(),
                );
            }

            counter += 1;
        }

        if counter > 0 {
            sr_try!(srd::srd_inst_channel_set_all(
                decoder.p_decoder_instance,
                channels_table.cast(),
            ));
        }

        let spi_decoder = SPIDecoder {
            decoder: decoder
        };

        Ok(spi_decoder)
    }
}