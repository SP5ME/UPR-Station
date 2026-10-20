use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use ultimatepr::runtime::{self, RuntimeOptions};

#[test]
fn retired_startup_option_is_rejected_instead_of_starting_radio_services() {
    let error = RuntimeOptions::from_cli(["--demo".to_owned()]).err().unwrap();
    assert_eq!(error, "Unknown argument: --demo");
    let options = RuntimeOptions::from_cli(Vec::<String>::new()).unwrap().unwrap();
    assert_eq!(options.config, PathBuf::from("data/station.conf"));
    assert_eq!(options.web_config, PathBuf::from("data/web.conf"));
}

struct Temporary(PathBuf);
impl Temporary {
    fn new() -> Self {
        // Windows timestamps can be equal for tests starting on different threads.
        static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp")
            .join(format!(
            "station-runtime-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
#[test]
fn temporary_directories_are_distinct() {
    let first = Temporary::new();
    let second = Temporary::new();
    assert_ne!(first.0, second.0);
}

#[test]
fn public_panel_does_not_grant_update_or_account_access() {
    let dir = Temporary::new();
    fs::write(dir.0.join("web.conf"), "bind=127.0.0.1:0\nauth_enabled=false\n").unwrap();
    let station = runtime::start(RuntimeOptions {
        address: "127.0.0.1:0".into(),
        config: dir.0.join("station.conf"),
        web_config: dir.0.join("web.conf"),
        web_config_explicit: true,
        token_override: None,
        install_signals: false,
    }).unwrap();
    assert!(request(station.address, "/api/state", "").starts_with("HTTP/1.1 200"));
    for route in ["update/upload", "update/install", "update/download", "auth/account"] {
        assert!(post_request(station.address, &format!("/api/{route}"), "X-Station-Request: 1\r\n").starts_with("HTTP/1.1 401"), "{route}");
    }
    // Read-only checks follow normal panel access, not installation privileges.
    // Invalid input avoids external GitHub calls in this offline test.
    let check = request(station.address, "/api/update/check?channel=invalid", "");
    assert!(check.starts_with("HTTP/1.1 400"));
    assert!(check.contains("kanał aktualizacji"));
    assert!(post_request(station.address, "/api/update/check", "").starts_with("HTTP/1.1 403"));
    let login = post_raw_request(station.address, "/api/auth/login", "X-Station-Request: 1\r\n", b"username=admin&password=packet");
    assert!(login.starts_with("HTTP/1.1 200"));
    let cookie = login.lines().find_map(|l| l.strip_prefix("Set-Cookie: ")).unwrap().split(';').next().unwrap();
    let auth = format!("X-Station-Request: 1\r\nCookie: {cookie}\r\n");
    assert!(post_request(station.address, "/api/update/status", &auth).starts_with("HTTP/1.1 200"));
    // Missing confirmation cannot trigger installation, even for an operator.
    assert!(post_request(station.address, "/api/update/install", &auth).starts_with("HTTP/1.1 400"));
    assert!(post_raw_request(station.address, "/api/update/upload", &auth, b"not a zip").starts_with("HTTP/1.1 400"));
    let build = request(station.address, "/api/build-info", "");
    assert!(build.contains(&format!("\"version\":\"{}\"", env!("CARGO_PKG_VERSION"))));
    assert!(build.contains("\"architecture\":") && build.contains("\"tag\":") && build.contains("\"commit\":"));
    assert!(request(station.address, "/api/state", "").starts_with("HTTP/1.1 200"));
    drop(station);
}
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn request(address: std::net::SocketAddr, path: &str, headers: &str) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {address}\r\n{headers}\r\n"
    )
    .unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    text
}
fn post_request(address: std::net::SocketAddr, path: &str, headers: &str) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 0\r\n{headers}\r\n"
    )
    .unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    text
}
fn post_raw_request(
    address: std::net::SocketAddr,
    path: &str,
    headers: &str,
    body: &[u8],
) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n{headers}\r\n",
        body.len()
    )
    .unwrap();
    stream.write_all(body).unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    text
}
#[test]
fn server_runtime_is_authenticated_locked_and_releases_resources_on_stop() {
    let dir = Temporary::new();
    let options = RuntimeOptions {
        address: "127.0.0.1:0".into(),
        config: dir.0.join("station.conf"),
        web_config: dir.0.join("web.conf"),
        web_config_explicit: true,
        token_override: Some("a".repeat(64)),
        install_signals: false,
    };
    let station = runtime::start(options.clone()).unwrap();
    assert_ne!(station.address.port(), 0);
    assert!(request(station.address, "/healthz", "").starts_with("HTTP/1.1 200"));
    assert!(request(station.address, "/api/state", "").starts_with("HTTP/1.1 401"));
    assert!(request(station.address, "/api/update/check?channel=invalid", "").starts_with("HTTP/1.1 401"));
    let auth = format!(
        "X-Station-Request: 1\r\nAuthorization: Bearer {}\r\n",
        "a".repeat(64)
    );
    let web_details = post_request(station.address, "/api/web/details", &auth);
    assert!(web_details.contains("\"runtime\":\"server\""));
    assert!(web_details.contains("\"webPanel\":true"));
    let snapshot = request(station.address, "/api/state", &auth);
    assert!(snapshot.starts_with("HTTP/1.1 200"));
    assert!(!snapshot.contains("\"demo\":"));
    let update_status = post_request(station.address, "/api/update/status", &auth);
    assert!(update_status.contains("\"state\":\"idle\""));
    let update_status_get = request(station.address, "/api/update/status", &auth);
    assert!(update_status_get.starts_with("HTTP/1.1 200"));
    assert!(update_status_get.contains("\"state\":\"idle\""));
    let uploaded = post_raw_request(
        station.address,
        "/api/update/upload",
        &auth,
        b"PK\x03\x04test update payload",
    );
    assert!(uploaded.starts_with("HTTP/1.1 200"));
    assert!(uploaded.contains("\"sha256\":"));
    let uploaded_status = post_request(station.address, "/api/update/status", &auth);
    assert!(uploaded_status.contains("\"state\":\"uploaded\""));
    assert!(request(station.address, "/api/state", &auth).starts_with("HTTP/1.1 200"));
    assert!(
        request(
            station.address,
            "/api/state",
            &(auth + "Origin: https://untrusted.example\r\n")
        )
        .starts_with("HTTP/1.1 403")
    );
    assert!(runtime::start(options.clone()).is_err());
    station.request_stop();
    station.wait().unwrap();
    let restarted = runtime::start(options).unwrap();
    let address = restarted.address;
    drop(restarted); // Drop must stop and join rather than leave a radio worker orphaned.
    assert!(TcpStream::connect(address).is_err());
}
