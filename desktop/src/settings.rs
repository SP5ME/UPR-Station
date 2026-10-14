use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const MIGRATION_MARKER: &str = ".profile-migration-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preferences {
    pub autostart: bool,
    pub connections: bool,
    pub mail: bool,
    pub files: bool,
    pub tnc: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            autostart: false,
            connections: true,
            mail: true,
            files: true,
            tnc: true,
        }
    }
}
impl Preferences {
    pub fn load(path: &Path) -> Result<Self, String> {
        match fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }
    fn parse(text: &str) -> Result<Self, String> {
        let mut result = Self::default();
        for line in text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
        {
            let (key, value) = line.split_once('=').ok_or("Invalid desktop.conf line")?;
            let field = match key.trim() {
                "autostart" => &mut result.autostart,
                "connections" => &mut result.connections,
                "mail" => &mut result.mail,
                "files" => &mut result.files,
                "tnc" => &mut result.tnc,
                _ => return Err(format!("Unknown desktop setting: {key}")),
            };
            *field = value
                .trim()
                .parse()
                .map_err(|_| format!("Invalid boolean: {key}"))?;
        }
        Ok(result)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = format!(
            "autostart={}\nconnections={}\nmail={}\nfiles={}\ntnc={}\n",
            self.autostart, self.connections, self.mail, self.files, self.tnc
        );
        ultimatepr::storage::write_atomic(path, text.as_bytes()).map_err(|e| e.to_string())
    }
}

pub fn default_data_dir() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        Ok(PathBuf::from(env::var_os("APPDATA").ok_or("APPDATA is missing")?)
            .join("UltimatePR Station"))
    }
    #[cfg(not(windows))]
    {
        let root = match env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
            Some(value) => PathBuf::from(value),
            None => {
                PathBuf::from(env::var_os("HOME").ok_or("HOME is missing")?).join(".local/share")
            }
        };
        if !root.is_absolute() {
            return Err("XDG_DATA_HOME must be absolute".into());
        }
        Ok(root.join("ultimatepr-station"))
    }
}

#[cfg(windows)]
fn legacy_data_dir() -> Result<PathBuf, String> {
    Ok(PathBuf::from(
        env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is missing")?,
    )
    .join("UltimatePR Station"))
}

fn file_mtime(path: &Path) -> Result<SystemTime, String> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn copy_file_atomic(source: &Path, destination: &Path) -> Result<(), String> {
    let bytes = fs::read(source).map_err(|error| format!("{}: {error}", source.display()))?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    ultimatepr::storage::write_atomic(destination, &bytes)
        .map_err(|error| format!("{}: {error}", destination.display()))
}

