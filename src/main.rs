use ultimatepr::runtime::{self, RuntimeOptions};
fn run() -> Result<(), String> {
    let Some(options) = RuntimeOptions::from_cli(std::env::args().skip(1))? else {
        return Ok(());
    };
    let path = options.config.clone();
    let station = runtime::start(options)?;
    println!(
        "UPR Station: http://{}\nConfiguration: {}\nRadio services remain active independently of the browser.",
        station.address,
        path.display()
    );
    station.wait()
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Station: {e}");
        std::process::exit(1);
    }
}
