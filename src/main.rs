use std::ptr::null_mut;

use sigrok_rs::types::SrError;
use sigrok_rs::{device::Device, session::Session};

use libsigrok_sys::sigrok::{self as sr, sr_context};

fn main() -> Result<(), SrError> {
    // println!(
    //     "Lib. version: {}; Package version: {}",
    //     get_sr_lib_version(),
    //     get_sr_package_version()
    // );

    // // let output_modules = OutputModule::scan();

    // // println!("{:?}", output_modules);

    // // let input_modules = InputModule::scan();

    // // println!("{:?}", input_modules);

    //let session = Session::try_from("fx2lafw").unwrap();
    // //session.set_trigger(TriggerEvent::One)?;

    let session = Session::try_from("demo").unwrap();
    session.run()?;

    Ok(())
}
