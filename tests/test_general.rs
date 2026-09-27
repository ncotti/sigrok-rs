// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! This file exercises general functionalities of the library, like scanning
//! for devices and running a basic sessions.

use sigrok_rs::{Session, SrError, device::Device};
use std::{
    io::{self},
    time::Duration,
};

use tempfile::NamedTempFile;

use std::io::BufRead;

/// The user should be able to access all information regarding connected
/// devices and their options.
#[test]
fn test_device_scan() -> Result<(), SrError> {
    let devices: Vec<Device> = Session::scan()?;
    let demo_device: Device = devices
        .into_iter()
        .find(|dev| dev.get_driver_name() == "demo")
        .unwrap();

    println!("{}", demo_device);
    assert!(demo_device.get_driver_name() == "demo");
    assert!(demo_device.get_model() == "Demo device");
    assert!(demo_device.get_serial_number() == "");
    assert!(demo_device.get_connection_id() == "");
    assert!(demo_device.get_version() == "");
    assert!(demo_device.get_vendor() == "");
    Ok(())
}

/// Runs a session for a given amount of time.
///
/// This just tests that a file is being written, and the session starts and
/// stops successfully, not its contents.
#[test]
fn test_run_timeout() -> Result<(), SrError> {
    let mut session: Session = Session::try_from("demo")?;
    let output_file = NamedTempFile::new().unwrap();

    session.set_output("ascii", output_file.path())?;
    session.set_timeout(Duration::from_millis(100));
    session.run()?;

    let file = std::fs::File::open(output_file.path()).unwrap();
    let mut reader = io::BufReader::new(file);

    let mut line0: String = String::new();
    reader.read_line(&mut line0).unwrap();
    let mut line1: String = String::new();
    reader.read_line(&mut line1).unwrap();
    let mut line2: String = String::new();
    reader.read_line(&mut line2).unwrap();
    let mut line3: String = String::new();
    reader.read_line(&mut line3).unwrap();
    let mut line4: String = String::new();
    reader.read_line(&mut line4).unwrap();
    let mut line5: String = String::new();
    reader.read_line(&mut line5).unwrap();
    let mut line6: String = String::new();
    reader.read_line(&mut line6).unwrap();
    let mut line7: String = String::new();
    reader.read_line(&mut line7).unwrap();
    let mut line8: String = String::new();
    reader.read_line(&mut line8).unwrap();
    let mut line9: String = String::new();
    reader.read_line(&mut line9).unwrap();
    let mut line10: String = String::new();
    reader.read_line(&mut line10).unwrap();

    assert!(line0 == "libsigrok 0.5.2\n");
    assert!(line1 == "Acquisition with 8/13 channels at 200 kHz\n");
    assert!(line2.starts_with("D0:"));
    assert!(line3.starts_with("D1:"));
    assert!(line4.starts_with("D2:"));
    assert!(line5.starts_with("D3:"));
    assert!(line6.starts_with("D4:"));
    assert!(line7.starts_with("D5:"));
    assert!(line8.starts_with("D6:"));
    assert!(line9.starts_with("D7:"));
    assert!(line10.starts_with("D0:"));

    Ok(())
}

/// This test measures the different timeouts and makes sure that execution
/// takes around that amount of time.
#[test]
fn test_run_timeout_measured() -> Result<(), SrError> {
    let mut session: Session = Session::try_from("demo")?;

    // 200 kHz, default value
    let samplerate: usize = 200000;

    // Default timeout should be 1 second
    let data = session.run()?;
    assert!(data.len() == samplerate * 1000 / 1000);

    session.set_timeout(Duration::from_millis(100));
    let data = session.run()?;
    assert!(data.len() == samplerate * 100 / 1000);

    session.set_timeout(Duration::from_millis(400));
    let data = session.run()?;
    assert!(data.len() == samplerate * 400 / 1000);

    Ok(())
}

/// Runs a session for a given amount of samples.
///
/// This just tests that a file is being written, and the session starts and
/// stops successfully, not its contents.
#[test]
fn test_run_samples() -> Result<(), SrError> {
    let mut session: Session = Session::try_from("demo")?;
    let output_file = NamedTempFile::new().unwrap();
    let samples: u64 = 40;

    session.set_output("ascii", output_file.path())?;
    let data = session.run_samples(samples)?;
    assert!(data.len() == samples as usize);

    let file = std::fs::File::open(output_file.path()).unwrap();
    let mut reader = io::BufReader::new(file);

    let mut line: String = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line == "libsigrok 0.5.2\n");

    let mut line: String = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line == "Acquisition with 8/13 channels at 200 kHz\n");

    // Line should be D0:<samples>\n
    let mut line: String = String::new();
    assert!(reader.read_line(&mut line).unwrap() == (samples + 4) as usize);

    Ok(())
}
