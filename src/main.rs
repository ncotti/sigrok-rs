use sigrok_rs::{input_module::InputModule, output_module::OutputModule, types::SrError, *};
use sigrok_rs::trigger::TriggerEvent;

fn main() -> Result<(), SrError> {
    println!(
        "Lib. version: {}; Package version: {}",
        get_sr_lib_version(),
        get_sr_package_version()
    );

    //let session = Session::try_from("demo").unwrap();

    let session = Session::try_from("fx2lafw").unwrap();

    // let output_modules = OutputModule::scan();

    // println!("{:?}", output_modules);

    // let input_modules = InputModule::scan();

    // println!("{:?}", input_modules);

    
    //session.set_trigger(TriggerEvent::One)?;
    println!("Hello");
    session.run(10000)?;
    println!("Bye");
    Ok(())
}
