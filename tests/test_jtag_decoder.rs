// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! Test JTAG decoder

use sigrok_rs::jtag::{JTAGDecoder, JTAGState, JTAGTdiTdo};
use sigrok_rs::{DataSample, SrError};

#[test]
fn test_jtag_decoder() -> Result<(), SrError> {
    let mut decoder = JTAGDecoder::new(None)?;

    // TMS TCK TDO TDI
    // Example from https://vlsitutorials.com/example-showing-jtag-operation/
    let data: Vec<u8> = vec![
        0b0000_0100, // Reset
        0b0000_0000,
        0b0000_0100, // Idle
        0b0000_0000,
        0b0000_1100, // Select DR (0)
        0b0000_1000,
        0b0000_1100, // Select IR
        0b0000_1000,
        0b0000_0100, // Capture IR
        0b0000_1000,
        0b0000_0100, // Shift IR
        0b0000_0000,
        0b0000_0100, // Shift IR
        0b0000_0000,
        0b0000_0101, // Shift IR
        0b0000_0000,
        0b0000_0110, // Shift IR
        0b0000_0000,
        0b0000_1101, // Exit1 IR
        0b0000_0001,
        0b0000_1101, // Update IR (8)
        0b0000_0001,
        0b0000_1101, // Select DR
        0b0000_0001,
        0b0000_0101, // Capture DR
        0b0000_0001,
        0b0000_0101, // Shift DR
        0b0000_0000,
        0b0000_0110, // Shift DR
        0b0000_0000,
        0b0000_0100, // Shift DR
        0b0000_0000,
        0b0000_1101, // Exit1 DR
        0b0000_0001,
        0b0000_1101, // Update DR (15)
        0b0000_0001,
        0b0000_0101, // Idle
        0b0000_0001,
        0b0000_0101, // Idle (17)
        0b0000_0001,
    ];
    let expected_ir = JTAGTdiTdo {
        tdi: vec![0b0000_1010],
        tdo: vec![0b0000_0100],
        sample: DataSample {
            value: 0,
            start_sample: 12,
            end_sample: 20,
        },
        len_in_bits: 4,
    };

    let expected_dr = JTAGTdiTdo {
        tdi: vec![0b0000_0100],
        tdo: vec![0b0000_0001],
        sample: DataSample {
            value: 0,
            start_sample: 28,
            end_sample: 34,
        },
        len_in_bits: 3,
    };

    let jtag_data = decoder.write(data)?;

    dbg!(jtag_data);
    assert!(jtag_data.len() == 18);
    let update_ir = &jtag_data[8].state;
    if let JTAGState::UpdateIR(data) = &update_ir {
        assert!(data.tdi == expected_ir.tdi);
        assert!(data.tdo == expected_ir.tdo);
        assert!(data.sample.value == expected_ir.sample.value);
        assert!(data.sample.start_sample == expected_ir.sample.start_sample);
        assert!(data.sample.end_sample == expected_ir.sample.end_sample);
        assert!(data.len_in_bits == expected_ir.len_in_bits);
    } else {
        assert!(false);
    }
    let update_dr = &jtag_data[15].state;
    if let JTAGState::UpdateDR(data) = &update_dr {
        assert!(data.tdi == expected_dr.tdi);
        assert!(data.tdo == expected_dr.tdo);
        assert!(data.sample.value == expected_dr.sample.value);
        assert!(data.sample.start_sample == expected_dr.sample.start_sample);
        assert!(data.sample.end_sample == expected_dr.sample.end_sample);
        assert!(data.len_in_bits == expected_dr.len_in_bits);
    } else {
        assert!(false);
    }

    Ok(())
}
