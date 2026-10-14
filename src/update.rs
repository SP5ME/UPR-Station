use crate::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[cfg(unix)]
use std::process::Command;

pub const MAX_UPLOAD: usize = 128 * 1024 * 1024;

pub fn directory(config_path: &Path) -> PathBuf {
    std::env::var_os("UPR_UPDATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| config_path.parent().unwrap_or(Path::new(".")).join("update"))
}

fn status_path(root: &Path) -> PathBuf { root.join("status.txt") }
fn package_path(root: &Path) -> PathBuf { root.join("incoming.zip") }

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
    if body.is_empty() { return Err("Pusta paczka aktualizacji".into()); }
    if body.len() > MAX_UPLOAD { return Err("Paczka aktualizacji jest zbyt duża".into()); }
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
    let package = package_path(root);
    if !package.is_file() { return Err("Najpierw wybierz paczkę ZIP".into()); }
    let current = status_json(root);
    if current.contains("\"state\":\"running\"") {
        return Err("Aktualizacja jest już uruchomiona".into());
    }
    write_status(root, "running", "", "Przygotowanie aktualizacji…")?;
    #[cfg(unix)]
    {
        let helper = std::env::var_os("UPR_UPDATE_HELPER").unwrap_or_else(|| "/usr/local/libexec/ultimatepr-update".into());
        let result = Command::new("doas")
            .args(["-n", helper.to_string_lossy().as_ref(), "--install"])
            .arg(&package)
            .arg(root)
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
                if !exit.success() && status_json(&status_root).contains("\"state\":\"running\"") {
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
