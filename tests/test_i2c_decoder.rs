// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! Test SPI decoder.

use sigrok_rs::{I2CDecoder, SrError};

#[test]
fn test_i2c_basic_transaction() -> Result<(), SrError> {
    let mut decoder = I2CDecoder::new(None, None)?;

    // Data will be [SDA, SCL]
    let data: Vec<u8> = vec![
        0b0000_0011,
        0b0000_0001, // Falling edge SDA while SCL=1. START. Sample 1
        0b0000_0010,
        0b0000_0011, // Sample 3. A6. Address is 0b1011100 (7 bits).
        0b0000_0000,
        0b0000_0001, // A5
        0b0000_0010,
        0b0000_0011, // A4
        0b0000_0010,
        0b0000_0011, // A3
        0b0000_0010,
        0b0000_0011, // A2
        0b0000_0000,
        0b0000_0001, // A1
        0b0000_0000,
        0b0000_0001, // A0. Sample 15
        0b0000_0010,
        0b0000_0011, // Read address. Sample 17
        0b0000_0000,
        0b0000_0001, // Address ACK. Sample 19
        0b0000_0000,
        0b0000_0001, // D7. Sample 21. Data is 0x72
        0b0000_0010,
        0b0000_0011, // D6
        0b0000_0010,
        0b0000_0011, // D5
        0b0000_0010,
        0b0000_0011, // D4
        0b0000_0000,
        0b0000_0001, // D3
        0b0000_0000,
        0b0000_0001, // D2
        0b0000_0010,
        0b0000_0011, // D1
        0b0000_0000,
        0b0000_0001, // D0. Sample 35
        0b0000_0010,
        0b0000_0011, // Data NACK. Sample 37
        0b0000_0000,
        0b0000_0001,
        0b0000_0011, // Rising edge with SCL=1. STOP. Sample 40.
    ];

    let data = decoder.write(data)?;

    dbg!(data);
    assert!(data.len() == 1);

    assert!(data[0].start.len() == 1);
    assert!(data[0].start[0].start_sample == 1 && data[0].start[0].end_sample == 1);

    assert!(data[0].address.len() == 1);
    assert!(
        data[0].address[0] == 0b1011100
            && data[0].address[0].start_sample == 3
            && data[0].address[0].end_sample == 19
    );

    assert!(data[0].rw_address[0] == true);
    assert!(
        data[0].address_ack[0] == 1
            && data[0].address_ack[0].start_sample == 19
            && data[0].address_ack[0].end_sample == 21
    );

    assert!(data[0].data.len() == 1);
    assert!(
        data[0].data[0] == 0x72
            && data[0].data[0].start_sample == 21
            && data[0].data[0].end_sample == 37
    );
    assert!(
        data[0].data_ack[0] == 0
            && data[0].data_ack[0].start_sample == 37
            && data[0].data_ack[0].end_sample == 39
    );

    assert!(data[0].stop.is_some());
    assert!(data[0].stop.as_ref().unwrap().start_sample == 40);

    Ok(())
}
