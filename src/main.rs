use std::ptr::null_mut;

use sigrok_rs::device::Device;
use sigrok_rs::types::SrError;

use libsigrok_sys::sigrok::{self as sr, sr_context};

fn main() -> Result<(), SrError> {
    // println!(
    //     "Lib. version: {}; Package version: {}",
    //     get_sr_lib_version(),
    //     get_sr_package_version()
    // );

    // //let session = Session::try_from("demo").unwrap();

    // let session = Session::try_from("fx2lafw").unwrap();

    // // let output_modules = OutputModule::scan();

    // // println!("{:?}", output_modules);

    // // let input_modules = InputModule::scan();

    // // println!("{:?}", input_modules);

    // //session.set_trigger(TriggerEvent::One)?;
    // println!("Hello");
    // session.run(10000)?;
    // println!("Bye");
    // Ok(())

    let mut context: *mut sr_context = null_mut();
    unsafe { sr::sr_init(&mut context) };
    //sr_try!(sr::sr_log_loglevel_set(LogLevel::LogSpew as i32));

    let devices: Vec<Device> = Device::scan(context)?;
    assert!(!devices.is_empty());

    dbg!(&devices);

    for device in &devices {
        println!("{}", device);
    }

    // let demo_device = devices
    //     .into_iter()
    //     .find(|dev| dev.get_model() == "Demo device")
    //     .expect("Demo device should exist");
    // assert!(demo_device.driver.get_name() == "demo");

    //sr_try!(sr::sr_exit(context));
    unsafe { sr::sr_exit(context) };
    Ok(())
}
