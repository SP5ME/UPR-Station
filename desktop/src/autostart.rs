//! Platform autostart registration. Paths are always absolute and quoted.
use std::path::Path;
#[cfg(target_os = "linux")]
const NAME: &str = "UPR Terminal";
// Preserve the existing Windows registration key across the display-name change.
#[cfg(windows)]
const RUN_VALUE: &str = "UltimatePR Station";

#[cfg(windows)]
fn command(exe: &Path, config: &Path, web: &Path) -> Result<String, String> {
    let quote = |p: &Path| -> Result<String, String> {
        let text = p.to_str().ok_or("Non-Unicode autostart path")?;
        if text.contains(['"', '\r', '\n']) {
            return Err("Invalid autostart path".into());
        }
        Ok(format!("\"{text}\""))
    };
    Ok(format!(
        "{} --minimized --config {} --web-config {}",
        quote(exe)?,
        quote(config)?,
        quote(web)?
    ))
}

#[cfg(windows)]
pub fn set(enabled: bool, config: &Path, web: &Path) -> Result<(), String> {
    use winreg::{
        RegKey,
        enums::{HKEY_CURRENT_USER, KEY_SET_VALUE},
    };
    let registry = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            KEY_SET_VALUE,
        )
        .map_err(|e| e.to_string())?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        registry
            .set_value(RUN_VALUE, &command(&exe, config, web)?)
            .map_err(|e| e.to_string())
    } else {
        match registry.delete_value(RUN_VALUE) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(target_os = "linux")]
fn entry() -> Result<std::path::PathBuf, String> {
    let root = match std::env::var_os("XDG_CONFIG_HOME").filter(|s| !s.is_empty()) {
        Some(s) => std::path::PathBuf::from(s),
        None => std::path::PathBuf::from(std::env::var_os("HOME").ok_or("HOME is missing")?)
            .join(".config"),
    };
    if !root.is_absolute() {
        return Err("XDG_CONFIG_HOME must be absolute".into());
    }
    Ok(root.join("autostart/org.ultimatepr.station.desktop"))
}
#[cfg(target_os = "linux")]
pub fn set(enabled: bool, config: &Path, web: &Path) -> Result<(), String> {
    let path = entry()?;
    if enabled {
        let quote = |p: &Path| -> Result<String, String> {
            let s = p.to_str().ok_or("Non-Unicode autostart path")?;
            if s.contains(['\n', '\r']) {
                return Err("Invalid autostart path".into());
            }
            Ok(format!(
                "\"{}\"",
                s.replace('\\', "\\\\\\\\")
                    .replace('"', "\\\\\"")
                    .replace('`', "\\\\`")
                    .replace('$', "\\\\$")
                    .replace('%', "%%")
            ))
        };
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let text = format!(
            "[Desktop Entry]\nType=Application\nName={NAME}\nTerminal=false\nExec={} --minimized --config {} --web-config {}\n",
            quote(&exe)?,
            quote(config)?,
            quote(web)?
        );
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        ultimatepr::storage::write_atomic(&path, text.as_bytes()).map_err(|e| e.to_string())
    } else {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn paths_with_spaces_are_quoted() {
        let c = command(
            Path::new("C:\\Program Files\\Station.exe"),
            Path::new("C:\\My Data\\station.conf"),
            Path::new("C:\\My Data\\web.conf"),
        )
        .unwrap();
        assert_eq!(
            c,
            "\"C:\\Program Files\\Station.exe\" --minimized --config \"C:\\My Data\\station.conf\" --web-config \"C:\\My Data\\web.conf\""
        );
    }
}
