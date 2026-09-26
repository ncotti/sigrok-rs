use sigrok_rs::session::Session;
use sigrok_rs::types::SrError;

fn main() -> Result<(), SrError> {
    let mut session = Session::try_from("demo").unwrap();
    session.set_output("bits", "tmp.txt")?;
    session.run_samples(100)?;

    //println!("{}", session.device);

    Ok(())
}
