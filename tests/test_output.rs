//! This file test the different output modules

use std::time::Duration;

use sigrok_rs::{Session, SrError};

/// Scanning for output modules should be possible from the session level.
#[test]
fn test_scan_output() -> Result<(), SrError> {
    let output_modules_info = Session::scan_output()?;
    let ascii_info = output_modules_info
        .iter()
        .find(|m| m.name == "ASCII")
        .unwrap();
    assert!(ascii_info.id == "ascii");

    // Output module should implement Display trait and DEbug trait.
    println!("{}", ascii_info);
    dbg!(ascii_info);
    Ok(())
}

#[test]
fn test_ascii() -> Result<(), SrError> {
    let mut session: Session = Session::try_from("demo")?;
    let filename = "tmp/ascii.txt";

    session.set_output("ascii", filename)?;
    session.run_samples(10)?;

    assert!(std::fs::exists(filename).unwrap());

    Ok(())
}

#[test]
fn test_binary() -> Result<(), SrError> {
    let mut session: Session = Session::try_from("demo")?;
    let filename = "tmp/binary.bin";

    session.set_output("binary", filename)?;
    session.run_samples(10)?;

    assert!(std::fs::exists(filename).unwrap());

    Ok(())
}

#[test]
fn test_bits() -> Result<(), SrError> {
    let mut session: Session = Session::try_from("demo")?;
    let filename = "tmp/bits.txt";

    session.set_output("bits", filename)?;
    session.run_samples(10)?;

    assert!(std::fs::exists(filename).unwrap());

    Ok(())
}

#[test]
fn test_hex() -> Result<(), SrError> {
    let mut session: Session = Session::try_from("demo")?;
    let filename = "tmp/hex.txt";

    session.set_output("hex", filename)?;
    session.run_samples(10)?;

    assert!(std::fs::exists(filename).unwrap());

    Ok(())
}

#[test]
fn test_null() -> Result<(), SrError> {
    let mut session: Session = Session::try_from("demo")?;

    session.run_samples(10)?;

    Ok(())
}

#[test]
fn test_vcd() -> Result<(), SrError> {
    let mut session: Session = Session::try_from("demo")?;
    let filename = "tmp/vcd.vcd";

    session.set_output("vcd", filename)?;
    session.run_samples(10)?;

    assert!(std::fs::exists(filename).unwrap());

    Ok(())
}

#[test]
fn test_csv() -> Result<(), SrError> {
    let mut session: Session = Session::try_from("demo")?;
    let filename = "tmp/csv.csv";

    session.set_output("csv", filename)?;
    session.run_samples(10)?;

    assert!(std::fs::exists(filename).unwrap());

    Ok(())
}

#[test]
fn test_wavedrom() -> Result<(), SrError> {
    let mut session: Session = Session::try_from("demo")?;
    let filename = "tmp/wave.json";

    session.set_output("wavedrom", filename)?;
    session.run_samples(10)?;

    assert!(std::fs::exists(filename).unwrap());

    Ok(())
}
