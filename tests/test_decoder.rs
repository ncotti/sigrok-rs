// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! Test decoder module

use sigrok_rs::Decoder;

#[test]
fn test_decoder_scan() {
    let decoder_ids = Decoder::scan().unwrap();
    assert!(decoder_ids.iter().find(|id| *id == "jtag").is_some());
    assert!(decoder_ids.iter().find(|id| *id == "spi").is_some());
    assert!(decoder_ids.iter().find(|id| *id == "i2c").is_some());
}
