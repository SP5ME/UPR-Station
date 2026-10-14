//! Standalone local demonstration TNC.
fn main() -> std::io::Result<()> {
    let listener = std::net::TcpListener::bind("127.0.0.1:18001")?;
    println!("Demo KISS TNC: 127.0.0.1:18001");
    ultimatepr::demo::run(
        listener,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    )
}
