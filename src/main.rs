// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

use sigrok_rs::{Session, SrError};

fn main() -> Result<(), SrError> {
    let mut session = Session::try_from("demo").unwrap();
    session.set_output("bits", "tmp.txt")?;
    session.run_samples(100)?;

    println!("{}", session.device);

    Ok(())
}
