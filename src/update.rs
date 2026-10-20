use crate::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use std::process::Command;
use std::sync::Mutex;

// One producer at a time, including the asynchronous GitHub download. The root
// helper additionally shares the native installer's cross-process lock.
static OPERATION: Mutex<()> = Mutex::new(());

pub const MAX_UPLOAD: usize = 128 * 1024 * 1024;

pub fn directory(config_path: &Path) -> PathBuf {
    std::env::var_os("UPR_UPDATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| if config_path == Path::new("/etc/ultimatepr/station.conf") {
            PathBuf::from("/var/lib/ultimatepr/update")
        } else { config_path.parent().unwrap_or(Path::new(".")).join("update") })
}

fn status_path(root: &Path) -> PathBuf { root.join("status.txt") }
fn package_path(root: &Path) -> PathBuf { root.join("incoming.zip") }

pub fn architecture() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        "arm" => "armv7",
        _ => "unsupported",
    }
}

pub fn build_info() -> String {
    json::object([
        ("version", json::string(env!("CARGO_PKG_VERSION"))),
        ("tag", json::string(option_env!("UPR_BUILD_TAG").unwrap_or("local"))),
        ("commit", json::string(option_env!("UPR_BUILD_COMMIT").unwrap_or("unknown"))),
        ("builtAt", option_env!("UPR_BUILD_EPOCH").and_then(|s| s.parse::<u64>().ok()).unwrap_or(0).to_string()),
        ("architecture", json::string(architecture())),
        ("webUpdate", if supported() { "true" } else { "false" }.into()),
    ])
}

pub fn supported() -> bool {
    cfg!(target_os = "linux") && Path::new("/usr/local/libexec/ultimatepr-update").is_file() && (
        Path::new("/etc/init.d/ultimatepr-station").is_file()
        || Path::new("/etc/systemd/system/ultimatepr-update.path").is_file()
    ) && !Path::new("/.dockerenv").exists()
}

fn ensure_idle(root: &Path) -> Result<(), String> {
    let text = fs::read_to_string(status_path(root)).unwrap_or_default();
    let state = text.lines().find_map(|l| l.strip_prefix("state=")).unwrap_or("idle");
    if matches!(state, "downloading" | "running" | "validating" | "building" | "stopping" | "starting") {
        return Err("Aktualizacja jest już uruchomiona. Sprawdź jej status.".into());
    }
    Ok(())
}

fn python(script: &str, args: &[&str]) -> Result<String, String> {
    let result = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .args(["-I", "-c", script]).args(args).output()
        .map_err(|e| format!("Wymagany Python 3 (składnik instalatora Station): {e}"))?;
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).trim().to_owned());
    }
    String::from_utf8(result.stdout).map_err(|e| e.to_string())
}

pub fn check(channel: &str) -> Result<String, String> {
    if !matches!(channel, "stable" | "test") { return Err("Nieprawidłowy kanał aktualizacji".into()); }
    python(include_str!("../deploy/server/update-release.py"), &["check", architecture(), channel])
}

pub fn download(root: &Path, tag: &str) -> Result<String, String> {
    if !supported() { return Err("Aktualizacja tej instalacji wymaga instalatora na hoście. Docker nie jest aktualizowany z panelu.".into()); }
    if tag.is_empty() || tag.len() > 101 || !tag.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)) {
        return Err("Nieprawidłowy identyfikator wydania".into());
    }
    let guard = OPERATION.try_lock().map_err(|_| "Inna operacja aktualizacji jest w toku")?;
    ensure_idle(root)?;
    write_status(root, "downloading", tag, "Pobieranie i sprawdzanie gotowej paczki GitHub. Radio nadal pracuje.")?;
    let root = root.to_owned();
    let tag = tag.to_owned();
    // Reserve the state before releasing the request; network I/O never runs on
    // the station command/radio thread.
    drop(guard);
    std::thread::spawn(move || {
        let _guard = OPERATION.lock().unwrap();
        let result = python(include_str!("../deploy/server/update-release.py"),
            &["download", architecture(), &root.to_string_lossy(), &tag])
            .and_then(|_| python(include_str!("../deploy/server/update-package.py"),
                &[&package_path(&root).to_string_lossy(), architecture(), &format!("--tag={tag}")]));
        match result {
            Ok(version) => { let _ = write_status(&root, "uploaded", version.trim(), &format!("Gotowa paczka {tag} pobrana. SHA-256 i zawartość ZIP poprawne. Możesz rozpocząć instalację.")); }
            Err(error) => { let _ = fs::remove_file(package_path(&root)); let _ = write_status(&root, "failed", &tag, &error); }
        }
    });
    Ok(json::object([("ok", "true".into())]))
}

fn validate(package: &Path) -> Result<String, String> {
    python(include_str!("../deploy/server/update-package.py"), &[&package.to_string_lossy(), architecture()])
}

