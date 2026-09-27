// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! This file changes a device configuration and does several runs.

use std::io::{self, BufRead};

use sigrok_rs::{Session, SrError};
use tempfile::NamedTempFile;

/// Changes the samplerate
#[test]
fn test_samplerate() -> Result<(), SrError> {
    let mut session = Session::try_from("demo")?;
    let file1 = NamedTempFile::new().unwrap();
    let file2 = NamedTempFile::new().unwrap();

    session.set_output("bits", file1.path())?;
    session.device.set_option("samplerate", "100000")?;
    session.run_samples(10)?;

    let file = std::fs::File::open(file1.path()).unwrap();
    let mut reader = io::BufReader::new(file);

    let mut line1: String = String::new();
    reader.read_line(&mut line1).unwrap();
    line1.clear();
    reader.read_line(&mut line1).unwrap();

    assert!(line1 == "Acquisition with 8/13 channels at 100 kHz\n");

    session.set_output("bits", file2.path())?;
    session.device.set_option("samplerate", "2000000")?;
    session.run_samples(10)?;

    let file = std::fs::File::open(file2.path()).unwrap();
    let mut reader = io::BufReader::new(file);

    let mut line1: String = String::new();
    reader.read_line(&mut line1).unwrap();
    line1.clear();
    reader.read_line(&mut line1).unwrap();

    assert!(line1 == "Acquisition with 8/13 channels at 2 MHz\n");
    Ok(())
}

/// Disables channels, and renames them.
#[test]
fn test_channel_enablement() -> Result<(), SrError> {
    let mut session = Session::try_from("demo")?;
    let file = NamedTempFile::new().unwrap();

    session.set_output("bits", file.path())?;

    // Simulate a SPI connection, with channels being SCLK, MOSI, MISO, CS
    session.device.set_channel_name("D0", "SCLK")?;
    session.device.set_channel_name("D1", "MOSI")?;
    session.device.set_channel_name("D2", "MISO")?;
    session.device.set_channel_name("D3", "CS")?;

    session.device.enable_channel("4", false)?;
    session.device.enable_channel("5", false)?;
    session.device.enable_channel("6", false)?;
    session.device.enable_channel("7", false)?;

    let data = session.run_samples(4)?;
    assert!(data.len() == 4);

    let file = std::fs::File::open(file.path()).unwrap();
    let mut reader = io::BufReader::new(file);

    let mut line: String = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line == "libsigrok 0.5.2\n");

    let mut line: String = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line == "Acquisition with 4/13 channels at 200 kHz\n");

    let mut line: String = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.starts_with("SCLK:"));

    let mut line: String = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.starts_with("MOSI:"));

    let mut line: String = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.starts_with("MISO:"));

    let mut line: String = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.starts_with("CS:"));

    // No more lines to be read.
    let mut line: String = String::new();
    assert!(reader.read_line(&mut line).unwrap() == 0);

    Ok(())
}
