use crate::{
    automation::{Automation, FILES},
    ax25::{Call, Frame},
    config::{Config, InterfaceConfig, TransportKind},
    digipeat::mark_for_repeat,
    link::{Engine, Event, SESSIONS, State},
    mailbox::{MessageType, Protocol, Store},
    mheard::Heard,
    transport::{Connector, Transport},
};
use crate::{
    history::History,
    json::{self, string as js},
    station_events::{Notification, NotificationKind, StationEvent},
    storage, telegram,
    transfers::Transfers,
    web_config::WebConfig,
};
use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Interface {
    config: InterfaceConfig,
    transport: Option<Transport>,
    connector: Connector,
    error: String,
}
pub struct Station {
    runtime: &'static str,
    demo_endpoint: Option<String>,
    config: Config,
    path: PathBuf,
    web_path: PathBuf,
    raw: String,
    engine: Engine,
    automatic: Automation,
    mailbox_engine: Option<Engine>,
    mailbox: Store,
    mail_protocol: Protocol,
    interfaces: Vec<Interface>,
    heard: Heard,
    mheard_path: PathBuf,
    mheard_dirty: bool,
    last_mheard_save: Instant,
    histories: [History; SESSIONS],
    monitor: History,
    statistics: crate::statistics::Statistics,
    transfers: Transfers,
    start: Instant,
    rx: u64,
    tx: u64,
    pub stopping: bool,
    event_sender: Option<std::sync::mpsc::SyncSender<crate::station_events::StationEvent>>,
    telegram_sender: Option<std::sync::mpsc::SyncSender<telegram::Command>>,
    mqtt_discovery_sender: Option<std::sync::mpsc::SyncSender<()>>,
    mailbox_seen: u64,
    presence_mode: String,
    absence_profile: u8,
    heartbeat_timeout: Duration,
    last_heartbeat: Instant,
    last_web_config_check: Instant,
    notifications: VecDeque<Notification>,
    notification_sounds: VecDeque<(u64, u8)>,
    next_notification_id: u64,
}
pub struct Request {
    pub action: String,
    pub fields: HashMap<String, String>,
    pub body: Vec<u8>,
}
pub struct Reply {
    pub mime: &'static str,
    pub body: Vec<u8>,
}

#[cfg(test)]
mod configuration_tests {
    use super::*;

    fn request(action: &str, text: &str) -> Request {
        Request { action: action.into(), fields: HashMap::from([("text".into(), text.into())]), body: vec![] }
    }

