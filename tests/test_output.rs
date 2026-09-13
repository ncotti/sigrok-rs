//! This file test the different output modules

use sigrok_rs::{Session, SrError, session};
use tempfile::NamedTempFile;

// #[test]
// fn test_ascii() -> Result<(), SrError> {
//     let output_file: NamedTempFile = NamedTempFile::new().unwrap();
//     let mut session: Session = Session::try_from("demo")?;

//     session.set_output("ascii", output_file.path())?;

//     session.run_samples(5)?;

//     Ok(())
// }