fn copy_tree_missing(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(source).map_err(|error| format!("{}: {error}", source.display()))? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let name = entry.file_name();
        if name == "webview" || name == "desktop.log" || name == ".station.lock" {
            continue;
        }
        let destination_path = destination.join(&name);
        if source_path.is_dir() {
            fs::create_dir_all(&destination_path)
                .map_err(|error| format!("{}: {error}", destination_path.display()))?;
            copy_tree_missing(&source_path, &destination_path)?;
        } else if !destination_path.exists() {
            copy_file_atomic(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

/// Move a user's old desktop profile into the shared application profile.
/// Existing files are never deleted; a marker makes the operation idempotent.
pub fn migrate_profile(canonical: &Path, legacy: &Path) -> Result<bool, String> {
    if canonical == legacy {
        return Ok(false);
    }
    let marker = canonical.join(MIGRATION_MARKER);
    if !legacy.is_dir() {
        return Ok(false);
    }
    let already_migrated = marker.is_file();

    let canonical_config = canonical.join("station.conf");
    let legacy_config = legacy.join("station.conf");
    let canonical_value = canonical_config
        .is_file()
        .then(|| ultimatepr::config::Config::load(&canonical_config).ok())
        .flatten();
    let canonical_valid = canonical_value.is_some();
    let legacy_valid = legacy_config.is_file()
        && ultimatepr::config::Config::load(&legacy_config).is_ok();
    if (canonical_config.is_file() && !canonical_valid && !legacy_config.is_file())
        || (legacy_config.is_file() && !legacy_valid && !canonical_config.is_file())
        || (canonical_config.is_file() && legacy_config.is_file() && !canonical_valid && !legacy_valid)
    {
        return Err(format!(
            "Existing station configuration is invalid: {} or {}",
            canonical_config.display(),
            legacy_config.display()
        ));
    }
    fs::create_dir_all(canonical).map_err(|error| format!("{}: {error}", canonical.display()))?;

    let canonical_pristine = canonical_value
        .as_ref()
        .is_some_and(|config| config.interfaces.is_empty());
    let source_config = match (canonical_valid, legacy_valid) {
        (true, true) => {
            if canonical_pristine || file_mtime(&legacy_config)? > file_mtime(&canonical_config)? {
                Some(legacy_config.as_path())
            } else {
                Some(canonical_config.as_path())
            }
        }
        (true, false) => Some(canonical_config.as_path()),
        (false, true) => Some(legacy_config.as_path()),
        (false, false) => None,
    };
    let selected_legacy = source_config == Some(legacy_config.as_path());
    if let Some(source) = source_config {
        if source != canonical_config {
            if canonical_config.exists() {
                ultimatepr::storage::backup_existing(&canonical_config)
                    .map_err(|error| format!("{}: {error}", canonical_config.display()))?;
            }
            copy_file_atomic(source, &canonical_config)?;
        }
    }
    if selected_legacy {
        for name in ["web.conf", "desktop.conf"] {
            let source = legacy.join(name);
            let destination = canonical.join(name);
            if source.is_file() {
                if destination.is_file() {
                    ultimatepr::storage::backup_existing(&destination)
                        .map_err(|error| format!("{}: {error}", destination.display()))?;
                }
                copy_file_atomic(&source, &destination)?;
            }
        }
    }
    copy_tree_missing(legacy, canonical)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    fs::write(&marker, format!("migrated_at={stamp}\nsource={}\n", legacy.display()))
        .map_err(|error| format!("{}: {error}", marker.display()))?;
    Ok(selected_legacy || !already_migrated)
}

pub struct Options {
    pub config: PathBuf,
    pub web_config: PathBuf,
    pub data: PathBuf,
    pub demo: bool,
    pub minimized: bool,
    pub server_mode: bool,
}
impl Options {
    pub fn parse() -> Result<Option<Self>, String> {
        let mut args = env::args().skip(1);
        let mut config = None;
        let mut web_config = None;
        let mut demo = false;
        let mut minimized = false;
        let mut server_mode = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--config" => {
                    config = Some(PathBuf::from(args.next().ok_or("Missing config path")?))
                }
                "--web-config" => {
                    web_config = Some(PathBuf::from(args.next().ok_or("Missing web config path")?))
                }
                "--demo" => demo = true,
                "--minimized" => minimized = true,
                "--headless" => server_mode = true,
                "--mode" => {
                    let mode = args.next().ok_or("Missing mode")?;
                    match mode.as_str() {
                        "server" => server_mode = true,
                        "desktop" => server_mode = false,
                        _ => return Err("Mode must be desktop or server".into()),
                    }
                }
                "--help" | "-h" => {
                    println!(
                        "UltimatePR Station desktop\n--config PATH\n--web-config PATH\n--demo (isolated data)\n--minimized\n--mode desktop|server (Linux; desktop is default)\n--headless (alias for --mode server)"
                    );
                    return Ok(None);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        if demo && (config.is_some() || web_config.is_some()) {
            return Err("--demo cannot be combined with real station configuration paths".into());
        }
        let default = default_data_dir()?;
        #[cfg(windows)]
        if config.is_none() && !demo {
            let legacy = legacy_data_dir()?;
            migrate_profile(&default, &legacy)?;
        }
        let config = config.unwrap_or_else(|| {
            if demo {
                default.join("demo/station.conf")
            } else {
                default.join("station.conf")
            }
        });
        let config = std::path::absolute(config).map_err(|e| e.to_string())?;
        let data = config.parent().ok_or("Invalid config path")?.to_path_buf();
        let web_config = std::path::absolute(web_config.unwrap_or_else(|| data.join("web.conf")))
            .map_err(|e| e.to_string())?;
        Ok(Some(Self {
            config,
            web_config,
            data,
            demo,
            minimized,
            server_mode,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn temp_profile(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ultimatepr-settings-{name}-{}-{stamp}", std::process::id()))
    }
    #[test]
    fn invalid_preferences_are_not_silently_overwritten() {
        assert!(Preferences::parse("mail=maybe").is_err());
        assert!(Preferences::parse("unknown=true").is_err());
        assert!(!Preferences::default().autostart);
        assert!(!Preferences::parse("mail=false").unwrap().mail);
    }

    #[test]
    fn profile_migration_preserves_user_files_and_is_idempotent() {
        let canonical = temp_profile("canonical");
        let legacy = temp_profile("legacy");
        fs::create_dir_all(legacy.join("messages")).unwrap();
        fs::write(
            legacy.join("station.conf"),
            ultimatepr::config::encode(&ultimatepr::config::Config {
                language: "pl".into(),
                ..ultimatepr::config::Config::fresh()
            }),
        )
        .unwrap();
        fs::write(legacy.join("web.conf"), "listen=127.0.0.1:8080\n").unwrap();
        fs::write(legacy.join("desktop.conf"), "mail=false\n").unwrap();
        fs::write(legacy.join("legacy.secret"), "secret\n").unwrap();
        fs::write(legacy.join("messages/custom.txt"), "hello\n").unwrap();

        assert!(migrate_profile(&canonical, &legacy).unwrap());
        assert_eq!(fs::read_to_string(canonical.join("legacy.secret")).unwrap(), "secret\n");
        assert_eq!(fs::read_to_string(canonical.join("messages/custom.txt")).unwrap(), "hello\n");
        assert_eq!(ultimatepr::config::Config::load(&canonical.join("station.conf")).unwrap().language, "pl");
        assert!(canonical.join(MIGRATION_MARKER).is_file());
        assert!(!migrate_profile(&canonical, &legacy).unwrap());

        let _ = fs::remove_dir_all(canonical);
        let _ = fs::remove_dir_all(legacy);
    }

    #[test]
    fn newer_legacy_config_replaces_canonical_and_keeps_backup() {
        let canonical = temp_profile("canonical-newer");
        let legacy = temp_profile("legacy-newer");
        fs::create_dir_all(&canonical).unwrap();
        fs::create_dir_all(&legacy).unwrap();
        fs::write(
            canonical.join("station.conf"),
            ultimatepr::config::encode(&ultimatepr::config::Config::fresh()),
        )
        .unwrap();
        std::thread::sleep(Duration::from_millis(50));
        fs::write(
            legacy.join("station.conf"),
            ultimatepr::config::encode(&ultimatepr::config::Config {
                language: "pl".into(),
                ..ultimatepr::config::Config::fresh()
            }),
        )
        .unwrap();

        assert!(migrate_profile(&canonical, &legacy).unwrap());
        assert_eq!(ultimatepr::config::Config::load(&canonical.join("station.conf")).unwrap().language, "pl");
        assert!(canonical.join("station.bak").is_file());

        let _ = fs::remove_dir_all(canonical);
        let _ = fs::remove_dir_all(legacy);
    }

    #[test]
    fn invalid_existing_config_blocks_migration() {
        let canonical = temp_profile("canonical-invalid");
        let legacy = temp_profile("legacy-invalid");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("station.conf"), "invalid line\n").unwrap();

        let error = migrate_profile(&canonical, &legacy).unwrap_err();
        assert!(error.contains("configuration is invalid"));
        assert!(!canonical.exists());

        let _ = fs::remove_dir_all(canonical);
        let _ = fs::remove_dir_all(legacy);
    }
}