    #[test]
    fn notification_sound_is_independent_bounded_and_not_duplicated_on_desktop() {
        let root = crate::test_temp_dir("station-independent-sound");
        fs::create_dir_all(&root).unwrap();
        let mut station = Station::from_text(root.join("station.conf"),
            "callsign=TEST-1\n".into(), None, None, "server").unwrap();
        let (sender, receiver) = std::sync::mpsc::sync_channel(8);
        station.event_sender = Some(sender);
        for (in_app, native) in [(false, false), (true, false), (false, true), (true, true)] {
            station.notifications.clear();
            station.notification_sounds.clear();
            let rule = &mut station.config.notifications.outgoing_connection;
            rule.in_app = in_app;
            rule.native = native;
            rule.telegram = false;
            rule.sound = 2;
            station.notify(NotificationKind::OutgoingConnection, "Connected", "PEER");
            assert_eq!(station.notifications.len(), usize::from(in_app));
            assert_eq!(station.notification_sounds.len(), 1);
            assert_eq!(station.notification_sounds[0].1, 2);
            let events: Vec<_> = receiver.try_iter().collect();
            assert_eq!(events.len(), 1);
            match &events[0] {
                StationEvent::Notification(n) => { assert!(native); assert_eq!(n.sound, 2); }
                StationEvent::Sound(count) => { assert!(!native); assert_eq!(*count, 2); }
            }
        }
        station.event_sender = None;
        station.notifications.clear();
        let rule = &mut station.config.notifications.outgoing_connection;
        rule.in_app = false;
        rule.native = false;
        rule.sound = 0;
        let before = station.notification_sounds.len();
        station.notify(NotificationKind::OutgoingConnection, "Silent", "");
        assert_eq!(station.notification_sounds.len(), before);
        station.config.notifications.outgoing_connection.sound = 1;
        for _ in 0..150 {
            station.notify(NotificationKind::OutgoingConnection, "Connected", "PEER");
        }
        assert_eq!(station.notification_sounds.len(), 100);
        assert!(station.notifications.is_empty());
        assert!(station.snapshot().contains("\"notificationSounds\":[{\"id\":"));
        station.config.notifications.yapp.sound = 3;
        let last = *station.notification_sounds.back().unwrap();
        station.notify(NotificationKind::Yapp, "Disabled module", "");
        assert_eq!(*station.notification_sounds.back().unwrap(), last);
        station.stopping = true;
        station.notify(NotificationKind::OutgoingConnection, "Stopping", "");
        assert_eq!(*station.notification_sounds.back().unwrap(), last);
        drop(station);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn live_preferences_preserve_session_transport_and_station_state() {
        let root = crate::test_temp_dir("station-live-settings");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("station.conf");
        let text = "callsign=TEST-1\ninterface_VHF=127.0.0.1:8001,0,0,true,Radio\nax25_autonegotiation=false\nax25_modulo_128=false\n";
        let mut station = Station::from_text(path.clone(), text.into(), None, None, "server").unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        station.interfaces[0].transport = Some(Transport::connect(&listener.local_addr().unwrap().to_string()).unwrap());
        let (_peer, _) = listener.accept().unwrap();
        let (_, events) = station.engine.connect_selected_on(0, "TEST-2".parse().unwrap(), vec![], 0, 0).unwrap();
        let sabm = events.into_iter().find_map(|event| match event {
            Event::TransmitOn(_, frame) | Event::Transmit(frame) => Some(frame), _ => None,
        }).unwrap();
        let mut peer = Engine::new("TEST-2".parse().unwrap());
        let ua = peer.receive(sabm, 1).into_iter().find_map(|event| match event {
            Event::Transmit(frame) | Event::TransmitOn(_, frame) => Some(frame), _ => None,
        }).unwrap();
        station.engine.receive(ua, 2);
        assert_eq!(station.engine.sessions[0].as_ref().unwrap().state, State::Connected);
        let started = station.start;
        station.rx = 17;
        station.tx = 23;
        let mut next = Config::parse(text).unwrap();
        next.notifications.terminal_message.in_app = false;
        next.notifications.mheard_new_station.in_app = true;
        next.myname = "Operator".into();
        next.beacon_seconds = 1200;
        next.interfaces[0].description = "New description".into();
        next.interfaces[0].priority = 9;
        next.yapp_max_files = 15;
        let updated = crate::config::encode(&next);
        let impact = station.command(request("config/check", &updated)).unwrap();
        assert!(String::from_utf8(impact.body).unwrap().contains("\"requiresDisconnect\":false"));
        station.command(request("config/save", &updated)).unwrap();
        assert_eq!(station.engine.sessions[0].as_ref().unwrap().state, State::Connected);
        assert!(station.interfaces[0].transport.is_some());
        assert_eq!(station.start, started);
        assert_eq!((station.rx, station.tx), (17, 23));
        assert_eq!(station.transfers.max_files, 15);
        assert_eq!(station.config.myname, "Operator");
        assert!(!station.config.notifications.terminal_message.in_app);
        assert!(station.config.notifications.mheard_new_station.in_app);
        assert!(!station.notifications.iter().any(|n| n.kind == NotificationKind::MheardNewStation));
        assert!(fs::read_to_string(&path).unwrap().contains("myname=Operator"));
        next.call = Some("OTHER-1".parse().unwrap());
        let structural = crate::config::encode(&next);
        assert!(station.command(request("config/save", &structural)).is_err());
        assert_eq!(station.engine.local.to_string(), "TEST-1");
        assert_eq!(fs::read_to_string(&path).unwrap(), updated);
        drop(station);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retired_module_has_no_runtime_api_and_keeps_existing_files() {
        let root = crate::test_temp_dir("station-retired-module");
        fs::create_dir_all(root.join("wl2k")).unwrap();
        let legacy_data = root.join("wl2k/preserved.dat");
        let legacy_secret = root.join("wl2k.password");
        fs::write(&legacy_data, b"existing mail").unwrap();
        fs::write(&legacy_secret, b"existing secret").unwrap();
        let path = root.join("station.conf");
        let text = "callsign=TEST-1\nmailbox_enabled=true\nmailbox_callsign=TEST-2\nwl2k_enabled=true\nwl2k_gateway=RMS-1\nnotification_winlink_in-app=true\n";
        fs::write(&path, text).unwrap();
        let mut station = Station::from_text(path.clone(), text.into(), None, None, "server").unwrap();
        assert!(station.mailbox_engine.is_some());
        for action in ["wl2k/sync", "wl2k/create", "wl2k/read", "wl2k/file", "wl2k/password", "wl2k/p2p/start", "wl2k/grib", "wl2k/disconnect", "wl2k/attachment", "wl2k/mark", "wl2k/queue"] {
            assert!(station.command(request(action, "")).is_err(), "{action}");
        }
        station.command(request("config/save", text)).unwrap();
        let details = String::from_utf8(station.command(request("config/details", "")).unwrap().body).unwrap();
        for body in [details, station.snapshot()] {
            assert!(!body.contains("wl2k"));
            assert!(!body.contains("winlink"));
            assert!(!body.contains("p2p"));
        }
        let saved = fs::read_to_string(&path).unwrap();
        assert!(!saved.contains("wl2k"));
        assert!(!saved.contains("winlink"));
        assert!(station.mailbox_engine.is_some());
        assert_eq!(fs::read(&legacy_data).unwrap(), b"existing mail");
        assert_eq!(fs::read(&legacy_secret).unwrap(), b"existing secret");
        assert!(!root.join("wl2k/messages").exists());
        assert!(!root.join("wl2k/attachments").exists());
        assert_eq!(fs::read_to_string(root.join("station.bak")).unwrap(), text);
        drop(station);
        fs::remove_dir_all(root).unwrap();
    }
}
impl Reply {
    pub fn json(s: String) -> Self {
        Self {
            mime: "application/json; charset=utf-8",
            body: s.into_bytes(),
        }
    }
    pub fn text(s: String) -> Self {
        Self {
            mime: "text/plain; charset=utf-8",
            body: s.into_bytes(),
        }
    }
    fn ok() -> Self {
        Self::json("{\"ok\":true}".into())
    }
}
fn engine(config: &Config, mail: bool) -> Engine {
    let call = if mail {
        config.mailbox_call.clone()
    } else {
        config.call.clone()
    };
    let mut e = Engine::new(call.unwrap_or_else(|| "NOCALL".parse().unwrap()));
    e.t1 = config.t1;
    e.t3 = config.t3;
    e.n2 = config.n2;
    e.paclen = config.paclen;
    e.accept = if mail {
        config.mailbox_enabled
    } else {
        config.accept
    };
    e.autonegotiation = config.ax25_autonegotiation;
    e.modulo_128 = config.ax25_modulo_128;
    if mail {
        e.incoming_limit = 2;
    }
    e
}
impl Station {
    /// Select the AX.25 control-field width from the engine that owns the
    /// connected session.  The terminal and MailBox engines may share
    /// the station callsign, so destination matching alone is ambiguous.
    /// A new SABM/SABME has no session yet and is deliberately decoded in
    /// modulo 8; after the link is created the negotiated mode is retained.
    fn extended_mode_for_incoming(&self, dest: &Call, source: &Call) -> bool {
        if let Some(engine) = &self.mailbox_engine
            && &engine.local == dest
            && engine
                .sessions
                .iter()
                .flatten()
                .any(|session| &session.remote == source)
        {
            return engine.extended_for(source);
        }
        self.engine.extended_for(source)
    }

    pub fn set_events(
        &mut self,
        sender: Option<std::sync::mpsc::SyncSender<crate::station_events::StationEvent>>,
    ) {
        self.event_sender = sender;
        self.mailbox_seen = self
            .mailbox
            .messages()
            .iter()
            .map(|m| m.id)
            .max()
            .unwrap_or(0);
    }
    pub fn set_telegram_sender(
        &mut self,
        sender: Option<std::sync::mpsc::SyncSender<telegram::Command>>,
    ) {
        self.telegram_sender = sender;
    }
    pub fn notify_application_started(&mut self) {
        self.notify(
            NotificationKind::ApplicationStarted,
            "Uruchomienie aplikacji",
            if self.runtime == "desktop" {
                "UPR Terminal jest gotowy do pracy"
            } else {
                "UPR Station jest gotowa do pracy"
            },
        );
    }
    pub fn set_mqtt_discovery_sender(&mut self, sender: Option<std::sync::mpsc::SyncSender<()>>) {
        self.mqtt_discovery_sender = sender;
    }
    fn telegram_token_path(&self) -> PathBuf {
        let configured = PathBuf::from(self.config.telegram_token_file.trim());
        if configured.is_absolute() {
            configured
        } else {
            self.path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join(configured)
        }
    }
    fn notify(
        &mut self,
        kind: NotificationKind,
        title: impl Into<String>,
        body: impl Into<String>,
    ) {
        if self.stopping {
            return;
        }
        let module_enabled = match kind {
            NotificationKind::Mailbox => self.config.mailbox_enabled,
            NotificationKind::Yapp => self.config.yapp_enabled,
            _ => true,
        };
        if !module_enabled {
            return;
        }
        let rule = self.config.notifications.rule(kind).clone();
        if !rule.in_app
            && !rule.native
            && rule.sound == 0
            && !(rule.telegram
                && self.config.telegram_enabled
                && !self.config.telegram_chat_id.is_empty())
        {
            return;
        }
        let title = title.into();
        let body = body.into();
        let notification = Notification {
            id: self.next_notification_id,
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            kind,
            title: title.clone(),
            body: body.clone(),
            sound: rule.sound,
        };
        self.next_notification_id = self.next_notification_id.saturating_add(1);
        if rule.sound > 0 {
            self.notification_sounds.push_back((notification.id, rule.sound));
            while self.notification_sounds.len() > 100 {
                self.notification_sounds.pop_front();
            }
        }
        if rule.in_app {
            self.notifications.push_back(notification.clone());
            while self.notifications.len() > 100 {
                self.notifications.pop_front();
            }
        }
        if rule.native
            && let Some(sender) = &self.event_sender
        {
            let _ = sender.try_send(StationEvent::Notification(notification));
        } else if rule.sound > 0
            && let Some(sender) = &self.event_sender
        {
            let _ = sender.try_send(StationEvent::Sound(rule.sound));
        }
        if rule.telegram
            && self.config.telegram_enabled
            && !self.config.telegram_chat_id.is_empty()
            && let Some(sender) = &self.telegram_sender
        {
            let text = format!("{title}\n{body}");
            let _ = sender.try_send(telegram::Command::Send(telegram::MessageJob {
                token_path: self.telegram_token_path(),
                chat_id: self.config.telegram_chat_id.clone(),
                text,
            }));
        }
    }
    pub fn notify_mail(&mut self) {
        if self.event_sender.is_none() && self.telegram_sender.is_none() {
            return;
        }
        let mailbox_events = self
            .mailbox
            .messages()
            .iter()
            .filter_map(|m| {
                if m.id > self.mailbox_seen && self.config.call.as_ref() != Some(&m.from) {
                    Some((m.id, m.from.to_string(), m.subject.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for m in self.mailbox.messages() {
            self.mailbox_seen = self.mailbox_seen.max(m.id);
        }
        for (_, from, subject) in mailbox_events {
            self.notify(
                NotificationKind::Mailbox,
                "Nowa wiadomość MailBox",
                format!("Od: {from} · {subject}"),
            );
        }
    }

    pub fn new_with_web_config(
        path: PathBuf,
        demo_endpoint: Option<String>,
        web_path: Option<PathBuf>,
        runtime: &'static str,
    ) -> Result<Self, String> {
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        Self::from_text(path, raw, demo_endpoint, web_path, runtime)
    }
    fn from_text(
        path: PathBuf,
        raw: String,
        demo_endpoint: Option<String>,
        web_path: Option<PathBuf>,
        runtime: &'static str,
    ) -> Result<Self, String> {
        let config = Config::parse(&raw)?;
        let web_settings = web_path
            .as_ref()
            .map(|path| WebConfig::load(path))
            .transpose()?
            .unwrap_or_default();
        if let Some(endpoint) = &demo_endpoint
            && (config.interfaces.len() != 1
                || config.interfaces[0].endpoint != *endpoint
                || config.interfaces[0].transport != crate::config::TransportKind::Tcp)
        {
            return Err("Demo mode only permits its built-in loopback TNC".into());
        }
        let mut mailbox = Store::open(&path)?;
        let warnings = mailbox.take_warnings();
        let directory = if config.yapp_download_dir.trim().is_empty() {
            path.parent().unwrap().join("files")
        } else {
            let configured = PathBuf::from(config.yapp_download_dir.trim());
            let base = if configured.is_absolute() {
                configured
            } else {
                path.parent().unwrap().join(configured)
            };
            if base
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("UPR YAPP"))
            {
                base
            } else {
                base.join("UPR YAPP")
            }
        };
        let yapp_max_files = config.yapp_max_files;
        let yapp_auto_cleanup = config.yapp_auto_cleanup;
        let mheard_path = path.clone().with_file_name("mheard.dat");
        let heard = Heard::load(&mheard_path, Heard::current_utc(), config.interfaces.len());
        let mut s = Self {
            runtime,
            demo_endpoint,
            engine: engine(&config, false),
            mailbox_engine: config.mailbox_enabled.then(|| engine(&config, true)),
            automatic: Automation::new(&config, &path, 0)?,
            interfaces: config
                .interfaces
                .iter()
                .map(|c| Interface {
                    config: c.clone(),
                    transport: None,
                    connector: Connector::default(),
                    error: String::new(),
                })
                .collect(),
            config,
            web_path: web_path.unwrap_or_else(|| path.with_file_name("web.conf")),
            path: path.clone(),
            raw,
            mailbox,
            mail_protocol: Protocol::default(),
            heard,
            mheard_path,
            mheard_dirty: false,
            last_mheard_save: Instant::now(),
            histories: std::array::from_fn(|_| History::default()),
            monitor: History::default(),
            statistics: crate::statistics::Statistics::load_with_retention(
                path.clone().with_file_name("statistics.json"),
                web_settings.statistics_enabled,
                web_settings.statistics_retention_days,
            ),
            transfers: Transfers::new(directory, yapp_max_files, yapp_auto_cleanup),
            start: Instant::now(),
            rx: 0,
            tx: 0,
            stopping: false,
            event_sender: None,
            telegram_sender: None,
            mqtt_discovery_sender: None,
            mailbox_seen: 0,
            presence_mode: web_settings.presence_mode,
            absence_profile: web_settings.absence_profile,
            heartbeat_timeout: Duration::from_secs(web_settings.heartbeat_timeout),
            // A freshly booted system may have a monotonic uptime shorter than
            // the configured timeout.  Do not subtract past Instant's origin.
            last_heartbeat: Instant::now()
                .checked_sub(Duration::from_secs(web_settings.heartbeat_timeout))
                .unwrap_or_else(Instant::now),
            last_web_config_check: Instant::now(),
            notifications: VecDeque::new(),
            notification_sounds: VecDeque::new(),
            next_notification_id: 1,
        };
        for w in warnings {
            s.log("error", &w);
        }
        s.log("info", "Station started");
        Ok(s)
    }
    fn now(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
    fn save_mheard_if_due(&mut self, force: bool) {
        let pruned = self
            .heard
            .prune(Heard::current_utc(), self.interfaces.len());
        self.mheard_dirty |= pruned;
        if !self.mheard_dirty
            || (!force && self.last_mheard_save.elapsed() < Duration::from_secs(5))
        {
            return;
        }
        match self.heard.save(&self.mheard_path) {
            Ok(()) => {
                self.mheard_dirty = false;
                self.last_mheard_save = Instant::now();
            }
            Err(error) => self.log(
                "error",
                &format!("Nie można zapisać pamięci MHeard: {error}"),
            ),
        }
    }
    pub fn files_directory(&self) -> PathBuf {
        self.transfers.directory.clone()
    }
    fn ports(&self) -> Vec<(String, String, bool)> {
        self.interfaces
            .iter()
            .map(|i| {
                (
                    i.config.name.clone(),
                    i.config.description.clone(),
                    i.transport.is_some(),
                )
            })
            .collect()
    }
    fn operator_available(&self) -> bool {
        match self.presence_mode.as_str() {
            "always" => true,
            "dnd" => false,
            _ => self.last_heartbeat.elapsed() <= self.heartbeat_timeout,
        }
    }
    fn incoming_profile(&self) -> u8 {
        if self.operator_available() {
            1
        } else {
            self.absence_profile.clamp(1, 4)
        }
    }
    fn log(&mut self, kind: &str, text: &str) {
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            % 86400;
        self.monitor.note(
            kind,
            &format!("{:02}:{:02}:{:02} {text}", t / 3600, t / 60 % 60, t % 60),
        );
    }
    fn lost(&mut self, id: usize, error: String) {
        let retry = self.now() + 1000;
        if self.interfaces[id].transport.is_some() {
            let name = self.interfaces[id].config.name.clone();
            self.notify(
                NotificationKind::TncDisconnected,
                "Utrata połączenia z TNC",
                format!("Interfejs {name} · {error}"),
            );
        }
        self.interfaces[id].transport = None;
        self.interfaces[id].error = error.clone();
        self.interfaces[id].connector.reset(retry);
        self.log(
            "error",
            &format!("TNC {}: {error}", self.interfaces[id].config.name),
        );
        let ev = self.engine.interface_lost(id);
        self.events(ev, id, false);
        if let Some(e) = &mut self.mailbox_engine {
            let ev = e.interface_lost(id);
            self.events(ev, id, true);
        }
    }
    fn transmit(&mut self, route: usize, frame: Frame) {
        if let Some(i) = self.interfaces.get_mut(route)
            && let Some(t) = &mut i.transport
        {
            match t.send(i.config.port, &frame.encode()) {
                Ok(()) => {
                    self.tx += 1;
                    let name = self.interfaces[route].config.name.clone();
                    self.statistics.tx(&name);
                    self.log(
                        "tx",
                        &format!(
                            "[{}] {}",
                            self.interfaces[route].config.name,
                            frame.summary()
                        ),
                    );
                }
                Err(e) => self.lost(route, e.to_string()),
            }
        }
    }
    fn events(&mut self, events: Vec<Event>, hint: usize, mail: bool) {
        let now = self.now();
        let ports = self.ports();
        let events = if mail {
            events
        } else {
            // YAPP data must never trigger automatic text commands.
            let (binary, text): (Vec<_>, Vec<_>) = events.into_iter().partition(
                |event| matches!(event,Event::Data(id,bytes) if self.transfers.handles(*id,bytes)),
            );
            let mut processed =
                self.automatic
                    .process_with_ports(&mut self.engine, &self.heard, &ports, text, now);
            processed.extend(binary);
            processed
        };
        let mut pending = VecDeque::from(events);
        while let Some(event) = pending.pop_front() {
            if let Event::Established(id) = &event {
                self.statistics.session();
                let e = if mail {
                    self.mailbox_engine.as_ref().unwrap()
                } else {
                    &self.engine
                };
                let connection =
                    (!mail)
                        .then(|| e.sessions[*id].as_ref())
                        .flatten()
                        .map(|session| {
                            (
                                session.incoming,
                                session.remote.to_string(),
                                self.interfaces
                                    .get(session.interface)
                                    .map(|item| item.config.name.clone())
                                    .unwrap_or_else(|| "TNC".into()),
                            )
                        });
                if let Some((incoming, call, interface)) = connection {
                    self.notify(
                        if incoming {
                            NotificationKind::IncomingConnection
                        } else {
                            NotificationKind::OutgoingConnection
                        },
                        if incoming {
                            "Nowe połączenie AX.25"
                        } else {
                            "Połączono przez AX.25"
                        },
                        format!("{call} przez {interface}"),
                    );
                }
            }
            match event {
                Event::Transmit(f) => {
                    let e = if mail {
                        self.mailbox_engine.as_ref().unwrap()
                    } else {
                        &self.engine
                    };
                    let route = e
                        .sessions
                        .iter()
                        .flatten()
                        .find(|s| s.remote == f.destination)
                        .map_or(hint, |s| s.interface);
                    self.transmit(route, f);
                }
                Event::TransmitOn(route, f) => self.transmit(route, f),
                Event::Established(id) if mail => {
                    let e = self.mailbox_engine.as_mut().unwrap();
                    let greeting = self.mail_protocol.connected(id, &e.local);
                    if let Ok(ev) = e.send(id, &greeting, now) {
                        pending.extend(ev);
                    }
                }
                Event::Established(id) => {
                    let profile = self.incoming_profile();
                    if !mail
                        && profile != 1
                        && self.engine.sessions[id]
                            .as_ref()
                            .is_some_and(|s| s.incoming)
                    {
                        let message = match profile {
                            2 => "Operator jest nieobecny. Dostępne komendy: /mh, /info, /ports.\r",
                            _ => "Operator jest obecnie niedostępny.\r",
                        };
                        if let Ok(ev) = self.engine.send(id, message.as_bytes(), now) {
                            pending.extend(ev);
                        }
                        // Profile 2 keeps the session open so the existing
                        // /mh, /info and /ports handlers remain available.
                        // Profile 3 announces the absence and disconnects.
                        if profile == 3 {
                            if let Ok(ev) = self.automatic.disconnect(&mut self.engine, id, now) {
                                pending.extend(ev);
                            }
                        }
                    }
                }
                Event::Data(id, bytes) if mail => {
                    let e = self.mailbox_engine.as_mut().unwrap();
                    if let Some(s) = &e.sessions[id] {
                        let remote = s.remote.clone();
                        for reply in self
                            .mail_protocol
                            .data(id, &remote, &bytes, &mut self.mailbox)
                        {
                            match e.send(id, &reply, now) {
                                Ok(ev) => pending.extend(ev),
                                Err(err) => self.monitor.note("error", &err),
                            }
                        }
                    }
                    if self.mail_protocol.take_disconnect(id)
                        && let Ok(ev) = e.disconnect(id, now)
                    {
                        pending.extend(ev);
                    }
                }
                Event::Data(id, bytes) => {
                    let (text, ev) = self.transfers.receive(
                        id,
                        &bytes,
                        self.config.yapp_enabled,
                        self.config.yapp_auto_receive,
                        &mut self.engine,
                        now,
                    );
                    self.histories[id].data(&text);
                    let message = String::from_utf8_lossy(&text);
                    if !message.trim().is_empty()
                        && let Some(session) = self.engine.sessions.get(id).and_then(Option::as_ref)
                    {
                        let interface = self
                            .interfaces
                            .get(session.interface)
                            .map(|item| item.config.name.clone())
                            .unwrap_or_else(|| "TNC".into());
                        let body = message.trim().chars().take(240).collect::<String>();
                        self.notify(
                            NotificationKind::TerminalMessage,
                            "Nowa wiadomość w terminalu",
                            format!("{} przez {}: {}", session.remote, interface, body),
                        );
                    }
                    pending.extend(ev);
                }
                Event::Status(id, text) | Event::Notice(id, text) => {
                    if mail {
                        self.log("info", &format!("MailBox {}: {text}", id + 1));
                        if self.mailbox_engine.as_ref().unwrap().sessions[id].is_none() {
                            self.mail_protocol.disconnected(id);
                        }
                    } else {
                        if text.to_ascii_lowercase().contains("disconnect") {
                            self.notify(
                                NotificationKind::Disconnection,
                                "Rozłączenie AX.25",
                                text.clone(),
                            );
                        }
                        self.histories[id].note("info", &text);
                        if self.engine.sessions[id].is_none() {
                            self.transfers.clear(id);
                        }
                    }
                }
            }
        }
    }
    fn telegram_token_present(&self) -> bool {
        storage::read_secret(&self.telegram_token_path()).is_ok_and(|bytes| !bytes.is_empty())
    }
    fn mqtt_password_path(&self) -> PathBuf {
        self.web_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("mqtt.password")
    }
    fn mqtt_password_present(&self) -> bool {
        storage::read_secret(&self.mqtt_password_path()).is_ok_and(|bytes| !bytes.is_empty())
    }
    pub fn tick(&mut self) {
        let now = self.now();
        self.save_mheard_if_due(false);
        if self.last_web_config_check.elapsed() >= Duration::from_secs(1) {
            if let Ok(settings) = WebConfig::load(&self.web_path) {
                self.presence_mode = settings.presence_mode;
                self.absence_profile = settings.absence_profile;
                self.heartbeat_timeout = Duration::from_secs(settings.heartbeat_timeout);
                if self.statistics.enabled != settings.statistics_enabled {
                    self.statistics.set_enabled(settings.statistics_enabled);
                }
                if self.statistics.retention_days() != settings.statistics_retention_days {
                    self.statistics
                        .set_retention_days(settings.statistics_retention_days);
                }
            }
            self.last_web_config_check = Instant::now();
        }
        self.engine.accept = self.config.accept && self.incoming_profile() != 4;
        let ports = self.ports();
        let ev = self
            .automatic
            .tick_on_interfaces(&mut self.engine, now, &ports, self.stopping);
        let beacon=ev.iter().any(|e|matches!(e,Event::TransmitOn(_,f) if f.control==3 && f.destination==self.config.beacon_to));
        self.events(ev, 0, false);
        if beacon
            && let Some(e) = &self.mailbox_engine
            && let Ok(ev) = self.automatic.beacon_events(e, &ports)
        {
            self.events(ev, 0, true);
        }
        for id in 0..self.interfaces.len() {
            let i = &mut self.interfaces[id];
            if !self.stopping
                && self.config.call.is_some()
                && i.config.enabled
                && i.transport.is_none()
                && let Some(result) = i.connector.poll(
                    &i.config.endpoint,
                    i.config.transport.clone(),
                    i.config.baud,
                    now,
                )
            {
                match result {
                    Ok(t) => {
                        i.transport = Some(t);
                        i.error.clear();
                        let name = self.interfaces[id].config.name.clone();
                        self.log("info", &format!("TNC {name} connected"));
                        self.notify(
                            NotificationKind::TncConnected,
                            "Połączono z TNC",
                            format!("Interfejs {name}"),
                        );
                    }
                    Err(e) => {
                        i.error = e.to_string();
                    }
                }
            }
            let frames = match &mut self.interfaces[id].transport {
                Some(t) => match t.poll() {
                    Ok(f) => f,
                    Err(e) => {
                        self.lost(id, e.to_string());
                        vec![]
                    }
                },
                None => vec![],
            };
            for (kind, bytes) in frames {
                if kind & 15 != 0 || kind >> 4 != self.interfaces[id].config.port {
                    continue;
                }
                let decoded = (|| {
                    let source = Frame::peek_source(&bytes)?;
                    let dest = Frame::peek_destination(&bytes)?;
                    let ext = self.extended_mode_for_incoming(&dest, &source);
                    Frame::decode_with_mode(&bytes, ext)
                })();
                match decoded {
                    Ok(f) => {
                        self.rx += 1;
                        let name = self.interfaces[id].config.name.clone();
                        let route = f
                            .path
                            .iter()
                            .map(|(call, repeated)| {
                                format!("{call}{}", if *repeated { "*" } else { "" })
                            })
                            .collect::<Vec<_>>()
                            .join(" ");
                        self.statistics.rx(&f.source.to_string(), &name, &route);
                        self.log(
                            "rx",
                            &format!(
                                "[{}] {} {}",
                                self.interfaces[id].config.name,
                                f.summary(),
                                String::from_utf8_lossy(&f.info)
                                    .replace('\r', "<CR>")
                                    .replace('\n', "<LF>")
                            ),
                        );
                        if !self
                            .mailbox_engine
                            .as_ref()
                            .is_some_and(|e| e.local == f.source)
                        {
                            let new_station = self.heard.record_on(
                                &f,
                                self.interfaces[id].config.port,
                                id,
                                &self.engine.local,
                                now,
                            );
                            self.mheard_dirty = true;
                            if new_station {
                                self.notify(
                                    NotificationKind::MheardNewStation,
                                    "MHeard",
                                    format!("Odebrano nową stacje: {}", f.source),
                                );
                            }
                        }
                        if self.config.digipeat {
                            let mut repeated = f.clone();
                            if mark_for_repeat(&mut repeated, &self.engine.local) {
                                self.transmit(id, repeated);
                            }
                        }
                        let mail = self
                            .mailbox_engine
                            .as_ref()
                            .is_some_and(|e| e.local == f.destination);
                        let ev = if mail {
                            self.mailbox_engine.as_mut().unwrap().receive_on(f, now, id)
                        } else {
                            self.engine.receive_on(f, now, id)
                        };
                        self.events(ev, id, mail);
                    }
                    Err(e) => self.log("error", e),
                }
            }
        }
        let ev = self.engine.tick(now);
        self.events(ev, 0, false);
        if let Some(e) = &mut self.mailbox_engine {
            let ev = e.tick(now);
            self.events(ev, 0, true);
        }
        let ev = self.transfers.tick(&mut self.engine, now);
        self.events(ev, 0, false);
        if std::mem::take(&mut self.transfers.received) > 0 {
            self.notify(
                NotificationKind::Yapp,
                "Odebrano plik YAPP",
                "Transfer pliku został zakończony.",
            );
        }
        for (id, note) in std::mem::take(&mut self.transfers.notices) {
            self.histories[id].note("info", &note);
        }
    }
    fn configuration_requires_rebuild(&self, next: &Config) -> bool {
        let current = &self.config;
        current.call != next.call
            || current.t1 != next.t1
            || current.t3 != next.t3
            || current.n2 != next.n2
            || current.paclen != next.paclen
            || current.ax25_autonegotiation != next.ax25_autonegotiation
            || current.ax25_modulo_128 != next.ax25_modulo_128
            || current.mailbox_enabled != next.mailbox_enabled
            || current.mailbox_call != next.mailbox_call
            || current.yapp_enabled != next.yapp_enabled
            || current.yapp_download_dir != next.yapp_download_dir
            || current.interfaces.len() != next.interfaces.len()
            || current.interfaces.iter().zip(&next.interfaces).any(|(a, b)| {
                a.name != b.name || a.endpoint != b.endpoint || a.transport != b.transport
                    || a.baud != b.baud || a.port != b.port || a.enabled != b.enabled
            })
    }

    pub fn active(&self) -> bool {
        self.engine.sessions.iter().any(Option::is_some)
            || self
                .mailbox_engine
                .as_ref()
                .is_some_and(|e| e.sessions.iter().any(Option::is_some))

    }
    pub fn shutdown(&mut self) {
        self.stopping = true;
        self.save_mheard_if_due(true);
        self.statistics.flush();
        let now = self.now();
        for id in 0..SESSIONS {
            // Shutdown is a transport-level event: send AX.25 DISC to every
            // live session immediately. Goodbye templates must not delay the
            // disconnect handshake while the process is leaving.
            if self.engine.sessions[id].is_some()
                && let Ok(ev) = self.engine.disconnect(id, now)
            {
                self.events(ev, 0, false);
            }
            if let Some(e) = &mut self.mailbox_engine
                && e.sessions[id].is_some()
                && let Ok(ev) = e.disconnect(id, now)
            {
                self.events(ev, 0, true);
            }
        }
    }
    pub fn command(&mut self, r: Request) -> Result<Reply, String> {
        let get = |k: &str| r.fields.get(k).map(String::as_str).unwrap_or("");
        let id = || -> Result<usize, String> {
            let id = get("session")
                .parse::<usize>()
                .map_err(|_| "Invalid session")?;
            if id >= SESSIONS {
                return Err("Invalid session".into());
            }
            Ok(id)
        };
        let now = self.now();
        if self.stopping {
            return Err("Station is shutting down".into());
        }
        match r.action.as_str() {
            "connect" => {
                if self.config.call.is_none() {
                    return Err("Configure your callsign first".into());
                }
                let route = get("interface")
                    .parse::<usize>()
                    .map_err(|_| "Invalid interface")?;
                if !self
                    .interfaces
                    .get(route)
                    .is_some_and(|i| i.transport.is_some())
                {
                    return Err("TNC interface is offline".into());
                }
                let path = get("via")
                    .split_whitespace()
                    .map(str::parse)
                    .collect::<Result<Vec<_>, _>>()?;
                let (selected, ev) = self.engine.connect_selected_on(
                    id()?,
                    get("call").parse()?,
                    path,
                    now,
                    route,
                )?;
                self.events(ev, route, false);
                return Ok(Reply::json(format!("{{\"session\":{selected}}}")));
            }
            "disconnect-all" => {
                for session in 0..SESSIONS {
                    if self.engine.sessions[session].is_some() {
                        let ev = self.automatic.disconnect(&mut self.engine, session, now)?;
                        self.events(ev, 0, false);
                    }
                    if let Some(engine) = &mut self.mailbox_engine
                        && engine.sessions[session].is_some()
                    {
                        let ev = engine.disconnect(session, now)?;
                        self.events(ev, 0, true);
                    }
                }
            }
            "disconnect" => {
                let ev = self.automatic.disconnect(&mut self.engine, id()?, now)?;
                self.events(ev, 0, false);
            }
            "send" => {
                let id = id()?;
                if self.transfers.busy(id) {
                    return Err("A YAPP transfer is active".into());
                }
                let text = get("text");
                if text.is_empty() || text.len() > 4096 {
                    return Err("Message must contain 1â€“4096 UTF-8 bytes".into());
                }
                if !self.engine.sessions[id]
                    .as_ref()
                    .is_some_and(|s| s.state == State::Connected)
                {
                    return Err("Session is not connected".into());
                }
                let mut bytes = text.as_bytes().to_vec();
                bytes.push(13);
                let ev = self.engine.send(id, &bytes, now)?;
                self.histories[id].note("tx", text);
                self.events(ev, 0, false);
            }
            "broadcast" => {
                let route = get("interface")
                    .parse::<usize>()
                    .map_err(|_| "Invalid interface")?;
                if !self
                    .interfaces
                    .get(route)
                    .is_some_and(|i| i.transport.is_some())
                {
                    return Err("TNC interface is offline".into());
                }
                let source = self
                    .config
                    .call
                    .clone()
                    .ok_or("Configure your callsign first")?;
                let destination = if get("destination").is_empty() {
                    "CQ".parse()?
                } else {
                    get("destination").parse()?
                };
                let path = get("via")
                    .split_whitespace()
                    .map(str::parse)
                    .collect::<Result<Vec<_>, _>>()?;
                if path.len() > 8 {
                    return Err("Broadcast path allows at most 8 digipeaters".into());
                }
                let text = get("text");
                if text.is_empty() || text.len() > 256 {
                    return Err("Broadcast message must contain 1-256 UTF-8 bytes".into());
                }
                self.transmit(
                    route,
                    Frame {
                        source,
                        destination,
                        path: path.into_iter().map(|call| (call, false)).collect(),
                        command: true,
                        response: false,
                        control: 3,
                        extended: false,
                        pid: Some(240),
                        info: text.as_bytes().to_vec(),
                    },
                );
            }
            "clear" => self.histories[id()?].clear(),
            "beacon" => {
                if self.config.call.is_none() {
                    return Err("Configure your callsign first".into());
                }
                let ports = self.ports();
                let ev = self.automatic.beacon_events(&self.engine, &ports)?;
                self.events(ev, 0, false);
                if let Some(e) = &self.mailbox_engine {
                    let ev = self.automatic.beacon_events(e, &ports)?;
                    self.events(ev, 0, true);
                }
            }
            "retry" => {
                for i in &mut self.interfaces {
                    i.connector.retry(now);
                }
            }
            "config" => return Ok(Reply::text(self.raw.clone())),
            "stats" => return Ok(Reply::json(self.statistics.snapshot())),
            "update/status" => {
                let root = crate::update::directory(&self.path);
                return Ok(Reply::json(crate::update::status_json(&root)));
            }
            "update/upload" => {
                let root = crate::update::directory(&self.path);
                return crate::update::upload(&root, &r.body).map(Reply::json);
            }
            "update/install" => {
                let root = crate::update::directory(&self.path);
                return crate::update::start(&root).map(Reply::text);
            }
            "web/details" => {
                let config = WebConfig::load(&self.web_path)?;
                return Ok(Reply::json(json::object([
                    ("runtime", js(self.runtime)),
                    ("webPanel", "true".into()),
                    ("bind", js(&config.bind.to_string())),
                    (
                        "tokenFile",
                        js(&config
                            .token_file
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()),
                    ),
                    ("authEnabled", config.auth_enabled.to_string()),
                    ("presenceMode", js(&config.presence_mode)),
                    (
                        "presenceBarEnabled",
                        config.presence_bar_enabled.to_string(),
                    ),
                    ("absenceProfile", config.absence_profile.to_string()),
                    ("heartbeatTimeout", config.heartbeat_timeout.to_string()),
                    ("statisticsEnabled", config.statistics_enabled.to_string()),
                    (
                        "statisticsRetentionDays",
                        js(&config
                            .statistics_retention_days
                            .map(|days| days.to_string())
                            .unwrap_or_else(|| "continuous".into())),
                    ),
                    ("mqttEnabled", config.mqtt_enabled.to_string()),
                    ("mqttBroker", js(&config.mqtt_broker)),
                    ("mqttUsername", js(&config.mqtt_username)),
                    ("mqttTls", config.mqtt_tls.to_string()),
                    ("mqttClientId", js(&config.mqtt_client_id)),
                    ("mqttTopicPrefix", js(&config.mqtt_topic_prefix)),
                    ("mqttDiscoveryTopic", js(&config.mqtt_discovery_topic)),
                    ("mqttDiscoveryEnabled", config.mqtt_discovery_enabled.to_string()),
                    ("mqttPasswordPresent", self.mqtt_password_present().to_string()),
                    ("path", js(&self.web_path.display().to_string())),
                ])));
            }
            "notifications/test" => {
                let channel = get("channel");
                return match channel {
                    "in-app" => {
                        let notification = Notification {
                            id: self.next_notification_id,
                            time: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            kind: NotificationKind::Critical,
                            title: "Test powiadomienia".into(),
                            body: "To jest test Centrum powiadomień.".into(),
                            sound: 0,
                        };
                        self.next_notification_id = self.next_notification_id.saturating_add(1);
                        self.notifications.push_back(notification);
                        while self.notifications.len() > 100 {
                            self.notifications.pop_front();
                        }
                        Ok(Reply::ok())
                    }
                    "native" => {
                        let Some(sender) = &self.event_sender else {
                            return Err(
                                "Powiadomienia systemowe są dostępne tylko w aplikacji desktopowej"
                                    .into(),
                            );
                        };
                        sender
                            .try_send(StationEvent::Notification(Notification {
                                id: self.next_notification_id,
                                time: SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs(),
                                kind: NotificationKind::Critical,
                                title: "Test powiadomienia".into(),
                                body: "To jest test dymka systemowego.".into(),
                                sound: 0,
                            }))
                            .map_err(|_| "Kolejka powiadomień desktopowych jest zajęta")?;
                        Ok(Reply::ok())
                    }
                    "sound" => {
                        let Some(sender) = &self.event_sender else {
                            return Err(
                                "Dźwięk systemowy jest dostępny tylko w aplikacji desktopowej"
                                    .into(),
                            );
                        };
                        sender
                            .try_send(StationEvent::Sound(1))
                            .map_err(|_| "Kolejka powiadomień desktopowych jest zajęta")?;
                        Ok(Reply::ok())
                    }
                    _ => Err("Nieznany kanał testowy powiadomień".into()),
                };
            }
            "telegram/status" => {
                let status = if !self.config.telegram_enabled {
                    "disabled"
                } else if !self.telegram_token_present() {
                    "missing_token"
                } else if self.config.telegram_chat_id.is_empty() {
                    "waiting_chat"
                } else {
                    "configured"
                };
                return Ok(Reply::json(json::object([
                    ("status", js(status)),
                    ("enabled", self.config.telegram_enabled.to_string()),
                    ("tokenPresent", self.telegram_token_present().to_string()),
                    ("chatId", js(&self.config.telegram_chat_id)),
                ])));
            }
            "telegram/token" => {
                let token = get("token").trim();
                let path = self.telegram_token_path();
                if token.is_empty() {
                    match fs::remove_file(&path) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => {
                            return Err(format!("Nie można usunąć tokenu Telegrama: {error}"));
                        }
                    }
                } else {
                    crate::telegram::TelegramClient::new(token.to_owned())
                        .map_err(|e| e.to_string())?;
                    storage::write_secret_atomic(&path, token.as_bytes())
                        .map_err(|e| format!("Nie można zapisać tokenu Telegrama: {e}"))?;
                }
                return Ok(Reply::ok());
            }
            "telegram/select" => {
                let chat = get("chat_id").trim();
                if !crate::telegram::valid_chat_id(chat) {
                    return Err("Nieprawidłowy identyfikator czatu Telegrama".into());
                }
                self.config.telegram_chat_id = chat.into();
                let encoded = crate::config::encode(&self.config);
                storage::backup_existing(&self.path).map_err(|e| e.to_string())?;
                storage::write_atomic(&self.path, encoded.as_bytes()).map_err(|e| e.to_string())?;
                self.raw = encoded;
                return Ok(Reply::ok());
            }
            "telegram/discover" => {
                let Some(sender) = &self.telegram_sender else {
                    return Err("Klient Telegrama nie jest uruchomiony".into());
                };
                let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
                sender
                    .try_send(crate::telegram::Command::Discover {
                        token_path: self.telegram_token_path(),
                        reply: reply_tx,
                    })
                    .map_err(|_| "Kolejka Telegrama jest zajęta")?;
                let chats = reply_rx
                    .recv_timeout(Duration::from_secs(15))
                    .map_err(|_| "Telegram nie odpowiedział w wymaganym czasie")??;
                return Ok(Reply::json(json::array(chats.into_iter().map(|chat| {
                    json::object([
                        ("id", js(&chat.id)),
                        ("kind", js(&chat.kind)),
                        ("name", js(&chat.name)),
                    ])
                }))));
            }
            "telegram/test" => {
                let chat = if get("chat_id").trim().is_empty() {
                    self.config.telegram_chat_id.clone()
                } else {
                    get("chat_id").trim().into()
                };
                if !crate::telegram::valid_chat_id(&chat) {
                    return Err("Wybierz lub wpisz identyfikator czatu Telegrama".into());
                }
                let Some(sender) = &self.telegram_sender else {
                    return Err("Klient Telegrama nie jest uruchomiony".into());
                };
                let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
                sender
                    .try_send(crate::telegram::Command::Test {
                        token_path: self.telegram_token_path(),
                        chat_id: chat,
                        reply: reply_tx,
                    })
                    .map_err(|_| "Kolejka Telegrama jest zajęta")?;
                reply_rx
                    .recv_timeout(Duration::from_secs(15))
                    .map_err(|_| "Telegram nie odpowiedział w wymaganym czasie")??;
                return Ok(Reply::ok());
            }
            "mqtt/password" => {
                let password = get("password");
                if password.len() > 256
                    || password
                        .bytes()
                        .any(|byte| byte.is_ascii_control())
                {
                    return Err("Hasło MQTT może mieć maksymalnie 256 znaków i nie może zawierać znaków sterujących".into());
                }
                let path = self.mqtt_password_path();
                if password.is_empty() {
                    match fs::remove_file(&path) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(format!("Nie można usunąć hasła MQTT: {error}")),
                    }
                } else {
                    storage::write_secret_atomic(&path, password.as_bytes())
                        .map_err(|error| format!("Nie można zapisać hasła MQTT: {error}"))?;
                }
                return Ok(Reply::ok());
            }
            "mqtt/discovery" => {
                let Some(sender) = &self.mqtt_discovery_sender else {
                    return Err("MQTT nie jest uruchomione. Włącz MQTT i zrestartuj aplikację.".into());
                };
                sender
                    .try_send(())
                    .map_err(|_| "Kolejka MQTT jest zajęta albo usługa jest zatrzymywana")?;
                return Ok(Reply::ok());
            }
            "serial/ports" => {
                let ports = Transport::available_serial_ports().map_err(|e| e.to_string())?;
                return Ok(Reply::json(json::array(
                    ports.into_iter().map(|port| js(&port)),
                )));
            }
            "web/save" => {
                let old_web = WebConfig::load(&self.web_path)?;
                let bind = get("bind")
                    .parse()
                    .map_err(|_| "Invalid web bind address")?;
                let token_file = get("token_file");
                let auth_enabled = match get("auth_enabled") {
                    "true" => true,
                    "false" => false,
                    "" => old_web.auth_enabled,
                    _ => return Err("auth_enabled must be true/false".into()),
                };
                let presence_mode = match get("presence_mode") {
                    "auto" | "always" | "dnd" => get("presence_mode").to_owned(),
                    "" => old_web.presence_mode.clone(),
                    _ => return Err("Invalid presence_mode".into()),
                };
                let presence_bar_enabled = match get("presence_bar_enabled") {
                    "true" => true,
                    "false" => false,
                    "" => old_web.presence_bar_enabled,
                    _ => return Err("presence_bar_enabled must be true/false".into()),
                };
                let absence_profile = if get("absence_profile").is_empty() {
                    old_web.absence_profile
                } else {
                    get("absence_profile")
                        .parse::<u8>()
                        .map_err(|_| "Invalid absence_profile")?
                };
                if !(1..=4).contains(&absence_profile) {
                    return Err("absence_profile must be 1..4".into());
                }
                let heartbeat_timeout = if get("heartbeat_timeout").is_empty() {
                    old_web.heartbeat_timeout
                } else {
                    get("heartbeat_timeout")
                        .parse::<u64>()
                        .map_err(|_| "Invalid heartbeat_timeout")?
                        .clamp(15, 3600)
                };
                let statistics_enabled = match get("statistics_enabled") {
                    "true" => true,
                    "false" => false,
                    "" => old_web.statistics_enabled,
                    _ => return Err("statistics_enabled must be true/false".into()),
                };
                let statistics_retention_days = match get("statistics_retention_days") {
                    "" => old_web.statistics_retention_days,
                    "continuous" => None,
                    value => Some(
                        value
                            .parse::<u64>()
                            .map_err(|_| "statistics_retention_days must be continuous or a number")?
                            .clamp(1, 3650),
                    ),
                };
                let mqtt_enabled = match get("mqtt_enabled") {
                    "true" => true,
                    "false" => false,
                    "" => old_web.mqtt_enabled,
                    _ => return Err("mqtt_enabled must be true/false".into()),
                };
                let mqtt_tls = match get("mqtt_tls") {
                    "true" => true,
                    "false" => false,
                    "" => old_web.mqtt_tls,
                    _ => return Err("mqtt_tls must be true/false".into()),
                };
                let mqtt_discovery_enabled = match get("mqtt_discovery_enabled") {
                    "true" => true,
                    "false" => false,
                    "" => old_web.mqtt_discovery_enabled,
                    _ => return Err("mqtt_discovery_enabled must be true/false".into()),
                };
                let mqtt_broker = if get("mqtt_broker").is_empty() {
                    old_web.mqtt_broker.clone()
                } else {
                    get("mqtt_broker").trim().to_owned()
                };
                let mqtt_username = get("mqtt_username").trim().to_owned();
                let mqtt_client_id = if get("mqtt_client_id").trim().is_empty() {
                    old_web.mqtt_client_id.clone()
                } else {
                    get("mqtt_client_id").trim().to_owned()
                };
                let mqtt_topic_prefix = if get("mqtt_topic_prefix").trim().is_empty() {
                    old_web.mqtt_topic_prefix.clone()
                } else {
                    get("mqtt_topic_prefix").trim().to_owned()
                };
                let mqtt_discovery_topic = if get("mqtt_discovery_topic").trim().is_empty() {
                    old_web.mqtt_discovery_topic.clone()
                } else {
                    get("mqtt_discovery_topic").trim().to_owned()
                };
                if mqtt_broker.len() > 256
                    || mqtt_username.len() > 128
                    || mqtt_client_id.len() > 128
                    || mqtt_topic_prefix.len() > 128
                    || mqtt_discovery_topic.len() > 128
                {
                    return Err("Ustawienia MQTT przekraczają dozwolony rozmiar".into());
                }
                if mqtt_topic_prefix.chars().any(char::is_whitespace)
                    || mqtt_topic_prefix.contains('#')
                    || mqtt_topic_prefix.contains('+')
                {
                    return Err("Prefiks tematów MQTT nie może zawierać białych znaków, # ani +".into());
                }
                if mqtt_discovery_topic.chars().any(char::is_whitespace)
                    || mqtt_discovery_topic.contains('#')
                    || mqtt_discovery_topic.contains('+')
                {
                    return Err("Temat Discovery MQTT nie może zawierać białych znaków, # ani +".into());
                }
                let token_path = (!token_file.is_empty()).then(|| PathBuf::from(token_file));
                if token_path.as_ref().is_some_and(|path| {
                    path.components()
                        .any(|component| matches!(component, std::path::Component::ParentDir))
                }) {
                    return Err("Token file path cannot contain '..'".into());
                }
                let config = WebConfig {
                    bind,
                    token_file: token_path,
                    auth_enabled,
                    auth_configured: true,
                    presence_mode: presence_mode.clone(),
                    presence_bar_enabled,
                    absence_profile,
                    heartbeat_timeout,
                    statistics_enabled,
                    statistics_retention_days,
                    mqtt_enabled,
                    mqtt_broker,
                    mqtt_username,
                    mqtt_tls,
                    mqtt_client_id,
                    mqtt_topic_prefix,
                    mqtt_discovery_topic,
                    mqtt_discovery_enabled,
                };
                let restart_required = config.bind != old_web.bind
                    || config.token_file != old_web.token_file
                    || config.auth_enabled != old_web.auth_enabled
                    || config.mqtt_enabled != old_web.mqtt_enabled
                    || config.mqtt_broker != old_web.mqtt_broker
                    || config.mqtt_username != old_web.mqtt_username
                    || config.mqtt_tls != old_web.mqtt_tls
                    || config.mqtt_client_id != old_web.mqtt_client_id
                    || config.mqtt_topic_prefix != old_web.mqtt_topic_prefix
                    || config.mqtt_discovery_topic != old_web.mqtt_discovery_topic
                    || config.mqtt_discovery_enabled != old_web.mqtt_discovery_enabled;
                storage::backup_existing(&self.web_path).map_err(|e| e.to_string())?;
                storage::write_atomic(&self.web_path, config.encode().as_bytes())
                    .map_err(|e| e.to_string())?;
                self.presence_mode = presence_mode;
                self.absence_profile = absence_profile;
                self.heartbeat_timeout = Duration::from_secs(heartbeat_timeout);
                self.statistics.set_enabled(statistics_enabled);
                self.statistics
                    .set_retention_days(statistics_retention_days);
                return Ok(Reply::json(json::object([
                    ("ok", "true".into()),
                    ("restartRequired", restart_required.to_string()),
                ])));
            }
            "presence/mode" => {
                let mode = get("mode");
                if !matches!(mode, "auto" | "always" | "dnd") {
                    return Err("Invalid presence mode".into());
                }
                let mut config = WebConfig::load(&self.web_path)?;
                config.presence_mode = mode.to_owned();
                storage::backup_existing(&self.web_path).map_err(|e| e.to_string())?;
                storage::write_atomic(&self.web_path, config.encode().as_bytes())
                    .map_err(|e| e.to_string())?;
                self.presence_mode = mode.to_owned();
                return Ok(Reply::json(json::object([
                    ("ok", "true".into()),
                    ("presenceMode", js(mode)),
                    ("available", self.operator_available().to_string()),
                ])));
            }
            "presence/heartbeat" => {
                self.last_heartbeat = Instant::now();
                return Ok(Reply::json(json::object([
                    ("ok", "true".into()),
                    ("available", self.operator_available().to_string()),
                ])));
            }
            "config/details" => {
                let interfaces = self.config.interfaces.iter().map(|interface| {
                    let path = interface
                        .beacon_path
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" ");
                    json::object([
                        ("name", js(&interface.name)),
                        ("description", js(&interface.description)),
                        ("endpoint", js(&interface.endpoint)),
                        (
                            "transport",
                            js(match interface.transport {
                                TransportKind::Tcp => "tcp",
                                TransportKind::Serial => "serial",
                            }),
                        ),
                        ("baud", interface.baud.to_string()),
                        ("port", interface.port.to_string()),
                        ("priority", interface.priority.to_string()),
                        ("enabled", interface.enabled.to_string()),
                        ("beaconEnabled", interface.beacon_enabled.to_string()),
                        ("beaconPath", js(&path)),
                    ])
                });
                let notifications = crate::station_events::NotificationKind::ALL
                    .into_iter()
                    .map(|kind| {
                        let rule = self.config.notifications.rule(kind);
                        json::object([
                            ("kind", js(kind.key())),
                            ("label", js(kind.label())),
                            ("inApp", rule.in_app.to_string()),
                            ("native", rule.native.to_string()),
                            ("telegram", rule.telegram.to_string()),
                            ("sound", rule.sound.to_string()),
                        ])
                    });
                return Ok(Reply::json(json::object([
                    (
                        "callsign",
                        js(&self
                            .config
                            .call
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default()),
                    ),
                    ("myname", js(&self.config.myname)),
                    ("locator", js(&self.config.locator)),
                    ("language", js(&self.config.language)),
                    ("welcome", self.config.welcome.to_string()),
                    ("goodbye", self.config.goodbye.to_string()),
                    ("remoteCommands", self.config.remote_commands.to_string()),
                    ("beacon", self.config.beacon.to_string()),
                    ("beaconSeconds", self.config.beacon_seconds.to_string()),
                    ("beaconTo", js(&self.config.beacon_to.to_string())),
                    ("digipeat", self.config.digipeat.to_string()),
                    ("t1", self.config.t1.to_string()),
                    ("t3", self.config.t3.to_string()),
                    ("retries", self.config.n2.to_string()),
                    ("paclen", self.config.paclen.to_string()),
                    ("accept", self.config.accept.to_string()),
                    (
                        "ax25Autonegotiation",
                        self.config.ax25_autonegotiation.to_string(),
                    ),
                    (
                        "ax25Modulo128",
                        self.config.ax25_modulo_128.to_string(),
                    ),
                    ("mailboxEnabled", self.config.mailbox_enabled.to_string()),
                    (
                        "mailboxCall",
                        js(&self
                            .config
                            .mailbox_call
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default()),
                    ),
                    ("yappEnabled", self.config.yapp_enabled.to_string()),
                    ("yappAutoReceive", self.config.yapp_auto_receive.to_string()),
                    ("yappAskLocation", self.config.yapp_ask_location.to_string()),
                    ("yappDownloadDir", js(&self.config.yapp_download_dir)),
                    ("yappMaxFiles", self.config.yapp_max_files.to_string()),
                    ("yappAutoCleanup", self.config.yapp_auto_cleanup.to_string()),
                    ("telegramEnabled", self.config.telegram_enabled.to_string()),
                    ("telegramChatId", js(&self.config.telegram_chat_id)),
                    (
                        "telegramTokenPresent",
                        self.telegram_token_present().to_string(),
                    ),
                    ("notifications", json::array(notifications)),
                    ("interfaces", json::array(interfaces)),
                ])));
            }
            "config/check" | "config/save" => {
                let text = get("text");
                if text.len() > 65536 {
                    return Err("Configuration exceeds 64 KiB".into());
                }
                let config = Config::parse(text)?;
                let rebuild = self.configuration_requires_rebuild(&config);
                if r.action == "config/check" {
                    return Ok(Reply::json(json::object([
                        ("requiresDisconnect", (rebuild && self.active()).to_string()),
                    ])));
                }
                if rebuild && self.active() {
                    return Err(
                        "Disconnect terminal and MailBox sessions before saving configuration"
                            .into(),
                    );
                }
                let text = crate::config::encode(&config);
                if !rebuild {
                    storage::backup_existing(&self.path).map_err(|e| e.to_string())?;
                    storage::write_atomic(&self.path, text.as_bytes()).map_err(|e| e.to_string())?;
                    self.automatic.apply_config(&config, self.now());
                    for (interface, settings) in self.interfaces.iter_mut().zip(&config.interfaces) {
                        interface.config = settings.clone();
                    }
                    self.engine.accept = config.accept && self.incoming_profile() != 4;
                    self.transfers.max_files = config.yapp_max_files;
                    self.transfers.auto_cleanup = config.yapp_auto_cleanup;
                    self.config = config;
                    self.raw = text;
                    return Ok(Reply::ok());
                }
                let mut next = Self::from_text(
                    self.path.clone(),
                    text.clone(),
                    self.demo_endpoint.clone(),
                    Some(self.web_path.clone()),
                    self.runtime,
                )?;
                storage::backup_existing(&self.path).map_err(|e| e.to_string())?;
                storage::write_atomic(&self.path, text.as_bytes()).map_err(|e| e.to_string())?;
                next.set_events(self.event_sender.clone());
                next.set_telegram_sender(self.telegram_sender.clone());
                next.set_mqtt_discovery_sender(self.mqtt_discovery_sender.clone());
                *self = next;
            }
            "templates" => {
                return Ok(Reply::json(json::array(
                    self.automatic.templates.text.iter().map(|t| js(t)),
                )));
            }
            "templates/save" => {
                let index = get("index")
                    .parse::<usize>()
                    .map_err(|_| "Invalid template")?;
                if index >= FILES.len() {
                    return Err("Invalid template".into());
                }
                crate::automation::validate(get("text"), index)?;
                let path = self
                    .path
                    .parent()
                    .unwrap()
                    .join("messages")
                    .join(FILES[index]);
                storage::write_atomic(&path, get("text").as_bytes()).map_err(|e| e.to_string())?;
                self.automatic.templates.text[index] = get("text").into();
            }
            "mail/create" => {
                let from = self
                    .config
                    .call
                    .clone()
                    .ok_or("Configure your callsign first")?;
                let kind = match get("kind") {
                    "B" => MessageType::Bulletin,
                    "T" => MessageType::Traffic,
                    _ => MessageType::Personal,
                };
                self.mailbox.add_typed(
                    from,
                    get("to").parse()?,
                    kind,
                    get("subject"),
                    get("body"),
                )?;
            }
            "mail/read" => {
                let id = get("id").parse::<u64>().map_err(|_| "Invalid message ID")?;
                let m = self
                    .mailbox
                    .messages()
                    .iter()
                    .find(|m| m.id == id)
                    .cloned()
                    .ok_or("Message not found")?;
                self.mailbox.mark_read(id)?;
                return Ok(Reply::json(json::object([
                    ("from", js(&m.from.to_string())),
                    ("subject", js(&m.subject)),
                    ("body", js(&m.body)),
                ])));
            }
            "mail/mark" => {
                let id = get("id").parse::<u64>().map_err(|_| "Invalid message ID")?;
                let read = match get("read") {
                    "true" => true,
                    "false" => false,
                    _ => return Err("read must be true or false".into()),
                };
                if !self.mailbox.set_read(id, read)? {
                    return Err("Message not found".into());
                }
            }
            "mail/delete" => {
                let id = get("id").parse::<u64>().map_err(|_| "Invalid message ID")?;
                if !self.mailbox.delete(id)? {
                    return Err("Message not found".into());
                }
            }
            "yapp/send" => {
                if !self.config.yapp_enabled {
                    return Err("Enable YAPP in configuration first".into());
                }
                let ev =
                    self.transfers
                        .start(id()?, get("name"), &r.body, &mut self.engine, now)?;
                self.events(ev, 0, false);
            }
            "yapp/accept" => {
                let ev =
                    self.transfers
                        .answer(id()?, get("accept") == "true", &mut self.engine, now)?;
                self.events(ev, 0, false);
            }
            "yapp/cancel" => {
                let ev = self.transfers.cancel(id()?, &mut self.engine, now)?;
                self.events(ev, 0, false);
            }
            "files/delete" => {
                self.transfers.delete_file(get("name"))?;
            }
            "monitor/export" => return Ok(Reply::text(self.monitor.text())),
            "files" => {
                let files = self
                    .transfers
                    .list_files()
                    .into_iter()
                    .map(|(name, size, modified)| {
                        json::object([
                            ("name", js(&name)),
                            ("size", size.to_string()),
                            ("modified", modified.to_string()),
                        ])
                    })
                    .collect::<Vec<_>>();
                return Ok(Reply::json(json::object([
                    ("files", json::array(files)),
                    ("count", self.transfers.list_files().len().to_string()),
                    ("usedBytes", self.transfers.used_bytes().to_string()),
                    ("maxFiles", self.transfers.max_files.to_string()),
                    ("autoCleanup", self.transfers.auto_cleanup.to_string()),
                ])));
            }
            _ => return Err("Unknown command".into()),
        }
        Ok(Reply::ok())
    }
    pub fn mqtt_callsign(&self) -> String {
        self
            .config
            .call
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "NOCALL".into())
    }

    pub fn mqtt_messages(
        &self,
        prefix: &str,
        discovery_enabled: bool,
        discovery_topic: &str,
    ) -> Vec<crate::mqtt::Message> {
        let callsign = self.mqtt_callsign();
        let base = format!("{}/{}", prefix.trim_end_matches('/'), callsign);
        let status_topic = format!("{base}/status");
        let tnc_topic = format!("{base}/tnc");
        let mheard_topic = format!("{base}/mheard");
        let device = json::object([
            ("ids", js(&format!("ultimatepr-{callsign}"))),
            ("name", js(&format!("UltimatePR {callsign}"))),
            ("mf", js("UltimatePR")),
            ("mdl", js("Station")),
        ]);
        let discovery =
            |component: &str,
             unique: &str,
             name: &str,
             topic: &str,
             template: &str,
             attributes_template: Option<&str>| {
                let mut fields = vec![
                    ("name", js(name)),
                    ("uniq_id", js(unique)),
                    ("stat_t", js(topic)),
                    ("val_tpl", js(template)),
                    ("dev", device.clone()),
                ];
                if let Some(attributes_template) = attributes_template {
                    fields.push(("json_attributes_topic", js(topic)));
                    fields.push(("json_attributes_template", js(attributes_template)));
                }
                crate::mqtt::Message {
                    topic: format!(
                        "{}/sensor/ultimatepr-{callsign}/{component}/config",
                        discovery_topic.trim_end_matches('/'),
                    ),
                    payload: json::object(fields),
                    retain: true,
                }
            };
        let mut messages = vec![
            crate::mqtt::Message {
                topic: status_topic.clone(),
                payload: json::object([
                    ("state", js("online")),
                    ("callsign", js(&callsign)),
                    ("uptime", (self.now() / 1000).to_string()),
                ]),
                retain: true,
            },
            crate::mqtt::Message {
                topic: tnc_topic.clone(),
                payload: json::array(self.interfaces.iter().map(|interface| {
                    json::object([
                        ("name", js(&interface.config.name)),
                        ("enabled", interface.config.enabled.to_string()),
                        ("online", interface.transport.is_some().to_string()),
                        ("error", js(&interface.error)),
                    ])
                })),
                retain: true,
            },
            crate::mqtt::Message {
                topic: mheard_topic.clone(),
                payload: json::array(self.heard.entries().iter().map(|entry| {
                    json::object([
                        ("call", js(&entry.source.to_string())),
                        (
                            "interface",
                            js(&self
                                .config
                                .interfaces
                                .get(entry.interface)
                                .map(|interface| interface.name.clone())
                                .unwrap_or_else(|| format!("TNC{}", entry.interface + 1))),
                        ),
                        (
                            "age",
                            entry
                                .age_seconds_at(self.now(), Heard::current_utc())
                                .to_string(),
                        ),
                        ("destination", js(&entry.destination.to_string())),
                        ("via", js(&entry.route())),
                    ])
                })),
                retain: true,
            },
        ];
        if discovery_enabled {
            messages.push(discovery(
                "status",
                &format!("ultimatepr-{callsign}-status"),
                &format!("{callsign} status"),
                &status_topic,
                "{{ value_json.state }}",
                None,
            ));
            messages.push(discovery(
                "tnc",
                &format!("ultimatepr-{callsign}-tnc"),
                &format!("{callsign} TNC"),
                &tnc_topic,
                "{{ value_json | length }}",
                Some("{\"interfaces\": {{ value_json | tojson }}}"),
            ));
            messages.push(discovery(
                "mheard",
                &format!("ultimatepr-{callsign}-mheard"),
                &format!("{callsign} MHeard"),
                &mheard_topic,
                "{{ value_json | length }}",
                Some("{\"stations\": {{ value_json | tojson }}}"),
            ));
        }
        messages
    }

    pub fn snapshot(&self) -> String {
        let sessions = json::array((0..SESSIONS).map(|id| {
            let s = &self.engine.sessions[id];
            json::object([
                ("id", id.to_string()),
                (
                    "call",
                    js(&s.as_ref().map(|s| s.remote.to_string()).unwrap_or_default()),
                ),
                (
                    "state",
                    js(&s
                        .as_ref()
                        .map(|s| format!("{:?}", s.state))
                        .unwrap_or("Idle".into())),
                ),
                (
                    "interface",
                    s.as_ref()
                        .map_or("null".into(), |s| s.interface.to_string()),
                ),
                ("queued", s.as_ref().map_or(0, |s| s.buffered()).to_string()),
                ("history", self.histories[id].json()),
                ("transfer", self.transfers.snapshot(id)),
            ])
        }));
        json::object([
            ("demo", self.demo_endpoint.is_some().to_string()),
            (
                "callsign",
                js(&self
                    .config
                    .call
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default()),
            ),
            ("operator", js(&self.config.myname)),
            ("uptime", (self.now() / 1000).to_string()),
            ("rx", self.rx.to_string()),
            ("tx", self.tx.to_string()),
            ("stopping", self.stopping.to_string()),
            ("sessions", sessions),
            (
                "interfaces",
                json::array(self.interfaces.iter().enumerate().map(|(id, i)| {
                    json::object([
                        ("id", id.to_string()),
                        ("name", js(&i.config.name)),
                        ("description", js(&i.config.description)),
                        ("endpoint", js(&i.config.endpoint)),
                        ("enabled", i.config.enabled.to_string()),
                        ("online", i.transport.is_some().to_string()),
                        ("error", js(&i.error)),
                    ])
                })),
            ),
            ("monitor", self.monitor.json()),
            (
                "notificationSounds",
                json::array(self.notification_sounds.iter().map(|(id, count)| {
                    json::object([("id", id.to_string()), ("count", count.to_string())])
                })),
            ),
            (
                "notifications",
                json::array(self.notifications.iter().map(|notification| {
                    json::object([
                        ("id", notification.id.to_string()),
                        ("time", notification.time.to_string()),
                        ("kind", js(notification.kind.key())),
                        ("title", js(&notification.title)),
                        ("body", js(&notification.body)),
                        ("sound", notification.sound.to_string()),
                    ])
                })),
            ),
            (
                "heard",
                json::array(self.heard.entries().iter().map(|e| {
                    json::object([
                        ("call", js(&e.source.to_string())),
                        ("interface", e.interface.to_string()),
                        ("frames", e.frames.to_string()),
                        (
                            "age",
                            e.age_seconds_at(self.now(), Heard::current_utc())
                                .to_string(),
                        ),
                        ("destination", js(&e.destination.to_string())),
                        ("via", js(&e.route())),
                        ("port", e.port.to_string()),
                        ("utc", js(&e.utc())),
                        ("last_frame", js(&e.last_frame)),
                    ])
                })),
            ),
            (
                "mail",
                json::array(self.mailbox.messages().iter().rev().map(|m| {
                    json::object([
                        ("id", m.id.to_string()),
                        ("from", js(&m.from.to_string())),
                        ("to", js(&m.to.to_string())),
                        ("subject", js(&m.subject)),
                        ("body", js(&m.body)),
                        ("kind", js(m.message_type.code())),
                        ("read", m.read.to_string()),
                        ("time", m.timestamp.to_string()),
                    ])
                })),
            ),
            ("locator", js(&self.config.locator)),
            ("mailboxEnabled", self.config.mailbox_enabled.to_string()),
            (
                "mailboxSessions",
                self.mailbox_engine
                    .as_ref()
                    .map_or(0, |e| e.sessions.iter().flatten().count())
                    .to_string(),
            ),
            ("yappEnabled", self.config.yapp_enabled.to_string()),
        ])
    }
}
