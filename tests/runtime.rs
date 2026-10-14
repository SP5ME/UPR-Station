use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use ultimatepr::runtime::{self, RuntimeOptions};

struct Temporary(PathBuf);
impl Temporary {
    fn new() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp")
            .join(format!(
            "station-runtime-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
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
fn desktop_runtime_is_authenticated_locked_and_releases_resources_on_stop() {
    let dir = Temporary::new();
    let options = RuntimeOptions {
        address: "127.0.0.1:0".into(),
        config: dir.0.join("station.conf"),
        web_config: dir.0.join("web.conf"),
        web_config_explicit: true,
        demo: false,
        desktop: false,
        desktop_token: Some("a".repeat(64)),
        install_signals: false,
    };
    let station = runtime::start(options.clone(), None).unwrap();
    assert_ne!(station.address.port(), 0);
    assert!(request(station.address, "/healthz", "").starts_with("HTTP/1.1 200"));
    assert!(request(station.address, "/api/state", "").starts_with("HTTP/1.1 401"));
    let auth = format!(
        "X-Station-Request: 1\r\nAuthorization: Bearer {}\r\n",
        "a".repeat(64)
    );
    let web_details = post_request(station.address, "/api/web/details", &auth);
    assert!(web_details.contains("\"runtime\":\"server\""));
    assert!(web_details.contains("\"webPanel\":true"));
    let update_status = post_request(station.address, "/api/update/status", &auth);
    assert!(update_status.contains("\"state\":\"idle\""));
    let update_status_get = request(station.address, "/api/update/status", &auth);
    assert!(update_status_get.starts_with("HTTP/1.1 200"));
    assert!(update_status_get.contains("\"state\":\"idle\""));
    let uploaded = post_raw_request(
        station.address,
        "/api/update/upload",
        &auth,
        b"test update payload",
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
    assert!(runtime::start(options.clone(), None).is_err());
    station.request_stop();
    station.wait().unwrap();
    let restarted = runtime::start(options, None).unwrap();
    let address = restarted.address;
    drop(restarted); // Drop must stop and join rather than leave a radio worker orphaned.
    assert!(TcpStream::connect(address).is_err());
}

#[test]
fn desktop_runtime_reports_desktop_profile() {
    let dir = Temporary::new();
    let options = RuntimeOptions {
        address: "127.0.0.1:0".into(),
        config: dir.0.join("station.conf"),
        web_config: dir.0.join("web.conf"),
        web_config_explicit: true,
        demo: false,
        desktop: true,
        desktop_token: Some("b".repeat(64)),
        install_signals: false,
    };
    let station = runtime::start(options, None).unwrap();
    let auth = format!(
        "X-Station-Request: 1\r\nAuthorization: Bearer {}\r\n",
        "b".repeat(64)
    );
    let web_details = post_request(station.address, "/api/web/details", &auth);
    assert!(web_details.contains("\"runtime\":\"desktop\""));
    assert!(web_details.contains("\"webPanel\":true"));
    station.request_stop();
    station.wait().unwrap();
}
