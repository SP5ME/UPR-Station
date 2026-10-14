//! Local automatic services. Only complete received text lines on connected sessions are commands.
use crate::{
    ax25::Frame,
    config::Config,
    link::{Engine, Event, SESSIONS, State},
    mheard::Heard,
};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
pub const FILES: [&str; 4] = ["welcome.txt", "goodbye.txt", "info.txt", "beacon.txt"];
const DEFAULTS: [&str; 4] = [
    include_str!("../messages/welcome.txt"),
    include_str!("../messages/goodbye.txt"),
    include_str!("../messages/info.txt"),
    include_str!("../messages/beacon.txt"),
];
pub const BYE_WAIT_MS: u64 = 10000;
pub struct Templates {
    pub text: [String; 4],
}
impl Templates {
    pub fn load(config: &Path) -> Result<Self, String> {
        let root = config.parent().unwrap_or(Path::new(".")).join("messages");
        let mut text = std::array::from_fn(|i| DEFAULTS[i].to_owned());
        for (i, name) in FILES.iter().enumerate() {
            match fs::File::open(root.join(name)) {
                Ok(file) => {
                    let mut s = String::new();
                    file.take(4097)
                        .read_to_string(&mut s)
                        .map_err(|e| e.to_string())?;
                    validate(&s, i)?;
                    text[i] = s;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("Template {name}: {e}")),
            }
        }
        Ok(Self { text })
    }
    pub fn save(&mut self, config: &Path, index: usize, text: String) -> Result<(), String> {
        validate(&text, index)?;
        let root = config.parent().unwrap_or(Path::new(".")).join("messages");
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let path = root.join(FILES[index]);
        let temp = path.with_extension("txt.tmp");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        let result = (|| {
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp, &path)
        })();
        if let Err(e) = result {
            let _ = fs::remove_file(temp);
            return Err(e.to_string());
        }
        self.text[index] = text;
        Ok(())
    }
}
pub fn validate(text: &str, index: usize) -> Result<(), String> {
    if text.len() > 4096 {
        return Err("Template exceeds 4096 bytes".into());
    }
    if text
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err("Template contains unsupported control characters".into());
    }
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let end = rest[start..]
            .find('}')
            .ok_or("Unclosed template variable")?
            + start;
        let name = &rest[start + 1..end];
        if !matches!(name, "mycall" | "myname" | "mylocator" | "peercall")
            || index == 3 && name == "peercall"
        {
            return Err(format!("Unsupported template variable: {name}"));
        }
        rest = &rest[end + 1..];
    }
    Ok(())
}
fn render(text: &str, mycall: &str, myname: &str, mylocator: &str, peer: &str) -> Vec<u8> {
    // One pass: values containing braces are never interpreted as templates.
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('}').map(|n| n + start) else {
            break;
        };
        out.push_str(match &rest[start + 1..end] {
            "mycall" => mycall,
            "myname" => myname,
            "mylocator" => mylocator,
            "peercall" => peer,
            _ => "",
        });
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r")
        .into_bytes()
}
#[derive(Default)]
struct Input {
    bytes: Vec<u8>,
    discard: bool,
    last_reply: Option<u64>,
}
pub struct Automation {
    pub templates: Templates,
    name: String,
    locator: String,
    welcome: bool,
    goodbye: bool,
    remote: bool,
    beacon: bool,
    destination: crate::ax25::Call,
    beacon_interfaces: Vec<(String, bool, Vec<crate::ax25::Call>)>,
    interval: u64,
    next_beacon: u64,
    input: [Input; SESSIONS],
    closing: [Option<u64>; SESSIONS],
}
impl Automation {
    /// Refresh preferences without discarding partial commands or pending disconnects.
    pub fn apply_config(&mut self, config: &Config, now: u64) {
        let interval = config.beacon_seconds * 1000;
        if self.interval != interval || self.beacon != config.beacon {
            self.next_beacon = now + interval;
        }
        self.name = config.myname.clone();
        self.locator = config.locator.clone();
        self.welcome = config.welcome;
        self.goodbye = config.goodbye;
        self.remote = config.remote_commands;
        self.beacon = config.beacon;
        self.destination = config.beacon_to.clone();
        self.interval = interval;
        self.beacon_interfaces = config.interfaces.iter().map(|i| {
            (i.name.clone(), i.beacon_enabled, i.beacon_path.clone())
        }).collect();
    }

