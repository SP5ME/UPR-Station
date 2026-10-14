use crate::ax25::Call;
use crate::station_events::NotificationSettings;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

pub const MIN_BEACON_SECONDS: u64 = 10 * 60;
const DEFAULT_BEACON_SECONDS: u64 = MIN_BEACON_SECONDS;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportKind {
    Tcp,
    Serial,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceConfig {
    pub name: String,
    pub description: String,
    pub endpoint: String,
    pub transport: TransportKind,
    pub baud: u32,
    pub port: u8,
    pub priority: u8,
    pub enabled: bool,
    pub beacon_enabled: bool,
    pub beacon_path: Vec<Call>,
}

fn copy_directory(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source = entry.path();
        let destination = destination.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_directory(&source, &destination)?;
        } else if kind.is_file() && !destination.exists() {
            fs::copy(source, destination)?;
        }
    }
    Ok(())
}
pub struct Config {
    pub myname: String,
    /// Optional Maidenhead locator for the station and message templates.
    pub locator: String,
    pub welcome: bool,
    pub goodbye: bool,
    pub remote_commands: bool,
    pub beacon: bool,
    pub digipeat: bool,
    pub beacon_seconds: u64,
    pub beacon_to: Call,
    pub language: String,
    pub call: Option<Call>,
    pub endpoint: String,
    pub port: u8,
    pub t1: u64,
    pub t3: u64,
    pub n2: u8,
    pub paclen: usize,
    pub accept: bool,
    /// Negotiate AX.25 parameters with XID after link establishment.
    pub ax25_autonegotiation: bool,
    /// Allow modulo-128 link setup with SABME and accept incoming SABME.
    pub ax25_modulo_128: bool,
    pub mailbox_enabled: bool,
    pub mailbox_call: Option<Call>,
    pub yapp_enabled: bool,
    pub yapp_auto_receive: bool,
    pub yapp_ask_location: bool,
    pub yapp_download_dir: String,
    pub yapp_max_files: usize,
    pub yapp_auto_cleanup: bool,
    /// Enables outbound Telegram notifications.
    pub telegram_enabled: bool,
    /// Telegram chat id or @channel username for outbound notifications.
    pub telegram_chat_id: String,
    /// Relative filename containing the Telegram bot token.
    pub telegram_token_file: String,
    /// Shared delivery preferences for application, native and sound notifications.
    pub notifications: NotificationSettings,
    pub interfaces: Vec<InterfaceConfig>,
}
impl Config {
    fn setup_marker_path(path: &Path) -> PathBuf {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| "station.conf".into());
        path.with_file_name(format!("{name}.setup-required"))
    }

    pub fn mark_setup_required(path: &Path) -> Result<(), String> {
        let marker = Self::setup_marker_path(path);
        let parent = marker.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        fs::write(&marker, b"first-run setup required\n")
            .map_err(|e| format!("{}: {e}", marker.display()))
    }

    pub fn clear_setup_required(path: &Path) -> Result<(), String> {
        let marker = Self::setup_marker_path(path);
        match fs::remove_file(&marker) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("{}: {error}", marker.display())),
        }
    }

    pub fn fresh() -> Self {
        Self {
            myname: String::new(),
            locator: String::new(),
            welcome: true,
            goodbye: true,
            remote_commands: true,
            beacon: true,
            digipeat: true,
            beacon_seconds: DEFAULT_BEACON_SECONDS,
            beacon_to: "BEACON".parse().unwrap(),
            language: "en".into(),
            call: None,
            endpoint: "127.0.0.1:8001".into(),
            port: 0,
            t1: 5000,
            t3: 180000,
            n2: 10,
            paclen: 128,
            accept: true,
            ax25_autonegotiation: true,
            ax25_modulo_128: true,
            mailbox_enabled: false,
            mailbox_call: None,
            yapp_enabled: false,
            yapp_auto_receive: false,
            yapp_ask_location: false,
            yapp_download_dir: String::new(),
            yapp_max_files: 1000,
            yapp_auto_cleanup: false,
            telegram_enabled: false,
            telegram_chat_id: String::new(),
            telegram_token_file: "telegram.bot-token".into(),
            notifications: NotificationSettings::default(),
            interfaces: Vec::new(),
        }
    }

    pub fn default_path() -> PathBuf {
        #[cfg(windows)]
        if let Some(root) = env::var_os("APPDATA") {
            return PathBuf::from(root)
                .join("UltimatePR Station")
                .join("station.conf");
        }

        #[cfg(target_os = "macos")]
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("UltimatePR Station")
                .join("station.conf");
        }

        if let Some(root) = env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(root)
                .join("ultimatepr-station")
                .join("station.conf");
        }
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join("ultimatepr-station")
                .join("station.conf");
        }
        PathBuf::from("station.conf")
    }

    pub fn migrate_legacy(path: &Path, legacy: &Path) -> Result<bool, String> {
        if path.exists()
            || Self::setup_marker_path(path).exists()
            || !legacy.is_file()
            || path == legacy
        {
            return Ok(false);
        }
        let parent = path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;

        if let Some(legacy_parent) = legacy.parent() {
            for asset in ["messages", "languages"] {
                let source = legacy_parent.join(asset);
                let destination = parent.join(asset);
                if source.is_dir() && !destination.exists() {
                    copy_directory(&source, &destination)
                        .map_err(|e| format!("Cannot migrate {asset}: {e}"))?;
                }
            }
        }

        let mut destination = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
            Err(error) => return Err(format!("Cannot create {}: {error}", path.display())),
        };
        let mut source =
            fs::File::open(legacy).map_err(|e| format!("Cannot read {}: {e}", legacy.display()))?;
        if let Err(error) =
            io::copy(&mut source, &mut destination).and_then(|_| destination.sync_all())
        {
            let _ = fs::remove_file(path);
            return Err(format!("Cannot migrate {}: {error}", legacy.display()));
        }
        Ok(true)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&text)
    }

    pub fn load_or_fresh(path: &Path) -> Result<Self, String> {
        match fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::fresh()),
            Err(error) => Err(format!("{}: {error}", path.display())),
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let mut c = Self::fresh();
        let mut keys = std::collections::HashSet::new();
        let mut has_modulo_128_key = false;
        let mut interfaces = Vec::new();
        let mut beacon_profiles = std::collections::HashMap::new();
        let mut legacy_beacon_path = None;
        let mut legacy_beacon_interfaces = None;
        for (i, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("Config line {}: expected key=value", i + 1))?;
            let key = key.trim();
            let value = value.trim();
            if !keys.insert(key) {
                return Err(format!("Duplicate config key: {key}"));
            }
            if matches!(key, "notification_winlink_in_app" | "notification_winlink_in-app"
                | "notification_winlink_native" | "notification_winlink_telegram"
                | "notification_winlink_sound") {
                // Retired settings must not prevent loading an existing station.
                continue;
            }
            if key.starts_with("notification_") {
                c.notifications.set(key, value)?;
                continue;
            }
            let number = || {
                value
                    .parse::<u64>()
                    .map_err(|_| format!("Invalid number for {key}"))
            };
            match key {
                "myname" => {
                    if value.len() > 80 || value.chars().any(char::is_control) {
                        return Err("Operator name must be at most 80 bytes".into());
                    }
                    c.myname = value.into();
                }
                "locator" => {
                    if !value.is_empty() && !valid_locator(value) {
                        return Err("Locator must use Maidenhead form such as JO92aa".into());
                    }
                    c.locator = value.to_ascii_uppercase();
                }
                "welcome_enabled" => {
                    c.welcome = value
                        .parse()
                        .map_err(|_| "welcome_enabled must be true/false")?
                }
                "goodbye_enabled" => {
                    c.goodbye = value
                        .parse()
                        .map_err(|_| "goodbye_enabled must be true/false")?
                }
                "remote_commands" => {
                    c.remote_commands = value
                        .parse()
                        .map_err(|_| "remote_commands must be true/false")?
                }
                "beacon_enabled" => {
                    c.beacon = value
                        .parse()
                        .map_err(|_| "beacon_enabled must be true/false")?
                }
                "digipeat_enabled" => {
                    c.digipeat = value
                        .parse()
                        .map_err(|_| "digipeat_enabled must be true/false")?
                }
                "beacon_seconds" => {
                    let configured_seconds = number()?;
                    if !(10..=86400).contains(&configured_seconds) {
                        return Err("Beacon interval must be 10-1440 minutes".into());
                    }
                    // Older releases allowed shorter intervals. Keep those
                    // configs loadable, but enforce the current 10-minute floor.
                    c.beacon_seconds = configured_seconds.max(MIN_BEACON_SECONDS);
                }
                "beacon_to" => c.beacon_to = value.parse()?,
                "beacon_path" => {
                    let path = value
                        .split_whitespace()
                        .map(str::parse)
                        .collect::<Result<Vec<_>, _>>()?;
                    if path.len() > 8 {
                        return Err("Beacon path allows at most 8 digipeaters".into());
                    }
                    legacy_beacon_path = Some(path);
                }
                "beacon_interfaces" => {
                    legacy_beacon_interfaces = Some(if value.eq_ignore_ascii_case("all") {
                        None
                    } else {
                        Some(if value.is_empty() {
                            Vec::new()
                        } else {
                            value
                                .split(',')
                                .map(|name| name.trim().to_string())
                                .collect()
                        })
                    });
                }
                key if key.starts_with("beacon_interface_") => {
                    let name = key.trim_start_matches("beacon_interface_");
                    if name.is_empty()
                        || name.len() > 16
                        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    {
                        return Err("Invalid beacon interface name".into());
                    }
                    let (enabled, route) = value.split_once(',').unwrap_or((value, ""));
                    let enabled = enabled
                        .parse::<bool>()
                        .map_err(|_| "Beacon interface enabled must be true/false")?;
                    let path = route
                        .split_whitespace()
                        .map(str::parse)
                        .collect::<Result<Vec<_>, _>>()?;
                    if path.len() > 8 {
                        return Err("Beacon path allows at most 8 digipeaters".into());
                    }
                    if beacon_profiles
                        .insert(name.to_ascii_lowercase(), (enabled, path))
                        .is_some()
                    {
                        return Err("Duplicate beacon interface profile".into());
                    }
                }
                "language" => {
                    if value.is_empty()
                        || value.len() > 16
                        || !value.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
                    {
                        return Err("Invalid language code".into());
                    }
                    c.language = value.into();
                }
                "font_size" => {
                    let n = number()?;
                    if !(6..=36).contains(&n) {
                        return Err("font_size must be 6-36 points".into());
                    }
                    // Retained as a validated legacy key; the terminal emulator
                    // controls font sizing independently of this application.
                }
                "callsign" => {
                    if !value.is_empty() {
                        c.call = Some(value.parse()?);
                    }
                }
                "tnc" => c.endpoint = value.into(),
                "kiss_port" => {
                    let n = number()?;
                    if n > 15 {
                        return Err("kiss_port must be 0-15".into());
                    }
                    c.port = n as u8;
                }
                "t1_ms" => c.t1 = number()?,
                "t3_ms" => c.t3 = number()?,
                "retries" => {
                    let n = number()?;
                    if !(1..=30).contains(&n) {
                        return Err("retries must be 1-30".into());
                    }
                    c.n2 = n as u8;
                }
                "paclen" => {
                    let n = number()?;
                    if !(1..=256).contains(&n) {
                        return Err("paclen must be 1-256".into());
                    }
                    c.paclen = n as usize;
                }
                "accept_incoming" => {
                    c.accept = value
                        .parse()
                        .map_err(|_| "accept_incoming must be true/false")?
                }
                "ax25_autonegotiation" => {
                    c.ax25_autonegotiation = value
                        .parse()
                        .map_err(|_| "ax25_autonegotiation must be true/false")?
                }
                "ax25_modulo_128" => {
                    c.ax25_modulo_128 = value
                        .parse()
                        .map_err(|_| "ax25_modulo_128 must be true/false")?;
                    has_modulo_128_key = true;
                }
                "mailbox_enabled" => {
                    c.mailbox_enabled = value
                        .parse()
                        .map_err(|_| "mailbox_enabled must be true/false")?
                }
                "mailbox_callsign" => {
                    c.mailbox_call = if value.is_empty() {
                        None
                    } else {
                        Some(value.parse()?)
                    }
                }
                "yapp_enabled" => {
                    c.yapp_enabled = value
                        .parse()
                        .map_err(|_| "yapp_enabled must be true/false")?
                }
                "yapp_auto_receive" => {
                    c.yapp_auto_receive = value
                        .parse()
                        .map_err(|_| "yapp_auto_receive must be true/false")?
                }
                "yapp_ask_location" => {
                    c.yapp_ask_location = value
                        .parse()
                        .map_err(|_| "yapp_ask_location must be true/false")?
                }
                "yapp_download_dir" => {
                    if value.len() > 1024
                        || value.contains('#')
                        || value.chars().any(char::is_control)
                    {
                        return Err("YAPP download directory contains unsupported characters or exceeds 1024 bytes".into());
                    }
                    c.yapp_download_dir = value.into();
                }
                "yapp_max_files" => {
                    c.yapp_max_files = value
                        .parse::<usize>()
                        .map_err(|_| "yapp_max_files must be a number")?
                        .clamp(1, 100000);
                }
                "yapp_auto_cleanup" => {
                    c.yapp_auto_cleanup = value
                        .parse()
                        .map_err(|_| "yapp_auto_cleanup must be true/false")?;
                }
                // Compatibility only: retired module settings are accepted but never used or emitted.
                "wl2k_enabled" | "wl2k_call" | "wl2k_gateway" | "wl2k_via"
                | "wl2k_interface" | "wl2k_sync_hours" | "wl2k_auto_sync"
                | "wl2k_locator" | "wl2k_password_file" => {}
                "telegram_enabled" => {
                    c.telegram_enabled = value
                        .parse()
                        .map_err(|_| "telegram_enabled must be true/false")?;
                }
                "telegram_chat_id" => {
                    if value.len() > 128 || value.chars().any(char::is_control) {
                        return Err(
                            "Telegram chat id is too long or contains control characters".into(),
                        );
                    }
                    if !value.is_empty() && !crate::telegram::valid_chat_id(value) {
                        return Err("Telegram chat id must be numeric or start with @".into());
                    }
                    c.telegram_chat_id = value.into();
                }
                "telegram_token_file" => {
                    if !valid_secret_filename(value) {
                        return Err(
                            "Telegram token filename must be a safe relative filename".into()
                        );
                    }
                    c.telegram_token_file = value.into();
                }
                key if key.starts_with("interface_") => {
                    let name = key.trim_start_matches("interface_");
                    if name.is_empty()
                        || name.len() > 16
                        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    {
                        return Err("Interface name must be 1-16 ASCII letters, digits or _".into());
                    }
                    let fields = value.splitn(5, ',').map(str::trim).collect::<Vec<_>>();
                    if !(2..=5).contains(&fields.len()) {
                        return Err(
                            "Interface value must be endpoint,kiss_port[,priority[,enabled[,description]]]"
                                .into(),
                        );
                    }
                    let port = fields[1]
                        .parse::<u8>()
                        .map_err(|_| "KISS port must be 0-15")?;
                    if port > 15 {
                        return Err("KISS port must be 0-15".into());
                    }
                    let priority = fields.get(2).map_or(Ok(100), |v| {
                        v.parse::<u8>()
                            .map_err(|_| "Interface priority must be 0-255")
                    })?;
                    let enabled = fields.get(3).map_or(Ok(true), |v| {
                        v.parse::<bool>()
                            .map_err(|_| "Interface enabled must be true/false")
                    })?;
                    let description = value
                        .splitn(5, ',')
                        .nth(4)
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    if description.chars().count() > 60 || description.chars().any(char::is_control)
                    {
                        return Err("Interface description must be at most 60 characters".into());
                    }
                    let (transport, endpoint, baud) = parse_transport_endpoint(fields[0])?;
                    interfaces.push(InterfaceConfig {
                        name: name.to_string(),
                        description,
                        endpoint,
                        transport,
                        baud,
                        port,
                        priority,
                        enabled,
                        beacon_enabled: true,
                        beacon_path: Vec::new(),
                    });
                }
                _ => return Err(format!("Unknown config key: {key}")),
            }
        }
        if c.t1 < 1000 || c.t1 > 120000 || c.t3 < c.t1 || c.t3 > 3600000 {
            return Err("Timers: t1_ms 1000-120000; t3_ms >= t1_ms and <= 3600000".into());
        }
        if interfaces.is_empty() {
            interfaces.push(InterfaceConfig {
                name: "default".into(),
                description: String::new(),
                endpoint: c.endpoint.clone(),
                transport: TransportKind::Tcp,
                baud: 9600,
                port: c.port,
                priority: 100,
                enabled: true,
                beacon_enabled: true,
                beacon_path: Vec::new(),
            });
        }
        if interfaces.len() > 10 {
            return Err("At most 10 TNC interfaces are supported".into());
        }
        let mut names = std::collections::HashSet::new();
        let mut serial_devices = std::collections::HashSet::new();
        for interface in &interfaces {
            if !names.insert(interface.name.to_ascii_lowercase()) {
                return Err("Interface names must be unique".into());
            }
            if interface.transport == TransportKind::Serial
                && !serial_devices.insert(interface.endpoint.to_ascii_uppercase())
            {
                return Err("A serial COM device can only be assigned to one TNC interface".into());
            }
        }
        if let Some(selected) = &mut legacy_beacon_interfaces {
            let selected = selected.as_mut();
            if let Some(selected) = selected {
                let mut unique = std::collections::HashSet::new();
                for name in selected.iter_mut() {
                    let interface = interfaces
                        .iter()
                        .find(|i| i.name.eq_ignore_ascii_case(name))
                        .ok_or("beacon_interfaces contains an unknown interface")?;
                    *name = interface.name.clone();
                    if !unique.insert(name.to_ascii_lowercase()) {
                        return Err("beacon_interfaces contains a duplicate interface".into());
                    }
                }
            }
        }
        for interface in &mut interfaces {
            if let Some((enabled, path)) =
                beacon_profiles.remove(&interface.name.to_ascii_lowercase())
            {
                interface.beacon_enabled = enabled;
                interface.beacon_path = path;
            } else if legacy_beacon_path.is_some() || legacy_beacon_interfaces.is_some() {
                interface.beacon_path = legacy_beacon_path.clone().unwrap_or_default();
                interface.beacon_enabled = legacy_beacon_interfaces
                    .as_ref()
                    .and_then(Option::as_ref)
                    .is_none_or(|selected| {
                        selected
                            .iter()
                            .any(|name| name.eq_ignore_ascii_case(&interface.name))
                    });
            }
        }
        if !beacon_profiles.is_empty() {
            return Err("beacon_interface_ contains an unknown interface".into());
        }
        if c.mailbox_enabled && c.mailbox_call.is_none() {
            return Err("Enabled MailBox requires mailbox_callsign".into());
        }
        if c.mailbox_call.is_some() && c.mailbox_call == c.call {
            return Err("MailBox callsign must differ from station callsign".into());
        }
        interfaces.sort_by_key(|i| i.priority);
        if let Some(first) = interfaces.first() {
            c.endpoint = first.endpoint.clone();
            c.port = first.port;
        }
        c.interfaces = interfaces;
        // Preserve the effective behavior of older files. Before this key
        // existed, ax25_autonegotiation was the only switch controlling the
        // modulo-128 offer, so a missing key inherits that value.
        if !has_modulo_128_key {
            c.ax25_modulo_128 = c.ax25_autonegotiation;
        }
        Ok(c)
    }
}

