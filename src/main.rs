// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

use sigrok_rs::{Session, SrError};

fn main() -> Result<(), SrError> {
    let mut session = Session::autoconnect().unwrap_or_else(|_| Session::try_from("demo").unwrap());

    session.set_output("bits", "tmp.txt")?;
    session.device.set_option("samplerate", "200000")?;
    let data = session.run_samples(100)?;
    assert!(data.len() == 100);

    Ok(())
}
