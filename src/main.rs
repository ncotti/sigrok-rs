use sigrok_rs::*;

fn main() {
    println!(
        "Lib. version: {}; Package version: {}",
        get_sr_lib_version(),
        get_sr_package_version()
    );

    let _session = Session::try_from("demo").unwrap();
}