pub fn encode_transport_endpoint(interface: &InterfaceConfig) -> String {
    match interface.transport {
        TransportKind::Tcp => interface.endpoint.clone(),
        TransportKind::Serial => format!("serial:{}@{}", interface.endpoint, interface.baud),
    }
}

/// Serialize the structured configuration into the portable Terminal format.
/// The web UI uses this canonical form when saving settings without exposing
/// the configuration file as the primary editing surface.
pub fn encode(config: &Config) -> String {
    let mut lines = vec![
        format!(
            "callsign={}",
            config
                .call
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default()
        ),
        format!("myname={}", config.myname),
        format!("locator={}", config.locator),
        format!("language={}", config.language),
        format!("welcome_enabled={}", config.welcome),
        format!("goodbye_enabled={}", config.goodbye),
        format!("remote_commands={}", config.remote_commands),
        format!("beacon_enabled={}", config.beacon),
        format!("beacon_seconds={}", config.beacon_seconds),
        format!("beacon_to={}", config.beacon_to),
        format!("digipeat_enabled={}", config.digipeat),
        format!("t1_ms={}", config.t1),
        format!("t3_ms={}", config.t3),
        format!("retries={}", config.n2),
        format!("paclen={}", config.paclen),
        format!("accept_incoming={}", config.accept),
        format!("ax25_autonegotiation={}", config.ax25_autonegotiation),
        format!("ax25_modulo_128={}", config.ax25_modulo_128),
        format!("mailbox_enabled={}", config.mailbox_enabled),
        format!(
            "mailbox_callsign={}",
            config
                .mailbox_call
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default()
        ),
        format!("yapp_enabled={}", config.yapp_enabled),
        format!("yapp_auto_receive={}", config.yapp_auto_receive),
        format!("yapp_ask_location={}", config.yapp_ask_location),
        format!("yapp_download_dir={}", config.yapp_download_dir),
        format!("yapp_max_files={}", config.yapp_max_files),
        format!("yapp_auto_cleanup={}", config.yapp_auto_cleanup),
        format!("telegram_enabled={}", config.telegram_enabled),
        format!("telegram_chat_id={}", config.telegram_chat_id),
        format!("telegram_token_file={}", config.telegram_token_file),
    ];
    lines.extend(config.notifications.encode_lines());
    for interface in &config.interfaces {
        lines.push(format!(
            "interface_{}={},{},{},{},{}",
            interface.name,
            encode_transport_endpoint(interface),
            interface.port,
            interface.priority,
            interface.enabled,
            interface.description
        ));
        let path = interface
            .beacon_path
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(format!(
            "beacon_interface_{}={},{}",
            interface.name, interface.beacon_enabled, path
        ));
    }
    if config.interfaces.is_empty() {
        lines.push(format!("tnc={}", config.endpoint));
        lines.push(format!("kiss_port={}", config.port));
    }
    lines.join("\n") + "\n"
}

