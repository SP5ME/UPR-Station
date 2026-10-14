use ultimatepr::runtime::{self, RuntimeOptions};
fn run() -> Result<(), String> {
    let Some(options) = RuntimeOptions::from_cli(std::env::args().skip(1))? else {
        return Ok(());
    };
    let path = options.config.clone();
    let demo = options.demo;
    let station = runtime::start(options, None)?;
    println!(
        "UltimatePR Station: http://{}\nConfiguration: {}\n{}",
        station.address,
        path.display(),
        if demo {
            "DEMO: connect to DEMO; no radio attached."
        } else {
            "Radio services remain active independently of the browser."
        }
    );
    station.wait()
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Station: {e}");
        std::process::exit(1);
    }
}
