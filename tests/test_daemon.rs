// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! This file attempts to run sessions on the background, while doing
//! other things in the foreground.

use std::time::Duration;

use sigrok_rs::{Session, SrError};

/// Busy work is done while the data capture session runs on the background.
#[test]
fn test_background_run() -> Result<(), SrError> {
    let mut session = Session::try_from("demo")?;

    session.set_timeout(Duration::from_millis(100))?;
    session.device.set_option("samplerate", "200000")?;

    let timer = std::time::Instant::now();
    session.run_daemon()?;

    // Simulate a busy work of 200ms.
    std::thread::sleep(Duration::from_millis(200));

    // The data capture should have ended in the 200ms span, so the total time
    // should be less than the addition of both.
    assert!(timer.elapsed() < Duration::from_millis(220));

    let data = session.read()?;
    assert!(data.len() == 200000 * 100 / 1000);
    Ok(())
}

/// Background run is cut short.
#[test]
fn test_run_abort() -> Result<(), SrError> {
    let mut session = Session::try_from("demo")?;

    // 10kHz and 10000 samples amount to 1 second
    session.device.set_option("samplerate", "10000")?;
    session.run_samples_daemon(100000)?;
    std::thread::sleep(Duration::from_millis(100));
    let data = session.abort()?;

    assert!(!data.is_empty());
    assert!(data.len() < 100000);

    session.run_samples_daemon(100)?;
    let data = session.read()?;
    assert!(data.len() == 100);
    Ok(())
}

#[test]
fn test_read_without_run() -> Result<(), SrError> {
    let mut session = Session::try_from("demo")?;

    // Read without running any session should fail
    let result = session.read();
    assert!(result.is_err());
    assert!(result.unwrap_err() == SrError::SrNoData);

    // Read after foreground run should fail
    let data = session.run_samples(10)?;
    assert!(data.len() == 10);

    let result = session.read();
    assert!(result.is_err());
    assert!(result.unwrap_err() == SrError::SrNoData);

    // Double read, the second one should fail
    session.run_samples_daemon(15)?;
    let data = session.read()?;
    assert!(data.len() == 15);
    let result = session.read();
    assert!(result.is_err());
    assert!(result.unwrap_err() == SrError::SrNoData);
    Ok(())
}

#[test]
fn test_background_and_foreground_at_the_same_time() -> Result<(), SrError> {
    let mut session = Session::try_from("demo")?;

    session.set_timeout(Duration::from_millis(100))?;
    session.device.set_option("samplerate", "50000")?;
    session.run_daemon()?;

    let result = session.run();
    assert!(result.is_err());
    assert!(result.unwrap_err() == SrError::SrSessionAlreadyRunning);

    let data = session.read()?;
    assert!(data.len() == 50000 * 100 / 1000);
    Ok(())
}