pub fn write_status(root: &Path, state: &str, version: &str, log: &str) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let text = format!("state={state}\nversion={version}\nlog={log}\n");
    crate::storage::write_atomic(&status_path(root), text.as_bytes()).map_err(|e| e.to_string())
}

pub fn status_json(root: &Path) -> String {
    let mut state = "idle".to_owned();
    let mut version = String::new();
    let mut log = "Brak aktywnej aktualizacji.".to_owned();
    if let Ok(text) = fs::read_to_string(status_path(root)) {
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("state=") { state = value.into(); }
            if let Some(value) = line.strip_prefix("version=") { version = value.into(); }
            if let Some(value) = line.strip_prefix("log=") { log = value.into(); }
        }
    }
    json::object([
        ("state", json::string(&state)),
        ("version", json::string(&version)),
        ("log", json::string(&log)),
    ])
}

pub fn upload(root: &Path, body: &[u8]) -> Result<String, String> {
    let _guard = OPERATION.try_lock().map_err(|_| "Inna operacja aktualizacji jest w toku")?;
    ensure_idle(root)?;
    if body.is_empty() { return Err("Pusta paczka aktualizacji".into()); }
    if body.len() > MAX_UPLOAD { return Err("Paczka aktualizacji jest zbyt duża".into()); }
    if !body.starts_with(b"PK\x03\x04") { return Err("Wybierz binarną paczkę ZIP Station, nie inne archiwum".into()); }
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(body);
    crate::storage::write_atomic(&package_path(root), body).map_err(|e| e.to_string())?;
    let sha = format!("{:x}", hasher.finalize());
    write_status(root, "uploaded", "", &format!("Paczka zapisana. SHA-256: {sha}"))?;
    Ok(json::object([
        ("ok", "true".into()),
        ("size", body.len().to_string()),
        ("sha256", json::string(&sha)),
    ]))
}

pub fn start(root: &Path) -> Result<String, String> {
    let _guard = OPERATION.try_lock().map_err(|_| "Inna operacja aktualizacji jest w toku")?;
    ensure_idle(root)?;
    if !supported() { return Err("Brak obsługi aktualizacji z panelu dla tej instalacji. Użyj instalatora na hoście.".into()); }
    let package = package_path(root);
    if !package.is_file() { return Err("Najpierw wybierz paczkę ZIP".into()); }
    let version = validate(&package)?;
    write_status(root, "running", version.trim(), "Paczka sprawdzona. Przygotowanie aktualizacji…")?;
    #[cfg(unix)]
    {
        // A separate root service survives systemd stopping the Station cgroup.
        // NoNewPrivileges remains enabled on the application service.
        if Path::new("/etc/systemd/system/ultimatepr-update.path").is_file() {
            if let Err(e) = crate::storage::write_atomic(&root.join("install.request"), b"install\n") {
                let _ = write_status(root, "failed", "", &e.to_string());
                return Err(e.to_string());
            }
            return Ok("Aktualizacja przekazana do niezależnej usługi systemd".into());
        }
        use std::os::unix::process::CommandExt;
        let result = Command::new("doas")
            .args(["-n", "/usr/local/libexec/ultimatepr-update", "--request"])
            .process_group(0) // independent of the OpenRC application's group
            .spawn();
        let mut child = match result {
            Ok(child) => child,
            Err(error) => {
                let message = format!("Nie można uruchomić helpera aktualizacji: {error}");
                let _ = write_status(root, "failed", "", &message);
                return Err(message);
            }
        };
        let status_root = root.to_path_buf();
        std::thread::spawn(move || {
            if let Ok(exit) = child.wait() {
                if !exit.success() && ensure_idle(&status_root).is_err() {
                    let _ = write_status(
                        &status_root,
                        "failed",
                        "",
                        "Helper aktualizacji zakończył się błędem. Sprawdź doas i log usługi.",
                    );
                }
            }
        });
        return Ok("Aktualizacja rozpoczęta".into());
    }
    #[cfg(not(unix))]
    {
        let _ = package;
        let _ = write_status(root, "failed", "", "Aktualizacje z panelu są dostępne tylko dla Alpine/Linux");
        Err("Aktualizacje z panelu są dostępne tylko dla Alpine/Linux".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_states_block_replacement_of_an_update_package() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-tmp")
            .join(format!("update-states-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(package_path(&root), b"existing package").unwrap();
        for state in ["downloading", "running", "validating", "building", "stopping", "starting"] {
            write_status(&root, state, "vtest", "test").unwrap();
            assert!(upload(&root, b"PK\x03\x04replacement").is_err());
            assert_eq!(fs::read(package_path(&root)).unwrap(), b"existing package");
        }
        write_status(&root, "failed", "vtest", "test").unwrap();
        assert!(upload(&root, b"not zip").is_err());
        assert!(upload(&root, b"PK\x03\x04replacement").is_ok());
        assert_eq!(fs::read(package_path(&root)).unwrap(), b"PK\x03\x04replacement");
        fs::remove_dir_all(root).unwrap();
    }
}