    pub fn new(config: &Config, path: &Path, now: u64) -> Result<Self, String> {
        Ok(Self {
            templates: Templates::load(path)?,
            name: config.myname.clone(),
            locator: config.locator.clone(),
            welcome: config.welcome,
            goodbye: config.goodbye,
            remote: config.remote_commands,
            beacon: config.beacon,
            destination: config.beacon_to.clone(),
            beacon_interfaces: config
                .interfaces
                .iter()
                .map(|interface| {
                    (
                        interface.name.clone(),
                        interface.beacon_enabled,
                        interface.beacon_path.clone(),
                    )
                })
                .collect(),
            interval: config.beacon_seconds * 1000,
            next_beacon: now + config.beacon_seconds * 1000,
            input: std::array::from_fn(|_| Input::default()),
            closing: [None; SESSIONS],
        })
    }
    pub fn is_closing(&self, id: usize) -> bool {
        self.closing.get(id).is_some_and(Option::is_some)
    }
    fn message(&self, engine: &Engine, id: usize, index: usize) -> Vec<u8> {
        let peer = engine.sessions[id]
            .as_ref()
            .map(|s| s.remote.to_string())
            .unwrap_or_default();
        let mut data = render(
            &self.templates.text[index],
            &engine.local.to_string(),
            &self.name,
            &self.locator,
            &peer,
        );
        if !data.is_empty() && !data.ends_with(b"\r") {
            data.push(b'\r');
        }
        data
    }
    fn send(engine: &mut Engine, id: usize, data: &[u8], now: u64, out: &mut Vec<Event>) {
        if data.is_empty() {
            return;
        }
        match engine.send(id, data, now) {
            Ok(ev) => out.extend(ev),
            Err(e) => out.push(Event::Notice(
                id,
                format!("Automatic message not queued: {e}"),
            )),
        }
    }
    pub fn process(
        &mut self,
        engine: &mut Engine,
        heard: &Heard,
        events: Vec<Event>,
        now: u64,
    ) -> Vec<Event> {
        self.process_with_ports(engine, heard, &[], events, now)
    }
    pub fn process_with_ports(
        &mut self,
        engine: &mut Engine,
        heard: &Heard,
        ports: &[(String, String, bool)],
        events: Vec<Event>,
        now: u64,
    ) -> Vec<Event> {
        let mut extra = vec![];
        for event in &events {
            match event {
                Event::Established(id) => {
                    self.input[*id] = Input::default();
                    self.closing[*id] = None;
                    if self.welcome && engine.sessions[*id].as_ref().is_some_and(|s| s.incoming) {
                        let data = self.message(engine, *id, 0);
                        Self::send(engine, *id, &data, now, &mut extra);
                    }
                }
                Event::Data(id, data)
                    if self.remote
                        && !self.is_closing(*id)
                        && engine.sessions[*id]
                            .as_ref()
                            .is_some_and(|s| s.state == State::Connected) =>
                {
                    for byte in data {
                        if matches!(*byte, b'\r' | b'\n') {
                            let input = &mut self.input[*id];
                            let line = std::mem::take(&mut input.bytes);
                            let discard = std::mem::replace(&mut input.discard, false);
                            if discard
                                || input
                                    .last_reply
                                    .is_some_and(|last| now.saturating_sub(last) < 2000)
                            {
                                continue;
                            }
                            let command =
                                String::from_utf8_lossy(&line).trim().to_ascii_lowercase();
                            let mut words = command.split_whitespace();
                            let verb = words.next().unwrap_or_default();
                            let selector = words.next();
                            let extra_argument = words.next().is_some();
                            let reply = match verb {
                                "/help" | "/h" if selector.is_none() => b"Commands: /help /h - help; /info /i - station info; /mh [PORT|NUMBER] - latest 10 heard stations; /ports (/port) - list interfaces\r".to_vec(),
                                "/info" | "/i" if selector.is_none() => self.message(engine, *id, 2),
                                "/mh" if !extra_argument => {
                                    let interface = selector.and_then(|selector| {
                                        if let Ok(number) = selector.parse::<usize>() {
                                            return number.checked_sub(1).filter(|index| *index < ports.len());
                                        }
                                        let matches = ports
                                            .iter()
                                            .enumerate()
                                            .filter(|(_, (name, _, _))| {
                                                name.eq_ignore_ascii_case(selector)
                                                    || name.chars().take(10).eq(selector.chars())
                                            })
                                            .map(|(index, _)| index)
                                            .collect::<Vec<_>>();
                                        (matches.len() == 1).then(|| matches[0])
                                    });
                                    if selector.is_some() && interface.is_none() {
                                        b"Unknown or ambiguous TNC interface; use /ports\r".to_vec()
                                    } else {
                                        let mut text = match interface {
                                            Some(index) => format!(
                                                "MHeard {} on {} (latest 10)\r",
                                                engine.local,
                                                ports[index].0.chars().take(10).collect::<String>()
                                            ),
                                            None => format!("MHeard {} (latest 10)\r", engine.local),
                                        };
                                        let mut count = 0;
                                        for e in heard.entries().iter().filter(|e| {
                                            interface.is_none_or(|index| e.interface == index)
                                        }).take(10) {
                                            count += 1;
                                            text.push_str(&format!(
                                                "{} {} P:{} {} UTC {}s frames:{}\r",
                                                e.source,
                                                ports.get(e.interface).map_or_else(
                                                    || "?".to_string(),
                                                    |p| p.0.chars().take(10).collect(),
                                                ),
                                                e.port,
                                                e.utc(),
                                                e.age_seconds(now),
                                                e.frames
                                            ));
                                        }
                                        if count == 0 {
                                            text.push_str("No stations heard\r");
                                        }
                                        text.into_bytes()
                                    }
                                }
                                "/ports" | "/port" if selector.is_none() && !ports.is_empty() => {
                                    let mut text = format!("TNC interfaces for {}\r", engine.local);
                                    for (i, (name, description, connected)) in ports.iter().enumerate() {
                                        let short_name = name.chars().take(10).collect::<String>();
                                        text.push_str(&format!(
                                            "{} {}{} {}\r",
                                            i + 1,
                                            short_name,
                                            if description.is_empty() { String::new() } else { format!(" - {description}") },
                                            if *connected { "connected" } else { "reconnecting" }
                                        ));
                                    }
                                    text.into_bytes()
                                }
                                _ => continue,
                            };
                            self.input[*id].last_reply = Some(now);
                            Self::send(engine, *id, &reply, now, &mut extra);
                        } else if !self.input[*id].discard {
                            if self.input[*id].bytes.len() < 256 {
                                self.input[*id].bytes.push(*byte);
                            } else {
                                self.input[*id].bytes.clear();
                                self.input[*id].discard = true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        let mut out = events;
        out.extend(extra);
        out
    }
    pub fn disconnect(
        &mut self,
        engine: &mut Engine,
        id: usize,
        now: u64,
    ) -> Result<Vec<Event>, String> {
        if self.is_closing(id) {
            return Ok(vec![]);
        }
        let session = engine
            .sessions
            .get(id)
            .and_then(Option::as_ref)
            .ok_or("No session")?;
        if self.goodbye && session.incoming && session.state == State::Connected {
            let data = self.message(engine, id, 1);
            if !data.is_empty() {
                let mut out = vec![];
                Self::send(engine, id, &data, now, &mut out);
                self.closing[id] = Some(now + BYE_WAIT_MS);
                out.push(Event::Notice(
                    id,
                    "Sending goodbye before disconnect (up to 10 s)".into(),
                ));
                return Ok(out);
            }
        }
        engine.disconnect(id, now)
    }
    /// Manual transmission uses the same profile without changing the periodic timer.
    fn beacon_info(&self, engine: &Engine) -> Result<Vec<u8>, String> {
        let info = render(
            &self.templates.text[3],
            &engine.local.to_string(),
            &self.name,
            &self.locator,
            "",
        );
        if info.len() > 256 {
            return Err("Beacon skipped: expanded text exceeds 256 bytes".into());
        }
        Ok(info)
    }
    fn beacon_frame(&self, engine: &Engine, path: &[crate::ax25::Call], info: &[u8]) -> Frame {
        Frame {
            source: engine.local.clone(),
            destination: self.destination.clone(),
            path: path.iter().cloned().map(|c| (c, false)).collect(),
            command: true,
            response: false,
            control: 3,
            extended: false,
            pid: Some(240),
            info: info.to_vec(),
        }
    }
    pub fn beacon_events(
        &self,
        engine: &Engine,
        ports: &[(String, String, bool)],
    ) -> Result<Vec<Event>, String> {
        let info = self.beacon_info(engine)?;
        let mut events = Vec::new();
        for (index, (name, _, online)) in ports.iter().enumerate() {
            if !online {
                continue;
            }
            if let Some((_, true, path)) = self
                .beacon_interfaces
                .iter()
                .find(|(configured_name, _, _)| configured_name.eq_ignore_ascii_case(name))
            {
                events.push(Event::TransmitOn(
                    index,
                    self.beacon_frame(engine, path, &info),
                ));
            }
        }
        if events.is_empty() {
            return Err("No beacon-enabled interface is connected".into());
        }
        Ok(events)
    }
    pub fn tick(
        &mut self,
        engine: &mut Engine,
        now: u64,
        online: bool,
        quitting: bool,
    ) -> Vec<Event> {
        let name = self
            .beacon_interfaces
            .first()
            .map(|(name, _, _)| name.clone())
            .unwrap_or_else(|| "default".into());
        self.tick_on_interfaces(engine, now, &[(name, String::new(), online)], quitting)
            .into_iter()
            .map(|event| match event {
                Event::TransmitOn(_, frame) => Event::Transmit(frame),
                event => event,
            })
            .collect()
    }
    pub fn tick_on_interfaces(
        &mut self,
        engine: &mut Engine,
        now: u64,
        ports: &[(String, String, bool)],
        quitting: bool,
    ) -> Vec<Event> {
        let mut out = vec![];
        for id in 0..SESSIONS {
            if engine.sessions[id].is_none() {
                self.closing[id] = None;
                self.input[id] = Input::default();
            }
            if let Some(deadline) = self.closing[id]
                && (engine.sessions[id]
                    .as_ref()
                    .is_some_and(|s| s.buffered() == 0)
                    || now >= deadline)
            {
                self.closing[id] = None;
                if let Ok(ev) = engine.disconnect(id, now) {
                    out.extend(ev);
                }
            }
        }
        if !ports.iter().any(|(_, _, online)| *online) || quitting {
            self.next_beacon = now + self.interval;
        } else if self.beacon && now >= self.next_beacon {
            self.next_beacon = now + self.interval;
            match self.beacon_events(engine, ports) {
                Ok(events) => out.extend(events),
                Err(error) => out.push(Event::Notice(0, error)),
            }
        }
        out
    }
}
