#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod autostart;
mod settings;

use settings::{Options, Preferences};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};
use tauri::{
    Manager,
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
    tray::{TrayIconBuilder, TrayIconEvent},
    webview::NewWindowResponse,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use ultimatepr::{
    runtime::{self, RuntimeOptions, StationRuntime},
    station_events::StationEvent,
};

struct Desktop {
    runtime: Mutex<Option<StationRuntime>>,
    preferences: Mutex<Preferences>,
    checks: Mutex<HashMap<String, CheckMenuItem<tauri::Wry>>>,
    options: Options,
    closing: AtomicBool,
    exit_ready: AtomicBool,
    tray_ready: AtomicBool,
    close_dialog: AtomicBool,
}
fn log(path: &Path, message: &str) {
    // Bound local diagnostics; never log API tokens or message contents.
    if fs::metadata(path).is_ok_and(|m| m.len() > 1024 * 1024) {
        let _ = fs::rename(path, path.with_extension("previous.log"));
    }
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!(file, "{seconds}: {message}");
    }
}
fn report(app: &tauri::AppHandle, message: &str) {
    let state = app.state::<Arc<Desktop>>();
    log(&state.options.data.join("desktop.log"), message);
    app.dialog()
        .message(message)
        .title("UltimatePR Station")
        .show(|_| {});
}
fn reveal(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
fn shutdown(app: &tauri::AppHandle) {
    let state = app.state::<Arc<Desktop>>().inner().clone();
    if state.closing.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title("UltimatePR Station — kończenie pracy…");
    }
    let handle = app.clone();
    thread::spawn(move || {
        let station = state.runtime.lock().unwrap().take();
        if let Some(station) = station {
            station.request_stop();
            if let Err(e) = station.wait() {
                log(&state.options.data.join("desktop.log"), &e);
            }
        }
        state.exit_ready.store(true, Ordering::SeqCst);
        handle.exit(0);
    });
}
#[cfg(windows)]
fn tray_host_available() -> bool {
    true
}
#[cfg(target_os = "linux")]
fn tray_host_available() -> bool {
    // A successful tray build is insufficient on GNOME without an indicator host.
    let Ok(builder) = zbus::blocking::connection::Builder::session() else {
        return false;
    };
    let Ok(connection) = builder.method_timeout(Duration::from_secs(1)).build() else {
        return false;
    };
    let Ok(proxy) = zbus::blocking::Proxy::new(
        &connection,
        "org.kde.StatusNotifierWatcher",
        "/StatusNotifierWatcher",
        "org.kde.StatusNotifierWatcher",
    ) else {
        return false;
    };
    proxy
        .get_property::<bool>("IsStatusNotifierHostRegistered")
        .unwrap_or(false)
}
fn handle_menu(app: &tauri::AppHandle, id: &str) {
    match id {
        "open" => {
            reveal(app);
            return;
        }
        "quit" => {
            shutdown(app);
            return;
        }
        "data" => {
            let state = app.state::<Arc<Desktop>>();
            if let Err(e) = app
                .opener()
                .open_path(state.options.data.to_string_lossy(), None::<&str>)
            {
                report(app, &e.to_string());
            }
            return;
        }
        "help" => {
            app.dialog()
                .message(include_str!("../../docs/DEPLOYMENT.md"))
                .title("UltimatePR Station — instrukcja offline")
                .show(|_| {});
            return;
        }
        _ => {}
    }
    let state = app.state::<Arc<Desktop>>();
    let mut prefs = state.preferences.lock().unwrap();
    let old = prefs.clone();
    let value = match id {
        "autostart" => &mut prefs.autostart,
        "connections" => &mut prefs.connections,
        "mail" => &mut prefs.mail,
        "files" => &mut prefs.files,
        "tnc" => &mut prefs.tnc,
        "dnd" => {
            let path = &state.options.web_config;
            let mut config = match ultimatepr::web_config::WebConfig::load(path) {
                Ok(config) => config,
                Err(error) => { report(app, &error); return; }
            };
            config.presence_mode = if config.presence_mode == "dnd" { "auto".into() } else { "dnd".into() };
            if let Err(error) = ultimatepr::storage::write_atomic(path, config.encode().as_bytes()) {
                report(app, &error.to_string());
                return;
            }
            if let Some(item) = state.checks.lock().unwrap().get("dnd") { let _ = item.set_checked(config.presence_mode == "dnd"); }
            return;
        }
        _ => return,
    };
    *value = !*value;
    let checked = *value;
    let result = (|| {
        if id == "autostart" {
            autostart::set(
                prefs.autostart,
                &state.options.config,
                &state.options.web_config,
            )?;
        }
        if let Err(e) = prefs.save(&state.options.data.join("desktop.conf")) {
            if id == "autostart" {
                let _ = autostart::set(
                    old.autostart,
                    &state.options.config,
                    &state.options.web_config,
                );
            }
            return Err(e);
        }
        Ok::<_, String>(())
    })();
    let applied = if let Err(e) = result {
        *prefs = old;
        drop(prefs);
        report(app, &e);
        !checked
    } else {
        drop(prefs);
        checked
    };
    if let Some(item) = state.checks.lock().unwrap().get(id) {
        let _ = item.set_checked(applied);
    }
}
fn notifications(
    app: tauri::AppHandle,
    state: Arc<Desktop>,
    events: mpsc::Receiver<StationEvent>,
    tray_created: bool,
) {
    thread::spawn(move || {
        let mut passes = 0;
        while !state.closing.load(Ordering::Relaxed) {
            if passes % 20 == 0 {
                let ready = tray_created && tray_host_available();
                if state.tray_ready.swap(ready, Ordering::Relaxed) && !ready {
                    reveal(&app);
                }
            }
            passes += 1;
            let finished = state
                .runtime
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(StationRuntime::is_finished);
            if finished {
                reveal(&app);
                report(
                    &app,
                    "Backend stacji zakończył pracę. Sprawdź desktop.log i uruchom aplikację ponownie.",
                );
                shutdown(&app);
                break;
            }
            for event in events.try_iter().take(64) {
                match event {
                    StationEvent::Notification(notification) => {
                        if let Err(e) = app
                            .notification()
                            .builder()
                            .title(&notification.title)
                            .body(&notification.body)
                            .show()
                        {
                            log(&state.options.data.join("desktop.log"), &format!("Notification: {e}"));
                        }
                        play_notification_sound(notification.sound);
                    }
                    StationEvent::Sound(count) => play_notification_sound(count),
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
}
fn play_notification_sound(count: u8) {
    if count == 0 { return; }
    #[cfg(windows)]
    {
        unsafe extern "system" {
            fn Beep(frequency: u32, duration: u32) -> i32;
            fn MessageBeep(kind: u32) -> i32;
        }
        for index in 0..count {
            let frequency = 760 + u32::from(index) * 100;
            if unsafe { Beep(frequency, 320) } == 0 {
                unsafe { MessageBeep(0xFFFFFFFF); }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        for _ in 0..count {
            let _ = std::process::Command::new("canberra-gtk-play")
                .args(["-i", "message-new-instant"])
                .status();
        }
    }
}
fn setup(app: &mut tauri::App, state: &Arc<Desktop>) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(&state.options.data)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&state.options.data, fs::Permissions::from_mode(0o700))?;
    }
    let (sender, receiver) = mpsc::sync_channel(64);
    let station = runtime::start(
        RuntimeOptions {
            address: "127.0.0.1:0".into(),
            config: state.options.config.clone(),
            web_config: state.options.web_config.clone(),
            web_config_explicit: true,
            demo: state.options.demo,
            desktop: true,
            desktop_token: None,
            install_signals: false,
        },
        Some(sender),
    )?;
    let url = format!("http://{}", station.address);
    *state.runtime.lock().unwrap() = Some(station);
    let prefs = state.preferences.lock().unwrap().clone();
    prefs.save(&state.options.data.join("desktop.conf"))?;
    // Do not modify autostart registration on ordinary launch. Only a user menu action does so.
    let open = MenuItem::with_id(app, "open", "Otwórz", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Zakończ", true, None::<&str>)?;
    let data = MenuItem::with_id(app, "data", "Katalog danych", true, None::<&str>)?;
    let help = MenuItem::with_id(app, "help", "Instrukcja offline", true, None::<&str>)?;
    let mut checks = state.checks.lock().unwrap();
    for (id, title, checked) in [
        ("autostart", "Start po zalogowaniu", prefs.autostart),
        ("dnd", "Nie przeszkadzać", false),
    ] {
        checks.insert(
            id.into(),
            CheckMenuItem::with_id(
                app,
                id,
                title,
                id != "autostart" || !state.options.demo,
                checked,
                None::<&str>,
            )?,
        );
    }
    let items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = vec![
        &open,
        &checks["autostart"],
        &data,
        &help,
        &quit,
    ];
    let tray_menu = Menu::with_items(app, &items)?;
    let submenu = Submenu::with_items(app, "Stacja", true, &items)?;
    let menu = Menu::with_items(app, &[&submenu])?;
    drop(checks);
    app.set_menu(menu)?;
    let tray = TrayIconBuilder::with_id("station")
        .icon(
            app.default_window_icon()
                .ok_or("Application icon missing")?
                .clone(),
        )
        .tooltip("UltimatePR Station")
        .menu(&tray_menu)
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                reveal(tray.app_handle());
            }
        })
        .build(app);
    let tray_created = match tray {
        Ok(_) => true,
        Err(e) => {
            log(
                &state.options.data.join("desktop.log"),
                &format!("Tray: {e}"),
            );
            false
        }
    };
    let tray_ready = tray_created && tray_host_available();
    state.tray_ready.store(tray_ready, Ordering::Relaxed);
    let origin = url.clone();
    let opener = app.handle().clone();
    let new_window_app = app.handle().clone();
    let script = format!(
        "window.__UPR_DESKTOP__=true;if(location.origin!=={}){{window.location.replace({});}}",
        serde_json::to_string(&url)?,
        serde_json::to_string(&url)?
    );
    tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::External(url.parse()?))
        .title(if state.options.demo {
            "UltimatePR Station — DEMO"
        } else {
            "UltimatePR Station"
        })
        .inner_size(1280.0, 850.0)
        .min_inner_size(800.0, 600.0)
        .visible(!(state.options.minimized && tray_ready))
        .data_directory(state.options.data.join("webview"))
        .initialization_script(script)
        .on_navigation(move |target| {
            if target.origin().ascii_serialization() == origin {
                return true;
            }
            if matches!(target.scheme(), "https" | "http") {
                let _ = opener.opener().open_url(target.as_str(), None::<&str>);
            }
            false
        })
        .on_new_window(move |target, _| {
            if matches!(target.scheme(), "https" | "http") {
                let _ = new_window_app
                    .opener()
                    .open_url(target.as_str(), None::<&str>);
            }
            NewWindowResponse::Deny
        })
        .build()?;

    log(
        &state.options.data.join("desktop.log"),
        "Desktop backend ready (loopback only)",
    );
    notifications(app.handle().clone(), state.clone(), receiver, tray_created);
    Ok(())
}
fn run(options: Options) -> Result<(), String> {
    let preferences = Preferences::load(&options.data.join("desktop.conf"))?;
    let state = Arc::new(Desktop {
        runtime: Mutex::new(None),
        preferences: Mutex::new(preferences),
        checks: Mutex::new(HashMap::new()),
        options,
        closing: AtomicBool::new(false),
        exit_ready: AtomicBool::new(false),
        tray_ready: AtomicBool::new(false),
        close_dialog: AtomicBool::new(false),
    });
    let setup_state = state.clone();
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            reveal(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state.clone())
        .on_menu_event(|app, event| handle_menu(app, event.id().as_ref()))
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let state = app.state::<Arc<Desktop>>().inner().clone();
                if state.exit_ready.load(Ordering::Relaxed) {
                    return;
                }
                api.prevent_close();
                if state.closing.load(Ordering::Relaxed) {
                    return;
                }
                if state.tray_ready.load(Ordering::Relaxed) {
                    let _ = window.hide();
                } else if !state.close_dialog.swap(true, Ordering::Relaxed) {
                    let handle = app.clone();
                    app.dialog()
                        .message("Zasobnik jest niedostępny. Zakończyć pracę stacji?")
                        .title("UltimatePR Station")
                        .buttons(MessageDialogButtons::OkCancelCustom(
                            "Zakończ".into(),
                            "Pozostaw otwarte".into(),
                        ))
                        .show(move |yes| {
                            state.close_dialog.store(false, Ordering::Relaxed);
                            if yes {
                                shutdown(&handle);
                            }
                        });
                }
            }
        })
        .setup(move |app| setup(app, &setup_state))
        .build(tauri::generate_context!());
    match result {
        Ok(app) => app.run(|handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event
                && !handle
                    .state::<Arc<Desktop>>()
                    .exit_ready
                    .load(Ordering::Relaxed)
            {
                api.prevent_exit();
                shutdown(handle);
            }
        }),
        Err(e) => {
            if let Some(station) = state.runtime.lock().unwrap().take() {
                station.request_stop();
                let _ = station.wait();
            }
            return Err(e.to_string());
        }
    }
    Ok(())
}
fn run_server_mode(options: &Options) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let station = runtime::start(
            RuntimeOptions {
                address: "127.0.0.1:8080".into(),
                config: options.config.clone(),
                web_config: options.web_config.clone(),
                web_config_explicit: true,
                demo: options.demo,
                desktop: false,
                desktop_token: None,
                install_signals: true,
            },
            None,
        )?;
        eprintln!("UltimatePR Station server: http://{}", station.address);
        return station.wait();
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = options;
        Err("Tryb server jest dostępny tylko w wariancie Linux.".into())
    }
}
fn main() {
    let options = match Options::parse() {
        Ok(Some(o)) => o,
        Ok(None) => return,
        Err(e) => {
            startup_error(None, &e);
            return;
        }
    };
    if options.server_mode {
        let log_path = options.data.join("server.log");
        if let Err(e) = run_server_mode(&options) {
            startup_error(Some(log_path), &e);
        }
        return;
    }
    let log_path = options.data.join("desktop.log");
    if let Err(e) = run(options) {
        startup_error(Some(log_path), &e);
    }
}
fn startup_error(path: Option<PathBuf>, message: &str) {
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        log(&path, message);
    }
    eprintln!("UltimatePR Station desktop: {message}");
    rfd::MessageDialog::new()
        .set_title("UltimatePR Station — błąd uruchomienia")
        .set_description(message)
        .set_level(rfd::MessageLevel::Error)
        .show();
}
