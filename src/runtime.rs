//! Shared station lifecycle. No GUI dependencies.
use crate::{http, station, storage, web_config};
use std::{
    env, fs,
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

const INITIAL: &str = "# UltimatePR Station - configure callsign and TNC before radio operation\n# callsign=N0CALL-1\nmyname=\nlanguage=en\naccept_incoming=true\nax25_autonegotiation=true\nax25_modulo_128=true\npaclen=128\nt1_ms=5000\nt3_ms=180000\nretries=10\ndigipeat_enabled=false\nbeacon_enabled=false\nbeacon_seconds=600\nbeacon_to=BEACON\nwelcome_enabled=true\ngoodbye_enabled=true\nremote_commands=true\nmailbox_enabled=false\nyapp_enabled=false\nyapp_auto_receive=false\nyapp_max_files=1000\nyapp_auto_cleanup=false\n";

#[derive(Clone)]
pub struct RuntimeOptions {
    pub address: String,
    pub config: PathBuf,
    pub web_config: PathBuf,
    pub web_config_explicit: bool,
    pub desktop: bool,
    /// Deprecated compatibility field. Desktop no longer authenticates via
    /// an injected token, but older callers may still populate it.
    pub desktop_token: Option<String>,
    pub install_signals: bool,
}
impl RuntimeOptions {
    pub fn from_cli(args: impl IntoIterator<Item = String>) -> Result<Option<Self>, String> {
        let mut address = "127.0.0.1:8080".to_owned();
        let mut path = PathBuf::from("data/station.conf");
        let mut web_config_path = PathBuf::from("data/web.conf");
        let mut web_config_explicit = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--bind" => address = args.next().ok_or("Missing bind address")?,
                "--config" => path = PathBuf::from(args.next().ok_or("Missing config path")?),
                "--web-config" => {
                    web_config_path = PathBuf::from(args.next().ok_or("Missing web config path")?);
                    web_config_explicit = true;
                }
                "--help" | "-h" => {
                    println!(
                        "UltimatePR Station {}\n  --bind IP:PORT      default 127.0.0.1:8080\n  --config PATH       default data/station.conf\n  --web-config PATH   default data/web.conf\nServer access uses the operator account (default admin/packet on first run); STATION_TOKEN or token_file remain supported for API compatibility.\nTerminal-compatible configuration; no radio transmission before callsign setup.",
                        env!("CARGO_PKG_VERSION")
                    );
                    return Ok(None);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }

        Ok(Some(Self {
            address,
            config: path,
            web_config: web_config_path,
            web_config_explicit,
            desktop: false,
            desktop_token: None,
            install_signals: true,
        }))
    }
}

pub struct StationRuntime {
    pub address: SocketAddr,
    stop: Arc<AtomicBool>,
    coordinator: Option<thread::JoinHandle<Result<(), String>>>,
}
impl StationRuntime {
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
    pub fn is_finished(&self) -> bool {
        self.coordinator.as_ref().is_none_or(|t| t.is_finished())
    }
    pub fn wait(mut self) -> Result<(), String> {
        self.coordinator
            .take()
            .unwrap()
            .join()
            .map_err(|_| "Station coordinator panicked".to_owned())?
    }
}
impl Drop for StationRuntime {
    fn drop(&mut self) {
        self.request_stop();
        if let Some(worker) = self.coordinator.take() {
            let _ = worker.join();
        }
    }
}
struct StopOnDrop(Arc<AtomicBool>);
impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

pub fn start(
    options: RuntimeOptions,
    events: Option<mpsc::SyncSender<crate::station_events::StationEvent>>,
) -> Result<StationRuntime, String> {
    let RuntimeOptions {
        mut address,
        config: mut path,
        web_config: web_config_path,
        web_config_explicit,
        desktop,
        desktop_token,
        install_signals,
    } = options;
    if let Some(parent) = web_config_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut web_config = web_config::WebConfig::load(&web_config_path)?;
    if !web_config_path.exists() {
        storage::write_atomic(&web_config_path, web_config.encode().as_bytes())
            .map_err(|e| format!("Cannot create {}: {e}", web_config_path.display()))?;
    }
    if address == "127.0.0.1:8080" {
        address = web_config.bind.to_string();
    }
    let address: SocketAddr = address.parse().map_err(|_| "Invalid bind address")?;
    let token = if let Some(value) = desktop_token.clone() {
        value
    } else if let Ok(value) = env::var("STATION_TOKEN") {
        value
    } else if let Some(token_path) = web_config.token_file.as_ref() {
        let path = if token_path.is_absolute() {
            token_path.clone()
        } else {
            web_config_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join(token_path)
        };
        match fs::read_to_string(path) {
            Ok(value) => value.trim().to_owned(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.to_string()),
        }
    } else {
        String::new()
    };
    // Server accounts protect the web API when no legacy bearer token is configured.
    if !token.is_ascii()
        || token
            .bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
    {
        return Err("STATION_TOKEN must be printable ASCII without spaces".into());
    }
    let listener = TcpListener::bind(address).map_err(|e| format!("HTTP bind: {e}"))?;
    let address = listener.local_addr().map_err(|e| e.to_string())?;
    if !path.is_absolute() {
        path = env::current_dir().map_err(|e| e.to_string())?.join(path);
    }
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let _lock = DataLock::acquire(path.parent().unwrap().join(".station.lock"))?;
    if !path.exists() {
        storage::write_atomic(&path, INITIAL.as_bytes()).map_err(|e| e.to_string())?;
    }
    let station_web_path = if web_config_explicit {
        web_config_path.clone()
    } else {
        path.with_file_name("web.conf")
    };
    let auth_path = station_web_path.with_file_name("operator.account");
    if !web_config.auth_configured {
        // Older server installations protected the panel whenever an account
        // already existed. Desktop profiles historically used a local token;
        // keep those profiles open while making new installations open by
        // default as well.
        web_config.auth_enabled = desktop_token.is_some() || (!desktop && auth_path.exists());
        storage::write_atomic(&web_config_path, web_config.encode().as_bytes())
            .map_err(|e| format!("Cannot migrate {}: {e}", web_config_path.display()))?;
    }
    let auth = std::sync::Arc::new(std::sync::Mutex::new(crate::auth::Auth::load(auth_path)?));
    let runtime = if desktop { "desktop" } else { "server" };
    let mut station = station::Station::new_with_web_config(
        path.clone(),
        Some(station_web_path.clone()),
        runtime,
    )?;
    let mqtt_config = web_config::WebConfig::load(&station_web_path)?;
    let mqtt_password_path = station_web_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("mqtt.password");
    let mqtt_password = storage::read_secret(&mqtt_password_path)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_default();
    let mqtt = match crate::mqtt::Publisher::start(
        &mqtt_config,
        mqtt_password,
        station.mqtt_callsign(),
    ) {
        Ok(publisher) => publisher,
        Err(error) => {
            eprintln!("MQTT disabled: {error}");
            None
        }
    };
    let (mqtt_discovery_sender, mqtt_discovery_receiver) = mpsc::sync_channel::<()>(1);
    station.set_mqtt_discovery_sender(mqtt.is_some().then_some(mqtt_discovery_sender));
    let files_directory = station.files_directory();
    let stop = Arc::new(AtomicBool::new(false));
    if install_signals {
        #[cfg(windows)]
        {
            WINDOWS_STOP
                .set(stop.clone())
                .map_err(|_| "Signal handler already registered")?;
            if unsafe {
                windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(console_signal), 1)
            } == 0
            {
                return Err("Cannot register console signal handler".into());
            }
        }
        #[cfg(unix)]
        for signal in [signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT] {
            signal_hook::flag::register(signal, stop.clone()).map_err(|e| e.to_string())?;
        }
    }
    let snapshot = Arc::new(RwLock::new(station.snapshot()));
    let (tx, rx) = mpsc::sync_channel::<http::Envelope>(8);
    let (telegram_sender, telegram_worker) = if desktop {
        (None, None)
    } else {
        let (sender, worker) =
            crate::telegram::spawn_worker(stop.clone(), path.with_file_name("telegram.log"));
        (Some(sender), Some(worker))
    };
    let runtime_stop = stop.clone();
    let runtime_snapshot = snapshot.clone();
    station.set_events(events);
    station.set_telegram_sender(telegram_sender);
    station.notify_application_started();
    let runtime = thread::spawn(move || {
        let _stop_on_exit = StopOnDrop(runtime_stop.clone());
        let mut published = Instant::now();
        let mut mqtt_published = Instant::now() - Duration::from_secs(2);
        let mqtt = mqtt;
        let mut shutdown = None;
        loop {
            if runtime_stop.load(Ordering::Relaxed) && shutdown.is_none() {
                station.shutdown();
                shutdown = Some(Instant::now());
            }
            for _ in 0..8 {
                match rx.try_recv() {
                    Ok((request, reply)) => {
                        let response = station.command(request);
                        let _ = reply.send(response);
                    }
                    Err(_) => break,
                }
            }
            station.tick();
            if mqtt_discovery_receiver.try_recv().is_ok() {
                if let Some(publisher) = &mqtt {
                    publisher.publish(station.mqtt_messages(
                        &mqtt_config.mqtt_topic_prefix,
                        true,
                        &mqtt_config.mqtt_discovery_topic,
                    ));
                }
            }
            if published.elapsed() >= Duration::from_millis(500) {
                station.notify_mail();
                *runtime_snapshot.write().unwrap() = station.snapshot();
                published = Instant::now();
            }
            if mqtt_published.elapsed() >= Duration::from_secs(2) {
                if let Some(publisher) = &mqtt {
                    publisher.publish(station.mqtt_messages(
                        &mqtt_config.mqtt_topic_prefix,
                        false,
                        &mqtt_config.mqtt_discovery_topic,
                    ));
                }
                mqtt_published = Instant::now();
            }
            if let Some(start) = shutdown
                && (!station.active() || start.elapsed() > Duration::from_secs(12))
            {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
    });
    let server_stop = stop.clone();
    let coordinator = thread::spawn(move || {
        let _lock = _lock;
        let stop = server_stop;
        let result = http::Server {
            address,
            token,
            auth_enabled: web_config.auth_enabled,
            web_config: web_config_path.clone(),
            snapshot,
            commands: tx,
            stop: stop.clone(),
            files: files_directory,
            auth,
        }
        .run(listener);
        stop.store(true, Ordering::Relaxed);
        let runtime_result = runtime.join();
        if let Some(worker) = telegram_worker {
            let _ = worker.join();
        }
        runtime_result.map_err(|_| "Station runtime panicked".to_owned())?;
        result.map_err(|e| e.to_string())
    });
    Ok(StationRuntime {
        address,
        stop,
        coordinator: Some(coordinator),
    })
}
struct DataLock {
    _file: fs::File,
}
impl DataLock {
    fn acquire(path: PathBuf) -> Result<Self, String> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0);
        }
        let file = options.open(&path).map_err(|e| {
            format!(
                "Cannot lock configuration {} (another Station may be running): {e}",
                path.display()
            )
        })?;
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } < 0 {
                return Err("Another Station is using this configuration".into());
            }
        }
        Ok(Self { _file: file })
    }
}
#[cfg(windows)]
static WINDOWS_STOP: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();
#[cfg(windows)]
unsafe extern "system" fn console_signal(_: u32) -> i32 {
    if let Some(stop) = WINDOWS_STOP.get() {
        stop.store(true, Ordering::Relaxed);
    }
    1
}
