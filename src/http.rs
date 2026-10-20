//! Deliberately small HTTP/1.1 subset: one bounded request per connection, no pipelining,
//! chunking, keep-alive or arbitrary filesystem routes. Four workers isolate radio timers.
use crate::{
    json,
    station::{Reply, Request},
    transfers::{MAX_UPLOAD, safe_name},
};
use std::{
    collections::HashMap,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
pub type Envelope = (Request, mpsc::SyncSender<Result<Reply, String>>);
pub struct Server {
    pub address: SocketAddr,
    pub token: String,
    pub auth_enabled: bool,
    pub web_config: PathBuf,
    pub update_directory: PathBuf,
    pub snapshot: Arc<RwLock<String>>,
    pub commands: mpsc::SyncSender<Envelope>,
    pub stop: Arc<AtomicBool>,
    pub files: PathBuf,
    pub auth: crate::auth::SharedAuth,
}
struct Input {
    method: String,
    path: String,
    query: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}
impl Server {
    fn auth_is_enabled(&self) -> bool {
        crate::web_config::WebConfig::load(&self.web_config)
            .map(|config| config.auth_enabled)
            .unwrap_or(self.auth_enabled)
    }
    pub fn run(self, listener: TcpListener) -> io::Result<()> {
        listener.set_nonblocking(true)?;
        let shared = Arc::new(self);
        let (tx, rx) = mpsc::sync_channel::<TcpStream>(8);
        let rx = Arc::new(Mutex::new(rx));
        for _ in 0..4 {
            let rx = rx.clone();
            let server = shared.clone();
            thread::spawn(move || {
                loop {
                    let stream = rx.lock().unwrap().recv();
                    let Ok(mut stream) = stream else {
                        break;
                    };
                    let _ = server.handle(&mut stream);
                }
            });
        }
        while !shared.stop.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = tx.try_send(stream);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(20))
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    fn handle(&self, stream: &mut TcpStream) -> io::Result<()> {
        // Windows accepted sockets inherit the listener's nonblocking flag.
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let r = match read_request(stream) {
            Ok(r) => r,
            Err(e) => return send(stream, 400, "text/plain; charset=utf-8", e.as_bytes()),
        };
        let host = r.headers.get("host").map(String::as_str).unwrap_or("");
        if host.is_empty() {
            return send(stream, 400, "text/plain", b"Host required");
        }
        if self.address.ip().is_loopback() {
            let allowed = [
                self.address.to_string(),
                format!("localhost:{}", self.address.port()),
                format!("127.0.0.1:{}", self.address.port()),
                format!("[::1]:{}", self.address.port()),
            ];
            if !allowed.iter().any(|h| h.eq_ignore_ascii_case(host)) {
                return send(stream, 403, "text/plain", b"Untrusted host");
            }
        }
        if r.path == "/healthz" && r.method == "GET" {
            return send(stream, 200, "text/plain", b"ok\n");
        }
        if r.path.starts_with("/api/") {
            let session = r
                .headers
                .get("cookie")
                .and_then(|cookie| {
                    cookie
                        .split(';')
                        .map(str::trim)
                        .find_map(|part| part.strip_prefix("station_session="))
                })
                .unwrap_or("");
            if r.path == "/api/auth/login" && r.method == "POST" {
                let fields = match std::str::from_utf8(&r.body)
                    .map_err(|_| "Expected UTF-8".to_owned())
                    .and_then(form)
                {
                    Ok(f) => f,
                    Err(e) => return send(stream, 400, "text/plain", e.as_bytes()),
                };
                let result = self.auth.lock().unwrap().login(
                    fields.get("username").map(String::as_str).unwrap_or(""),
                    fields.get("password").map(String::as_str).unwrap_or(""),
                );
                return match result {
                    Some((value, force)) => {
                        let body = format!("{{\"forceChange\":{force}}}");
                        let cookie =
                            format!("station_session={value}; HttpOnly; SameSite=Strict; Path=/");
                        send_cookie(stream, 200, "application/json", body.as_bytes(), &cookie)
                    }
                    None => send(
                        stream,
                        401,
                        "application/json",
                        b"{\"error\":\"Invalid login\"}",
                    ),
                };
            }
            if r.path == "/api/auth/logout" && r.method == "POST" {
                self.auth.lock().unwrap().logout(session);
                return send_cookie(
                    stream,
                    200,
                    "application/json",
                    b"{}",
                    "station_session=; Max-Age=0; HttpOnly; SameSite=Strict; Path=/",
                );
            }
            let legacy = !self.token.is_empty()
                && constant_eq(
                    r.headers
                        .get("authorization")
                        .map(String::as_str)
                        .unwrap_or(""),
                    &format!("Bearer {}", self.token),
                );
            let auth_enabled = self.auth_is_enabled();
            let operator = legacy || self.auth.lock().unwrap().valid_session(session);
            // Public radio monitoring never grants software installation or
            // account takeover. Updates always require an operator session.
            let privileged = r.path.starts_with("/api/update/") && r.method == "POST"
                && !matches!(r.path.as_str(), "/api/update/check" | "/api/update/status")
                || r.path == "/api/auth/account";
            let authenticated = operator || (!auth_enabled && !privileged);
            if !authenticated {
                return send(
                    stream,
                    401,
                    "application/json",
                    b"{\"error\":\"Operator login required\"}",
                );
            }
            if r.path == "/api/auth/status" && r.method == "GET" {
                let a = self.auth.lock().unwrap();
                return send(
                    stream,
                    200,
                    "application/json",
                    format!(
                        "{{\"username\":{},\"forceChange\":{},\"enabled\":{}}}",
                        json::string(a.username()),
                        a.force_change,
                        auth_enabled
                    )
                    .as_bytes(),
                );
            }
            if r.path == "/api/build-info" && r.method == "GET" {
                return send(
                    stream,
                    200,
                    "application/json; charset=utf-8",
                    crate::update::build_info()
                    .as_bytes(),
                );
            }
            if r.path == "/api/auth/account" && r.method == "POST" {
                let fields = match std::str::from_utf8(&r.body)
                    .map_err(|_| "Expected UTF-8".to_owned())
                    .and_then(form)
                {
                    Ok(f) => f,
                    Err(e) => return send(stream, 400, "text/plain", e.as_bytes()),
                };
                let mut auth = self.auth.lock().unwrap();
                let username = fields.get("username").map(String::as_str).unwrap_or("");
                let password = fields.get("password").map(String::as_str).unwrap_or("");
                let result = if auth_enabled {
                    auth.update(session, username, password)
                } else {
                    auth.update_without_session(username, password)
                };
                return match result {
                    Ok(()) => send(stream, 200, "application/json", b"{}"),
                    Err(e) => send(
                        stream,
                        400,
                        "application/json",
                        json::object([("error", json::string(&e))]).as_bytes(),
                    ),
                };
            }
            if let Some(origin) = r.headers.get("origin")
                && origin != &format!("http://{host}")
                && origin != &format!("https://{host}")
            {
                return send(stream, 403, "text/plain", b"Cross-origin request rejected");
            }
            if r.method == "GET" && r.path == "/api/state" {
                return send(
                    stream,
                    200,
                    "application/json; charset=utf-8",
                    self.snapshot.read().unwrap().as_bytes(),
                );
            }
            if r.method == "GET" && r.path == "/api/stats" {
                let (tx, rx) = mpsc::sync_channel(1);
                if self
                    .commands
                    .try_send((
                        Request {
                            action: "stats".into(),
                            fields: HashMap::new(),
                            body: vec![],
                        },
                        tx,
                    ))
                    .is_err()
                {
                    return send(stream, 503, "text/plain", b"Station busy; retry");
                }
                return match rx.recv_timeout(Duration::from_secs(30)) {
                    Ok(Ok(reply)) => send(stream, 200, reply.mime, &reply.body),
                    Ok(Err(e)) => send(stream, 400, "text/plain", e.as_bytes()),
                    Err(_) => send(stream, 503, "text/plain", b"Station response timed out"),
                };
            }
            if r.method == "GET" && r.path == "/api/telegram/status" {
                let (tx, rx) = mpsc::sync_channel(1);
                if self
                    .commands
                    .try_send((
                        Request {
                            action: "telegram/status".into(),
                            fields: HashMap::new(),
                            body: vec![],
                        },
                        tx,
                    ))
                    .is_err()
                {
                    return send(stream, 503, "text/plain", b"Station busy; retry");
                }
                return match rx.recv_timeout(Duration::from_secs(10)) {
                    Ok(Ok(reply)) => send(stream, 200, reply.mime, &reply.body),
                    Ok(Err(e)) => send(stream, 400, "text/plain", e.as_bytes()),
                    Err(_) => send(stream, 503, "text/plain", b"Station response timed out"),
                };
            }
            if r.method == "GET" && r.path == "/api/update/check" {
                let result = form(&r.query).and_then(|fields| {
                    crate::update::check(fields.get("channel").map(String::as_str).unwrap_or("test"))
                });
                return match result {
                    Ok(body) => send(stream, 200, "application/json; charset=utf-8", body.as_bytes()),
                    Err(error) => send(stream, 400, "application/json; charset=utf-8", json::object([("error", json::string(&error))]).as_bytes()),
                };
            }
            if r.method == "GET" && r.path == "/api/update/status" {
                return send(stream, 200, "application/json", crate::update::status_json(&self.update_directory).as_bytes());
            }
            if r.method == "GET" && r.path == "/api/file" {
                let fields = match form(&r.query) {
                    Ok(f) => f,
                    Err(e) => return send(stream, 400, "text/plain", e.as_bytes()),
                };
                let name = match safe_name(fields.get("name").map(String::as_str).unwrap_or("")) {
                    Ok(n) => n,
                    Err(e) => return send(stream, 400, "text/plain", e.as_bytes()),
                };
                let path = self.files.join(name);
                if !std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file()) {
                    return send(stream, 404, "text/plain", b"File not found");
                }
                let mut file = std::fs::File::open(path)?;
                let size = file.metadata()?.len();
                head(stream, 200, "application/octet-stream", size)?;
                io::copy(&mut file, stream)?;
                return Ok(());
            }
            if r.method != "POST" {
                return send(stream, 405, "text/plain", b"Use POST");
            }
            if r.headers.get("x-station-request").map(String::as_str) != Some("1") {
                return send(stream, 403, "text/plain", b"Missing request marker");
            }
            let upload = matches!(
                r.path.as_str(),
                "/api/yapp/send" | "/api/update/upload"
            );
            if !upload
                && !r
                    .headers
                    .get("content-type")
                    .is_some_and(|v| v.starts_with("application/x-www-form-urlencoded"))
            {
                return send(stream, 415, "text/plain", b"Expected form encoding");
            }
            let fields = if upload {
                form(&r.query)
            } else {
                std::str::from_utf8(&r.body)
                    .map_err(|_| "Expected UTF-8".into())
                    .and_then(form)
            };
            let fields = match fields {
                Ok(f) => f,
                Err(e) => return send(stream, 400, "text/plain", e.as_bytes()),
            };
            // Updates are maintenance I/O, not radio-engine commands.
            if r.path.starts_with("/api/update/") {
                let root = &self.update_directory;
                let result = match r.path.as_str() {
                    "/api/update/check" => crate::update::check(fields.get("channel").map(String::as_str).unwrap_or("test")),
                    "/api/update/download" => crate::update::download(root, fields.get("tag").map(String::as_str).unwrap_or("")),
                    "/api/update/upload" => crate::update::upload(root, &r.body),
                    "/api/update/status" => Ok(crate::update::status_json(root)),
                    "/api/update/install" if fields.get("confirm").map(String::as_str) == Some("yes") => crate::update::start(root).map(|s| json::object([("message", json::string(&s))])),
                    _ => Err("Nieprawidłowe polecenie lub brak potwierdzenia instalacji".into()),
                };
                return match result {
                    Ok(body) => send(stream, 200, "application/json; charset=utf-8", body.as_bytes()),
                    Err(error) => send(stream, 400, "application/json; charset=utf-8", json::object([("error", json::string(&error))]).as_bytes()),
                };
            }
            let (tx, rx) = mpsc::sync_channel(1);
            let request = Request {
                action: r.path.trim_start_matches("/api/").into(),
                fields,
                body: if upload { r.body } else { vec![] },
            };
            if self.commands.try_send((request, tx)).is_err() {
                return send(stream, 503, "text/plain", b"Station busy; retry");
            }
            return match rx.recv_timeout(Duration::from_secs(30)) {
                Ok(Ok(reply)) => send(stream, 200, reply.mime, &reply.body),
                Ok(Err(e)) => send(
                    stream,
                    400,
                    "application/json; charset=utf-8",
                    json::object([("error", json::string(&e))]).as_bytes(),
                ),
                Err(_) => send(
                    stream,
                    503,
                    "text/plain",
                    b"Station response timed out; inspect state before retrying",
                ),
            };
        }
        if r.method != "GET" {
            return send(stream, 405, "text/plain", b"Use GET");
        }
        let (mime, bytes): (&str, &[u8]) = match r.path.as_str() {
            "/" => (
                "text/html; charset=utf-8",
                include_bytes!("../web/index.html"),
            ),
            "/app.css" => ("text/css; charset=utf-8", include_bytes!("../web/app.css")),
            "/app.js" => (
                "text/javascript; charset=utf-8",
                include_bytes!("../web/app.js"),
            ),
            "/icons.svg" => ("image/svg+xml", include_bytes!("../web/icons.svg")),
            "/ultimatepr-logo.png" => (
                "image/png",
                include_bytes!("../assets/branding/ultimatepr-logo.png"),
            ),
            "/lang/en.json" => (
                "application/json; charset=utf-8",
                include_bytes!("../web/lang/en.json"),
            ),
            "/lang/pl.json" => (
                "application/json; charset=utf-8",
                include_bytes!("../web/lang/pl.json"),
            ),
            _ => return send(stream, 404, "text/plain", b"Not found"),
        };
        send(stream, 200, mime, bytes)
    }
}
fn constant_eq(a: &str, b: &str) -> bool {
    let mut diff = a.len() ^ b.len();
    for (i, byte) in b.bytes().enumerate() {
        diff |= (a.as_bytes().get(i).copied().unwrap_or(0) ^ byte) as usize;
    }
    diff == 0
}
fn read_request(stream: &mut TcpStream) -> Result<Input, String> {
    let start = Instant::now();
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    let header_end;
    loop {
        if start.elapsed() > Duration::from_secs(10) {
            return Err("Header timeout".into());
        }
        let n = stream.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Incomplete request".into());
        }
        bytes.extend_from_slice(&buffer[..n]);
        if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = i + 4;
            if header_end > 16384 {
                return Err("Header too large".into());
            }
            break;
        }
        if bytes.len() > 16384 {
            return Err("Header too large".into());
        }
    }
    let header = std::str::from_utf8(&bytes[..header_end]).map_err(|_| "Invalid header")?;
    let mut lines = header.split("\r\n");
    let words = lines
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>();
    if words.len() != 3
        || words[2] != "HTTP/1.1"
        || !words[1].starts_with('/')
        || words[1].len() > 4096
    {
        return Err("Invalid request line".into());
    }
    let method = words[0].to_owned();
    let (path, query) = words[1].split_once('?').unwrap_or((words[1], ""));
    let (path, query) = (path.to_owned(), query.to_owned());
    let mut headers = HashMap::new();
    for line in lines.filter(|s| !s.is_empty()) {
        let (k, v) = line.split_once(':').ok_or("Invalid header")?;
        let key = k.to_ascii_lowercase();
        if k.is_empty()
            || k.bytes().any(|b| !b.is_ascii_alphanumeric() && b != b'-')
            || headers.insert(key, v.trim().to_owned()).is_some()
        {
            return Err("Invalid or duplicate header".into());
        }
    }
    if headers.contains_key("transfer-encoding") || headers.contains_key("expect") {
        return Err("Chunked/Expect requests are unsupported".into());
    }
    let length = headers
        .get("content-length")
        .map(|n| n.parse::<usize>())
        .transpose()
        .map_err(|_| "Invalid content length")?
        .unwrap_or(0);
    let limit = if matches!(path.as_str(), "/api/yapp/send" | "/api/update/upload") {
        MAX_UPLOAD
    } else {
        256 * 1024
    };
    if length > limit {
        return Err("Request body too large".into());
    }
    let mut body = bytes[header_end..].to_vec();
    if body.len() > length {
        return Err("Pipelining is unsupported".into());
    }
    while body.len() < length {
        if start.elapsed() > Duration::from_secs(30) {
            return Err("Upload timeout".into());
        }
        let cap = buffer.len().min(length - body.len());
        let n = stream.read(&mut buffer[..cap]).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Incomplete body".into());
        }
        body.extend_from_slice(&buffer[..n]);
    }
    Ok(Input {
        method,
        path,
        query,
        headers,
        body,
    })
}
pub fn form(text: &str) -> Result<HashMap<String, String>, String> {
    fn decode(s: &str) -> Result<String, String> {
        let mut out = Vec::new();
        let mut bytes = s.bytes();
        while let Some(b) = bytes.next() {
            match b {
                b'+' => out.push(b' '),
                b'%' => {
                    let a = bytes.next().ok_or("Bad escape")?;
                    let b = bytes.next().ok_or("Bad escape")?;
                    let hex = |c: u8| (c as char).to_digit(16).ok_or("Bad escape");
                    out.push((hex(a)? * 16 + hex(b)?) as u8);
                }
                b => out.push(b),
            }
        }
        String::from_utf8(out).map_err(|_| "Invalid UTF-8".into())
    }
    let mut out = HashMap::new();
    for pair in text.split('&').filter(|s| !s.is_empty()) {
        if out.len() >= 32 {
            return Err("Too many fields".into());
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if out.insert(decode(k)?, decode(v)?).is_some() {
            return Err("Duplicate field".into());
        }
    }
    Ok(out)
}
fn head(stream: &mut TcpStream, status: u16, mime: &str, length: u64) -> io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        415 => "Unsupported Media Type",
        _ => "Service Unavailable",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {mime}\r\nContent-Length: {length}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'\r\n\r\n"
    )
}
fn send(stream: &mut TcpStream, status: u16, mime: &str, bytes: &[u8]) -> io::Result<()> {
    head(stream, status, mime, bytes.len() as u64)?;
    stream.write_all(bytes)
}
fn send_cookie(
    stream: &mut TcpStream,
    status: u16,
    mime: &str,
    bytes: &[u8],
    cookie: &str,
) -> io::Result<()> {
    let reason = if status == 200 { "OK" } else { "Unauthorized" };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nSet-Cookie: {cookie}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'\r\n\r\n",
        bytes.len()
    )?;
    stream.write_all(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_forms() {
        assert_eq!(form("text=%C5%BC+ok").unwrap()["text"], "ż ok");
        assert!(form("a=1&a=2").is_err());
        assert!(form("a=%ff").is_err());
        assert!(form("a=%").is_err());
    }
}
