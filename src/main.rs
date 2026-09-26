use std::time::Duration;

use sigrok_rs::Device;
use sigrok_rs::session::Session;
use sigrok_rs::types::SrError;

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

    // let mut session = Session::try_from("demo").unwrap();
    // session.run_timeout(Duration::from_secs(1))?;

    // let output_modules_info = Session::scan_output()?;
    // dbg!(output_modules_info);

    let devices: Vec<Device> = Session::scan()?;
    let demo_device: Device = devices
        .into_iter()
        .find(|dev| dev.get_driver_name() == "demo")
        .unwrap();

    println!("{}", demo_device);

    Ok(())
}
