//! YAPP transfer lifecycle, separated from HTTP and from station rendering.
use crate::json;
use crate::{
    link::{Engine, Event, SESSIONS, State},
    yapp::{self, Decoder, Packet},
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub const MAX_UPLOAD: usize = 16 * 1024 * 1024;
const MAX_RECEIVE: u64 = 1024 * 1024 * 1024;
#[derive(PartialEq)]
enum Stage {
    Confirm,
    Header,
    Receive,
    End,
    Ready,
    File,
    Data,
    Eof,
    Eot,
}
struct Transfer {
    stage: Stage,
    decoder: Decoder,
    file: Option<File>,
    temporary: Option<PathBuf>,
    destination: Option<PathBuf>,
    name: String,
    size: u64,
    done: u64,
    last: Instant,
    sending: bool,
}
impl Drop for Transfer {
    fn drop(&mut self) {
        self.file.take();
        if let Some(path) = &self.temporary {
            let _ = fs::remove_file(path);
        }
    }
}
pub struct Transfers {
    active: [Option<Transfer>; SESSIONS],
    probe: [Vec<u8>; SESSIONS],
    pub directory: PathBuf,
    pub notices: Vec<(usize, String)>,
    pub received: usize,
    pub max_files: usize,
    pub auto_cleanup: bool,
}
pub fn safe_name(raw: &str) -> Result<String, String> {
    let stem = raw
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if raw.is_empty()
        || raw.len() > 128
        || raw.ends_with(['.', ' '])
        || !raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._- ()[]".contains(&b))
        || [".", "..", "CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || ["COM", "LPT"].iter().any(|p| {
            stem.strip_prefix(p)
                .is_some_and(|s| matches!(s, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
        })
    {
        return Err("Invalid filename (ASCII letters, digits, spaces and ._-()[] only)".into());
    }
    Ok(raw.into())
}
impl Transfers {
    pub fn new(directory: PathBuf, max_files: usize, auto_cleanup: bool) -> Self {
        let _ = fs::create_dir_all(&directory);
        let _ = cleanup_orphans(&directory);
        Self {
            active: std::array::from_fn(|_| None),
            probe: std::array::from_fn(|_| Vec::new()),
            directory,
            notices: vec![],
            received: 0,
            max_files: max_files.clamp(1, 100000),
            auto_cleanup,
        }
    }
    pub fn list_files(&self) -> Vec<(String, u64, u64)> {
        let mut files = fs::read_dir(&self.directory)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let ty = entry.file_type().ok()?;
                if !ty.is_file() {
                    return None;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || safe_name(&name).is_err() {
                    return None;
                }
                let meta = entry.metadata().ok()?;
                let modified = meta
                    .modified()
                    .ok()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?
                    .as_secs();
                Some((name, meta.len(), modified))
            })
            .collect::<Vec<_>>();
        files.sort_by(|a, b| a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()));
        files
    }
    pub fn used_bytes(&self) -> u64 {
        self.list_files().iter().map(|(_, size, _)| *size).sum()
    }
    pub fn delete_file(&mut self, raw: &str) -> Result<(), String> {
        let name = safe_name(raw)?;
        if self.active.iter().flatten().any(|t| {
            t.name == name
                || t.destination
                    .as_ref()
                    .is_some_and(|p| p.file_name().and_then(|n| n.to_str()) == Some(name.as_str()))
        }) {
            return Err("Cannot delete a file during an active transfer".into());
        }
        let path = self.directory.join(&name);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err("File not found".into()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn busy(&self, id: usize) -> bool {
        self.active[id].is_some()
    }
    pub fn handles(&self, id: usize, bytes: &[u8]) -> bool {
        self.busy(id)
            || !self.probe[id].is_empty()
            || bytes.last() == Some(&5)
            || bytes.windows(2).any(|p| p == yapp::send_init())
    }
    pub fn clear(&mut self, id: usize) {
        self.active[id] = None;
        self.probe[id].clear();
    }
    pub fn snapshot(&self, id: usize) -> String {
        match &self.active[id] {
            None => "null".into(),
            Some(t) => json::object([
                ("name", json::string(&t.name)),
                ("sending", t.sending.to_string()),
                ("done", t.done.to_string()),
                ("size", t.size.to_string()),
                ("confirm", (t.stage == Stage::Confirm).to_string()),
            ]),
        }
    }
    pub fn start(
        &mut self,
        id: usize,
        name: &str,
        bytes: &[u8],
        engine: &mut Engine,
        now: u64,
    ) -> Result<Vec<Event>, String> {
        if self.busy(id) {
            return Err("A transfer is already active".into());
        }
        if !engine.sessions[id]
            .as_ref()
            .is_some_and(|s| s.state == State::Connected)
        {
            return Err("Connect a session first".into());
        }
        let name = safe_name(name)?;
        if bytes.len() > MAX_UPLOAD {
            return Err("Web upload limit is 16 MiB".into());
        }
        fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
        let (mut file, path) = temporary(&self.directory, id)?;
        if let Err(e) = file.write_all(bytes) {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(e.to_string());
        }
        drop(file);
        let file = File::open(&path).map_err(|e| e.to_string())?;
        let transfer = Transfer {
            stage: Stage::Ready,
            decoder: Decoder::default(),
            file: Some(file),
            temporary: Some(path),
            destination: None,
            name,
            size: bytes.len() as u64,
            done: 0,
            last: Instant::now(),
            sending: true,
        };
        let events = engine.send(id, &yapp::send_init(), now)?;
        self.active[id] = Some(transfer);
        Ok(events)
    }
    pub fn answer(
        &mut self,
        id: usize,
        accept: bool,
        engine: &mut Engine,
        now: u64,
    ) -> Result<Vec<Event>, String> {
        let t = self.active[id].as_mut().ok_or("No incoming transfer")?;
        if t.stage != Stage::Confirm {
            return Err("Transfer is not waiting for confirmation".into());
        }
        let events = engine.send(
            id,
            &if accept {
                yapp::control(1)
            } else {
                yapp::not_ready("Rejected by operator")
            },
            now,
        )?;
        if accept {
            t.stage = Stage::Header;
            t.last = Instant::now();
        } else {
            self.clear(id);
        }
        Ok(events)
    }
    pub fn cancel(
        &mut self,
        id: usize,
        engine: &mut Engine,
        now: u64,
    ) -> Result<Vec<Event>, String> {
        if !self.busy(id) {
            return Err("No active transfer".into());
        }
        let events = engine.send(id, &yapp::cancel("Cancelled by operator"), now);
        self.clear(id);
        events
    }
    pub fn receive(
        &mut self,
        id: usize,
        bytes: &[u8],
        enabled: bool,
        auto: bool,
        engine: &mut Engine,
        now: u64,
    ) -> (Vec<u8>, Vec<Event>) {
        let mut text = Vec::new();
        let mut events = Vec::new();
        let mut data = bytes.to_vec();
        if self.active[id].is_none() {
            let mut probe = std::mem::take(&mut self.probe[id]);
            probe.extend_from_slice(bytes);
            if let Some(start) = probe.windows(2).position(|p| p == yapp::send_init()) {
                text.extend_from_slice(&probe[..start]);
                data = probe[start + 2..].to_vec();
                if !enabled {
                    if let Ok(ev) = engine.send(id, &yapp::not_ready("YAPP disabled"), now) {
                        events.extend(ev);
                    }
                    return (text, events);
                }
                self.active[id] = Some(Transfer {
                    stage: if auto { Stage::Header } else { Stage::Confirm },
                    decoder: Decoder::default(),
                    file: None,
                    temporary: None,
                    destination: None,
                    name: String::new(),
                    size: 0,
                    done: 0,
                    last: Instant::now(),
                    sending: false,
                });
                if auto {
                    if let Ok(ev) = engine.send(id, &yapp::control(1), now) {
                        events.extend(ev);
                    }
                } else {
                    self.notices
                        .push((id, "Incoming YAPP: operator approval required".into()));
                }
            } else {
                if probe.last() == Some(&5) {
                    probe.pop();
                    self.probe[id].push(5);
                }
                return (probe, events);
            }
        }
        let mut t = self.active[id].take().unwrap();
        t.last = Instant::now();
        let result = (|| -> Result<bool, String> {
            for packet in t.decoder.push(&data)? {
                let (keep, wire) = self.packet(id, &mut t, packet)?;
                if !wire.is_empty() {
                    events.extend(engine.send(id, &wire, now)?);
                }
                if !keep {
                    return Ok(false);
                }
            }
            Ok(true)
        })();
        match result {
            Ok(true) => self.active[id] = Some(t),
            Ok(false) => {}
            Err(e) => {
                self.notices.push((id, format!("YAPP: {e}")));
                if let Ok(ev) = engine.send(id, &yapp::cancel("Transfer error"), now) {
                    events.extend(ev);
                }
            }
        }
        (text, events)
    }
    fn packet(
        &mut self,
        id: usize,
        t: &mut Transfer,
        p: Packet,
    ) -> Result<(bool, Vec<u8>), String> {
        match p {
            Packet::Cancel(reason) | Packet::NotReady(reason) => {
                self.notices.push((
                    id,
                    format!("YAPP stopped: {}", String::from_utf8_lossy(&reason)),
                ));
                return Ok((false, yapp::control(5)));
            }
            Packet::CanAck => return Ok((false, vec![])),
            _ => {}
        }
        let wire = match p {
            Packet::SendInit if t.stage == Stage::Header => yapp::control(1),
            Packet::SendInit if t.stage == Stage::Confirm => vec![],
            Packet::Header { filename, size } if t.stage == Stage::Header => {
                t.name = safe_name(&filename)?;
                if size > MAX_RECEIVE {
                    return Err("File exceeds 1 GiB".into());
                }
                t.size = size;
                self.prepare_capacity()?;
                fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
                let (file, path) = temporary(&self.directory, id)?;
                t.file = Some(file);
                t.temporary = Some(path);
                t.destination = Some(self.directory.join(&t.name));
                t.stage = Stage::Receive;
                yapp::control(2)
            }
            Packet::Data(bytes) if t.stage == Stage::Receive => {
                if t.done + bytes.len() as u64 > t.size {
                    return Err("Data exceeds declared size".into());
                }
                t.file
                    .as_mut()
                    .unwrap()
                    .write_all(&bytes)
                    .map_err(|e| e.to_string())?;
                t.done += bytes.len() as u64;
                vec![]
            }
            Packet::Eof if t.stage == Stage::Receive => {
                if t.done != t.size {
                    return Err("File size mismatch".into());
                }
                t.file
                    .take()
                    .unwrap()
                    .sync_all()
                    .map_err(|e| e.to_string())?;
                t.stage = Stage::End;
                yapp::control(3)
            }
            Packet::Eot if t.stage == Stage::End => {
                // Same-filesystem hard link atomically publishes a complete file without overwriting.
                let temporary = t.temporary.as_ref().unwrap();
                let mut destination = t.destination.clone().unwrap();
                let mut committed = false;
                for n in 0..1000 {
                    if n > 0 {
                        destination = self.directory.join(format!("{n}-{}", t.name));
                    }
                    match fs::hard_link(temporary, &destination) {
                        Ok(()) => {
                            committed = true;
                            break;
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                        Err(e) => return Err(e.to_string()),
                    }
                }
                if !committed {
                    return Err("Too many files with this name".into());
                }
                self.received += 1;
                self.notices.push((
                    id,
                    format!(
                        "YAPP received: {}",
                        destination.file_name().unwrap().to_string_lossy()
                    ),
                ));
                return Ok((false, yapp::control(4)));
            }
            Packet::Ready if t.stage == Stage::Ready => {
                t.stage = Stage::File;
                yapp::header(&t.name, t.size)?
            }
            Packet::ReceiveFile if t.stage == Stage::File => {
                t.stage = Stage::Data;
                vec![]
            }
            Packet::AckEof if t.stage == Stage::Eof => {
                t.stage = Stage::Eot;
                yapp::end_of_transfer().to_vec()
            }
            Packet::AckEot if t.stage == Stage::Eot => {
                self.notices.push((id, format!("YAPP sent: {}", t.name)));
                return Ok((false, vec![]));
            }
            _ => return Err("Unexpected YAPP packet".into()),
        };
        Ok((true, wire))
    }
    pub fn tick(&mut self, engine: &mut Engine, now: u64) -> Vec<Event> {
        let mut events = vec![];
        for id in 0..SESSIONS {
            let Some(mut t) = self.active[id].take() else {
                continue;
            };
            if engine.sessions[id].is_none() {
                continue;
            }
            if t.last.elapsed() > Duration::from_secs(120) {
                self.notices.push((id, "YAPP timeout".into()));
                if let Ok(ev) = engine.send(id, &yapp::cancel("Timeout"), now) {
                    events.extend(ev);
                }
                continue;
            }
            let result = (|| -> Result<(), String> {
                if t.stage == Stage::Data && engine.sessions[id].as_ref().unwrap().buffered() < 4096
                {
                    if t.done < t.size {
                        // A YAPP data packet has a two-byte envelope. Keep
                        // the complete AX.25 information field within the
                        // negotiated N1 limit (256 is the YAPP maximum).
                        let payload_limit = engine
                            .tx_info_limit(id)
                            .unwrap_or(2)
                            .saturating_sub(2)
                            .min(256);
                        if payload_limit == 0 {
                            return Err(
                                "Negotiated AX.25 information size is too small for YAPP".into()
                            );
                        }
                        let mut data = [0; 256];
                        let n = t
                            .file
                            .as_mut()
                            .unwrap()
                            .read(&mut data[..payload_limit])
                            .map_err(|e| e.to_string())?;
                        if n == 0 {
                            return Err("Unexpected end of upload".into());
                        }
                        events.extend(engine.send(id, &yapp::data(&data[..n])?, now)?);
                        t.done += n as u64;
                        t.last = Instant::now();
                    } else {
                        events.extend(engine.send(id, &yapp::end_of_file(), now)?);
                        t.stage = Stage::Eof;
                        t.last = Instant::now();
                    }
                }
                Ok(())
            })();
            match result {
                Ok(()) => self.active[id] = Some(t),
                Err(e) => {
                    self.notices.push((id, e));
                    if let Ok(ev) = engine.send(id, &yapp::cancel("Read error"), now) {
                        events.extend(ev);
                    }
                }
            }
        }
        events
    }
}
impl Transfers {
    fn prepare_capacity(&mut self) -> Result<(), String> {
        let mut files = self.list_files();
        if files.len() < self.max_files {
            return Ok(());
        }
        if !self.auto_cleanup {
            return Err(format!("YAPP file limit reached ({})", self.max_files));
        }
        files.sort_by_key(|(_, _, modified)| *modified);
        let active_names = self
            .active
            .iter()
            .flatten()
            .filter_map(|t| {
                t.destination
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
            })
            .collect::<Vec<_>>();
        while files.len() >= self.max_files {
            let Some(index) = files
                .iter()
                .position(|(name, _, _)| !active_names.iter().any(|active| *active == name))
            else {
                return Err("YAPP file limit reached; no removable file".into());
            };
            let name = files.remove(index).0;
            fs::remove_file(self.directory.join(name)).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
fn cleanup_orphans(directory: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.file_type()?.is_file() && name.starts_with(".station-") && name.ends_with(".part")
        {
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(())
}
fn temporary(directory: &Path, id: usize) -> Result<(File, PathBuf), String> {
    for n in 0..1000 {
        let path = directory.join(format!(".station-{}-{id}-{n}.part", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(f) => return Ok((f, path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("Cannot reserve transfer file".into())
}