fn parse_transport_endpoint(value: &str) -> Result<(TransportKind, String, u32), String> {
    if let Some(serial) = value.strip_prefix("serial:") {
        let (device, baud) = serial
            .rsplit_once('@')
            .ok_or("Serial interface must be serial:COMx@baud")?;
        if !is_serial_device_name(device) {
            return Err("Serial device must be a COM port name such as COM3".into());
        }
        let baud = baud
            .parse::<u32>()
            .map_err(|_| "Serial baud rate must be a supported number")?;
        if ![1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200].contains(&baud) {
            return Err("Serial baud rate must be 1200-115200 from the supported list".into());
        }
        return Ok((
            TransportKind::Serial,
            if device.starts_with("/dev/") {
                device.to_owned()
            } else {
                device.to_ascii_uppercase()
            },
            baud,
        ));
    }
    if !value
        .rsplit_once(':')
        .is_some_and(|(host, port)| !host.is_empty() && port.parse::<u16>().is_ok_and(|p| p != 0))
    {
        return Err("Interface endpoint must be host:port".into());
    }
    Ok((TransportKind::Tcp, value.to_string(), 9600))
}

fn is_serial_device_name(name: &str) -> bool {
    if name.starts_with("/dev/") {
        return name.len() > 5
            && !name.split('/').any(|p| p == ".." || p == ".")
            && !name.chars().any(char::is_whitespace);
    }
    let Some(number) = name.strip_prefix("COM") else {
        return false;
    };
    !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_locator(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !matches!(bytes.len(), 4 | 6) {
        return false;
    }
    if !bytes[0].to_ascii_uppercase().is_ascii_uppercase()
        || !bytes[1].to_ascii_uppercase().is_ascii_uppercase()
        || bytes[0].to_ascii_uppercase() > b'R'
        || bytes[1].to_ascii_uppercase() > b'R'
        || !bytes[2].is_ascii_digit()
        || !bytes[3].is_ascii_digit()
    {
        return false;
    }
    bytes.len() == 4
        || (bytes[4].to_ascii_lowercase().is_ascii_lowercase()
            && bytes[5].to_ascii_lowercase().is_ascii_lowercase()
            && bytes[4].to_ascii_lowercase() <= b'x'
            && bytes[5].to_ascii_lowercase() <= b'x')
}

fn valid_secret_filename(value: &str) -> bool {
    value.is_empty()
        || (value.len() <= 80
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            && value != "."
            && value != "..")
}

#[cfg(test)]
mod tests {
    use super::{Config, TransportKind, encode, encode_transport_endpoint};
    use std::{fs, path::PathBuf};

    #[test]
    fn legacy_font_size_config_is_accepted_but_has_no_application_effect() {
        assert!(Config::parse("callsign=TEST-1\nfont_size=10").is_ok());
        assert!(Config::parse("callsign=TEST-1\nfont_size=37").is_err());
    }

    #[test]
    fn structured_config_encoding_round_trips() {
        let source = "callsign=TEST-1\nmyname=Station\ninterface_VHF=127.0.0.1:8001,0,10,true,Local\nbeacon_interface_VHF=true,RELAY\n";
        let parsed = Config::parse(source).unwrap();
        let encoded = encode(&parsed);
        let round_trip = Config::parse(&encoded).unwrap();
        assert_eq!(parsed.call, round_trip.call);
        assert_eq!(parsed.interfaces, round_trip.interfaces);
        assert_eq!(parsed.beacon_seconds, round_trip.beacon_seconds);
    }

    #[test]
    fn beacon_interval_floor_is_ten_minutes_and_short_legacy_values_are_clamped() {
        assert_eq!(Config::fresh().beacon_seconds, 600);
        assert_eq!(
            Config::parse("beacon_seconds=600").unwrap().beacon_seconds,
            600
        );
        assert_eq!(
            Config::parse("beacon_seconds=60").unwrap().beacon_seconds,
            600,
            "legacy one-minute settings must not make startup beacon every minute"
        );
        assert_eq!(
            Config::parse("beacon_seconds=599").unwrap().beacon_seconds,
            600
        );
        assert!(Config::parse("beacon_seconds=9").is_err());
        assert!(Config::parse("beacon_seconds=86401").is_err());
    }

    #[test]
    fn mailbox_requires_a_distinct_callsign_when_enabled() {
        assert!(!Config::fresh().mailbox_enabled);
        let config =
            Config::parse("callsign=TEST-1\nmailbox_enabled=true\nmailbox_callsign=TEST-2")
                .unwrap();
        assert!(config.mailbox_enabled);
        assert_eq!(config.mailbox_call.unwrap().to_string(), "TEST-2");
        assert!(
            !Config::parse("callsign=TEST-1\nmailbox_enabled=false")
                .unwrap()
                .mailbox_enabled
        );
        assert!(Config::parse("callsign=TEST-1\nmailbox_enabled=true").is_err());
        assert!(Config::parse("callsign=TEST-1\nmailbox_callsign=TEST-1").is_err());
    }

    #[test]
    fn yapp_and_automatic_receive_are_off_by_default_and_configurable() {
        let fresh = Config::fresh();
        assert!(!fresh.yapp_enabled);
        assert!(!fresh.yapp_auto_receive);
        assert!(!fresh.yapp_ask_location);
        assert!(fresh.yapp_download_dir.is_empty());
        let config = Config::parse(
            "callsign=TEST-1\nyapp_enabled=true\nyapp_auto_receive=true\nyapp_ask_location=true\nyapp_download_dir=C:/Radio/Files",
        )
        .unwrap();
        assert!(config.yapp_enabled && config.yapp_auto_receive && config.yapp_ask_location);
        assert_eq!(config.yapp_download_dir, "C:/Radio/Files");
    }

    #[test]
    fn retired_module_settings_are_ignored_and_not_encoded() {
        let source = "callsign=TEST-1\nmailbox_enabled=true\nmailbox_callsign=TEST-2\nwl2k_enabled=true\nwl2k_call=INVALID!\nwl2k_gateway=\nwl2k_interface=REMOVED\nwl2k_via=IGNORED\nwl2k_sync_hours=bad\nwl2k_auto_sync=true\nwl2k_locator=invalid\nwl2k_password_file=../unused\nnotification_winlink_in-app=true\nnotification_winlink_in_app=false\nnotification_winlink_native=true\nnotification_winlink_telegram=true\nnotification_winlink_sound=2\nnotification_mailbox_sound=3\n";
        let parsed = Config::parse(source).unwrap();
        assert!(parsed.mailbox_enabled);
        assert_eq!(parsed.notifications.mailbox.sound, 3);
        let encoded = encode(&parsed);
        assert!(!encoded.contains("wl2k"));
        assert!(!encoded.contains("winlink"));
        assert!(Config::parse(&encoded).unwrap().mailbox_enabled);
        assert!(Config::parse("wl2k_unknown=true\n").is_err());
        assert!(Config::parse("notification_winlink_unknown=true\n").is_err());
    }

    #[test]
    fn telegram_settings_round_trip_without_token() {
        let source = "callsign=TEST-1\ninterface_VHF=127.0.0.1:8001,0,0,true,Radio\ntelegram_enabled=true\ntelegram_chat_id=@upr_station\nnotification_mailbox_telegram=true\n";
        let parsed = Config::parse(source).unwrap();
        assert!(parsed.telegram_enabled);
        assert_eq!(parsed.telegram_chat_id, "@upr_station");
        assert!(parsed.notifications.mailbox.telegram);
        let encoded = encode(&parsed);
        assert!(!encoded.contains("telegram.bot-token="));
        let round_trip = Config::parse(&encoded).unwrap();
        assert_eq!(round_trip.telegram_chat_id, "@upr_station");
        assert!(round_trip.notifications.mailbox.telegram);
    }

    #[test]
    fn legacy_notification_in_app_key_round_trips_through_config_parser() {
        let source = "notification_application_started_in-app=true\n";
        let parsed = Config::parse(source).unwrap();
        assert!(parsed.notifications.application_started.in_app);
        let encoded = encode(&parsed);
        assert!(encoded.contains("notification_application_started_in_app=true"));
        assert!(!encoded.contains("notification_application_started_in-app="));
    }

    #[test]
    fn station_locator_round_trips_and_rejects_invalid_values() {
        let parsed = Config::parse("callsign=TEST-1\nlocator=jo92aa\n").unwrap();
        assert_eq!(parsed.locator, "JO92AA");
        assert_eq!(Config::parse(&encode(&parsed)).unwrap().locator, "JO92AA");
        assert!(Config::parse("locator=INVALID").is_err());
    }

    #[test]
    fn missing_config_starts_with_no_station_or_interface() {
        let path = PathBuf::from(format!("target/missing-config-{}.conf", std::process::id()));
        let _ = fs::remove_file(&path);
        let config = Config::load_or_fresh(&path).unwrap();
        assert!(config.call.is_none());
        assert!(config.interfaces.is_empty());
    }

    #[test]
    fn legacy_interfaces_remain_tcp_and_serial_interfaces_round_trip() {
        let tcp = Config::parse("callsign=TEST-1\ninterface_NET=127.0.0.1:8001,0,0,true,Network")
            .unwrap();
        assert_eq!(tcp.interfaces[0].transport, TransportKind::Tcp);
        assert_eq!(tcp.interfaces[0].endpoint, "127.0.0.1:8001");

        let serial =
            Config::parse("callsign=TEST-1\ninterface_VHF=serial:COM12@19200,0,0,true,Radio")
                .unwrap();
        assert_eq!(serial.interfaces[0].transport, TransportKind::Serial);
        assert_eq!(serial.interfaces[0].endpoint, "COM12");
        assert_eq!(serial.interfaces[0].baud, 19200);
        assert_eq!(
            encode_transport_endpoint(&serial.interfaces[0]),
            "serial:COM12@19200"
        );
    }

    #[test]
    fn serial_interfaces_reject_invalid_device_names_and_speeds() {
        for endpoint in ["serial:/dev/../etc/passwd@9600", "serial:COM1@12345"] {
            let text = format!("callsign=TEST-1\ninterface_VHF={endpoint},0,0,true,Radio");
            assert!(Config::parse(&text).is_err(), "accepted {endpoint}");
        }
    }

    #[test]
    fn linux_serial_preserves_device_case_and_round_trips() {
        let c =
            Config::parse("callsign=TEST-1\ninterface_VHF=serial:/dev/ttyUSB0@9600,0,0,true,Radio")
                .unwrap();
        assert_eq!(c.interfaces[0].endpoint, "/dev/ttyUSB0");
        assert_eq!(
            encode_transport_endpoint(&c.interfaces[0]),
            "serial:/dev/ttyUSB0@9600"
        );
    }

    #[test]
    fn missing_modulo_128_key_inherits_legacy_autonegotiation() {
        let legacy_off = Config::parse("ax25_autonegotiation=false\n").unwrap();
        assert!(!legacy_off.ax25_modulo_128);
        let legacy_on = Config::parse("ax25_autonegotiation=true\n").unwrap();
        assert!(legacy_on.ax25_modulo_128);
        let explicit = Config::parse(
            "ax25_autonegotiation=true\nax25_modulo_128=false\n",
        )
        .unwrap();
        assert!(!explicit.ax25_modulo_128);
    }

    #[test]
    fn legacy_config_and_sidecar_files_are_copied_without_removing_originals() {
        let root = PathBuf::from(format!("target/config-migration-{}", std::process::id()));
        let legacy_dir = root.join("legacy");
        let new_dir = root.join("user");
        let legacy = legacy_dir.join("terminal.conf");
        let destination = new_dir.join("terminal.conf");
        fs::create_dir_all(legacy_dir.join("messages")).unwrap();
        fs::create_dir_all(legacy_dir.join("languages")).unwrap();
        fs::write(&legacy, "callsign=TEST-1\n").unwrap();
        fs::write(legacy_dir.join("messages/welcome.txt"), "hello").unwrap();
        fs::write(legacy_dir.join("languages/xx.lang"), "Connected\tOK\n").unwrap();

        assert!(Config::migrate_legacy(&destination, &legacy).unwrap());
        assert_eq!(fs::read_to_string(&legacy).unwrap(), "callsign=TEST-1\n");
        assert_eq!(
            fs::read_to_string(&destination).unwrap(),
            "callsign=TEST-1\n"
        );
        assert_eq!(
            fs::read_to_string(new_dir.join("messages/welcome.txt")).unwrap(),
            "hello"
        );
        assert_eq!(
            fs::read_to_string(new_dir.join("languages/xx.lang")).unwrap(),
            "Connected\tOK\n"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn setup_marker_prevents_legacy_migration_after_configuration_reset() {
        let root = crate::test_temp_dir("ultimatepr-reset-test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let legacy = root.join("legacy.conf");
        let destination = root.join("user").join("terminal.conf");
        fs::write(&legacy, "callsign=OLD-1\n").unwrap();

        Config::mark_setup_required(&destination).unwrap();
        assert!(!Config::migrate_legacy(&destination, &legacy).unwrap());
        assert!(!destination.exists());

        Config::clear_setup_required(&destination).unwrap();
        assert!(Config::migrate_legacy(&destination, &legacy).unwrap());
        fs::remove_dir_all(root).unwrap();
    }
}
