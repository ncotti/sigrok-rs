// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! Test SPI decoder.

use sigrok_rs::{SPIChannels, SPIDecoder, SPIOptions, SrError};

#[test]
fn test_spi_cs() -> Result<(), SrError> {
    let options: Vec<SPIOptions> = vec![
        SPIOptions::Samplerate(100000),
        SPIOptions::ClockPolarity(0),
        SPIOptions::BitOrder(0),
        SPIOptions::CSPolarity(0),
        SPIOptions::ClockPhase(0),
    ];

    let channels: Vec<SPIChannels> = vec![SPIChannels::CLK, SPIChannels::CS, SPIChannels::MISO];
    let mut decoder = SPIDecoder::new(Some(options), Some(channels))?;

    // A total of 7 transitions for the pin "CS" are done.
    let data: Vec<u8> = vec![
        0b0000_0011, // sample 0
        0b0000_0010,
        0b0000_0011,
        0b0000_0000, // sample 3
        0b0000_0011, // sample 4
        0b0000_0010,
        0b0000_0001, // sample 6
        0b0000_0000,
        0b0000_0001,
        0b0000_0010, // sample 9
        0b0000_0011,
        0b0000_0000, // sample 11
        0b0000_0001,
        0b0000_0010, // sample 13
        0b0000_0011,
    ];

    let data = decoder.write(data)?;

    assert!(data.miso.is_empty());
    assert!(data.mosi.is_empty());
    assert!(data.cs.len() == 7);
    assert!(data.cs[0].start_sample == 0 && data.cs[0].end_sample == 0 && data.cs[0] == 1);
    assert!(data.cs[1].start_sample == 3 && data.cs[1].end_sample == 3 && data.cs[1] == 0);
    assert!(data.cs[2].start_sample == 4 && data.cs[2].end_sample == 4 && data.cs[2] == 1);
    assert!(data.cs[3].start_sample == 6 && data.cs[3].end_sample == 6 && data.cs[3] == 0);
    assert!(data.cs[4].start_sample == 9 && data.cs[4].end_sample == 9 && data.cs[4] == 1);
    assert!(data.cs[5].start_sample == 11 && data.cs[5].end_sample == 11 && data.cs[5] == 0);
    assert!(data.cs[6].start_sample == 13 && data.cs[6].end_sample == 13 && data.cs[6] == 1);

    Ok(())
}

#[test]
fn test_miso_mosi_data() -> Result<(), SrError> {
    let options: Vec<SPIOptions> = vec![
        SPIOptions::Samplerate(100000),
        SPIOptions::ClockPolarity(0),
        SPIOptions::BitOrder(0),
        SPIOptions::CSPolarity(0),
        SPIOptions::ClockPhase(0),
    ];

    let channels: Vec<SPIChannels> = vec![
        SPIChannels::CLK,
        SPIChannels::MISO,
        SPIChannels::MOSI,
        SPIChannels::CS,
    ];

    // Data follows the order of the variable "channels" above
    let data: Vec<u8> = vec![
        0b0000_1111,
        0b0000_1110,
        // 8-bit transaction start
        0b0000_0101, // sample 2
        0b0000_0100,
        0b0000_0101,
        0b0000_0100,
        0b0000_0001,
        0b0000_0000,
        0b0000_0101,
        0b0000_0100,
        0b0000_0111,
        0b0000_0110,
        0b0000_0111,
        0b0000_0110,
        0b0000_0111,
        0b0000_0110,
        0b0000_0011,
        0b0000_0010, // sample 17
        // 8-bit transaction end
        0b0000_1111,
        0b0000_1110,
        0b0000_1111,
        0b0000_1110,
    ];

    let mut decoder = SPIDecoder::new(Some(options), Some(channels))?;

    let data = decoder.write(data)?;

    assert!(data.cs.len() == 3);
    assert!(data.cs[0].start_sample == 0 && data.cs[0] == 1);
    assert!(data.cs[1].start_sample == 2 && data.cs[1] == 0);
    assert!(data.cs[2].start_sample == 18 && data.cs[2] == 1);

    assert!(data.miso.len() == 1);
    assert!(
        data.miso[0] == 0x0F && data.miso[0].start_sample == 2 && data.miso[0].end_sample == 18
    );

    assert!(data.mosi.len() == 1);
    assert!(
        data.mosi[0] == 0xDE && data.mosi[0].start_sample == 2 && data.mosi[0].end_sample == 18
    );

    Ok(())
}
