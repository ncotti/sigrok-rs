//! This file test the different trigger conditions and how the output is
//! generated.
//!
//! This is what the output file looks like after recording with triggers.
//! There is an extra line for the trigger, that points to the line before
//! the trigger condition was met (in this case, was all 8 channels being high).
//! The number is the sample number when the trigger occurred, counting from
//! zero.
//!
//! You should be able to read "SIGROK !" from the zeros.
//! ```txt
//! D0:10001111 00001111 10001111 00001111 10001111 01110111 11111111 00111111
//! D1:01110111 10011111 01110111 01110111 01110111 01101111 11111111 00111111
//! D2:01111111 10011111 01111111 01110111 01110111 01011111 11111111 00111111
//! D3:10001111 10011111 01100111 00001111 01110111 00111111 11111111 00111111
//! D4:11110111 10011111 01110111 01101111 01110111 01011111 11111111 00111111
//! D5:01110111 10011111 01110111 01110111 01110111 01101111 11111111 11111111
//! D6:10001111 00001111 10001111 01110111 10001111 01110111 11111111 00111111
//! D7:11111111 11111111 11111111 11111111 11111111 11111111 11111111 11111111
//! T:     ^ 5
//! ```txt
//!
//! The output data returned by the function starts from the first triggered
//! sample.

use std::{
    io::{self, BufRead},
    time::Duration,
};

use sigrok_rs::{Session, SrError, trigger::TriggerEvent};

use tempfile::NamedTempFile;

/// Tests the trigger condition.
#[test]
fn test_trigger_single_match() -> Result<(), SrError> {
    let mut session = Session::try_from("demo")?;
    let file = NamedTempFile::new().unwrap();

    session.set_output("bits", file.path())?;

    let events = vec![
        ("D0", TriggerEvent::One),
        ("D1", TriggerEvent::One),
        ("D2", TriggerEvent::One),
        ("D3", TriggerEvent::One),
        ("D4", TriggerEvent::One),
        ("D5", TriggerEvent::One),
        ("D6", TriggerEvent::One),
        ("D7", TriggerEvent::One),
    ];
    session.set_trigger("my_trigger", events)?;
    let data = session.run_samples(100)?;

    assert!(data.len() == 100);
    assert!(data[0] == 0xFF);

    let file = std::fs::File::open(file.path()).unwrap();
    let reader = io::BufReader::new(file);

    let mut line_count = 0;
    for line in reader.lines() {
        // 10 lines == 8 channels + 2 headers
        if line_count == 10 {
            let line = line.unwrap();
            assert!(line.starts_with("T:"));
            assert!(line.contains("^ 5"));
            break;
        }
        line_count += 1;
    }
    assert!(line_count == 10);

    Ok(())
}

#[test]
fn test_trigger_single_sample() -> Result<(), SrError> {
    let mut session = Session::try_from("demo")?;
    let file = NamedTempFile::new().unwrap();

    session.set_output("bits", file.path())?;

    // This condition appears on the 56th sample, but the default
    // timeout is 1 second, so it should reach
    let events = vec![
        ("D0", TriggerEvent::Zero),
        ("D1", TriggerEvent::Zero),
        ("D2", TriggerEvent::Zero),
        ("D3", TriggerEvent::Zero),
        ("D4", TriggerEvent::Zero),
        ("D5", TriggerEvent::One),
        ("D6", TriggerEvent::Zero),
        ("D7", TriggerEvent::One),
    ];
    session.set_trigger("my_trigger", events)?;
    let data = session.run_samples(1)?;

    assert!(data.len() == 1);
    assert!(data[0] == 0b10100000);

    let file = std::fs::File::open(file.path()).unwrap();
    let reader = io::BufReader::new(file);

    let mut line_count = 0;
    for line in reader.lines() {
        // 10 lines == 8 channels + 2 headers
        if line_count == 10 {
            let line = line.unwrap();
            assert!(line.starts_with("T:"));
            assert!(line.contains("^ 56"));
            break;
        }
        line_count += 1;
    }
    assert!(line_count == 10);

    Ok(())
}

/// A trigger with multiple stages, the second added happens first.
#[test]
fn test_trigger_multiple_stages_second_is_early() -> Result<(), SrError> {
    let mut session = Session::try_from("demo")?;
    let file = NamedTempFile::new().unwrap();

    session.set_output("bits", file.path())?;

    // This first condition is met on sample 56
    let events = vec![
        ("D0", TriggerEvent::Zero),
        ("D1", TriggerEvent::Zero),
        ("D2", TriggerEvent::Zero),
        ("D3", TriggerEvent::Zero),
        ("D4", TriggerEvent::Zero),
        ("D5", TriggerEvent::One),
        ("D6", TriggerEvent::Zero),
        ("D7", TriggerEvent::One),
    ];
    session.set_trigger("my_trigger", events)?;

    // This second condition is met before sample 56, but the first time it
    // appears after the first trigger is sample 58
    let events = vec![
        ("D0", TriggerEvent::One),
        ("D1", TriggerEvent::One),
        ("D2", TriggerEvent::One),
        ("D3", TriggerEvent::One),
        ("D4", TriggerEvent::One),
        ("D5", TriggerEvent::One),
        ("D6", TriggerEvent::One),
        ("D7", TriggerEvent::One),
    ];
    session.add_trigger_stage(events)?;
    let data = session.run_samples(100)?;

    assert!(data.len() == 100);
    assert!(data[0] == 0xFF);

    let file = std::fs::File::open(file.path()).unwrap();
    let reader = io::BufReader::new(file);

    let mut line_count = 0;
    for line in reader.lines() {
        // 10 lines == 8 channels + 2 headers
        if line_count == 10 {
            let line = line.unwrap();
            assert!(line.starts_with("T:"));
            assert!(line.contains("^ 58"));
            break;
        }
        line_count += 1;
    }
    assert!(line_count == 10);

    Ok(())
}

/// The trigger condition is never met, so it should exit for timeout.
#[test]
fn test_trigger_miss() -> Result<(), SrError> {
    let mut session = Session::try_from("demo")?;
    let file = NamedTempFile::new().unwrap();

    session.set_output("bits", file.path())?;

    let events = vec![
        ("D0", TriggerEvent::Zero),
        ("D1", TriggerEvent::Zero),
        ("D2", TriggerEvent::Zero),
        ("D3", TriggerEvent::Zero),
        ("D4", TriggerEvent::Zero),
        ("D5", TriggerEvent::Zero),
        ("D6", TriggerEvent::Zero),
        ("D7", TriggerEvent::Zero),
    ];
    session.set_trigger("my_trigger", events)?;
    session.set_timeout(Duration::from_millis(100));
    let data = session.run_samples(10);
    assert!(data.is_err());
    assert!(data.unwrap_err() == SrError::SrErrTimeout);

    Ok(())
}
