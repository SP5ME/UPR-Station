//! Station-only configuration. Radio settings remain importable from Terminal,
//! while HTTP/runtime settings live in a separate file owned by Station.
use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebConfig {
    pub bind: SocketAddr,
    pub token_file: Option<PathBuf>,
    pub auth_enabled: bool,
    pub presence_mode: String,
    pub presence_bar_enabled: bool,
    pub absence_profile: u8,
    pub heartbeat_timeout: u64,
    pub statistics_enabled: bool,
    /// None keeps the complete locally stored statistics history.
    pub statistics_retention_days: Option<u64>,
    /// MQTT transport is deliberately disabled until explicitly enabled.
    pub mqtt_enabled: bool,
    pub mqtt_broker: String,
    pub mqtt_username: String,
    pub mqtt_tls: bool,
    pub mqtt_client_id: String,
    pub mqtt_topic_prefix: String,
    pub mqtt_discovery_topic: String,
    pub mqtt_discovery_enabled: bool,
    /// Whether the file explicitly contained auth_enabled. Used only for
    /// backwards-compatible migration of older installations.
    pub auth_configured: bool,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:8080".parse().unwrap(),
            token_file: None,
            auth_enabled: false,
            auth_configured: false,
            presence_mode: "auto".into(),
            presence_bar_enabled: false,
            absence_profile: 2,
            heartbeat_timeout: 90,
            statistics_enabled: false,
            statistics_retention_days: None,
            mqtt_enabled: false,
            mqtt_broker: String::new(),
            mqtt_username: String::new(),
            mqtt_tls: false,
            mqtt_client_id: "ultimatepr-station".into(),
            mqtt_topic_prefix: "ultimatepr".into(),
            mqtt_discovery_topic: "homeassistant".into(),
            mqtt_discovery_enabled: true,
        }
    }
}

impl WebConfig {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let mut config = Self::default();
        for (line_no, raw) in fs::read_to_string(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .lines()
            .enumerate()
        {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("{}:{}: expected key=value", path.display(), line_no + 1))?;
            let value = value.trim();
            match key.trim() {
                "bind" => {
                    config.bind = value.parse().map_err(|_| {
                        format!("{}:{}: invalid bind address", path.display(), line_no + 1)
                    })?
                }
                "token_file" => {
                    if !value.is_empty() {
                        let token = PathBuf::from(value);
                        if token
                            .components()
                            .any(|component| matches!(component, std::path::Component::ParentDir))
                        {
                            return Err(format!(
                                "{}:{}: token_file cannot contain '..'",
                                path.display(),
                                line_no + 1
                            ));
                        }
                        config.token_file = Some(token);
                    }
                }
                "auth_enabled" => {
                    config.auth_enabled = value.parse().map_err(|_| {
                        format!(
                            "{}:{}: auth_enabled must be true/false",
                            path.display(),
                            line_no + 1
                        )
                    })?;
                    config.auth_configured = true;
                }
                "presence_mode" => {
                    if !matches!(value, "auto" | "always" | "dnd") {
                        return Err(format!(
                            "{}:{}: invalid presence_mode",
                            path.display(),
                            line_no + 1
                        ));
                    }
                    config.presence_mode = value.to_owned();
                }
                "presence_bar_enabled" => {
                    config.presence_bar_enabled = value.parse().map_err(|_| {
                        format!(
                            "{}:{}: presence_bar_enabled must be true/false",
                            path.display(),
                            line_no + 1
                        )
                    })?;
                }
                "absence_profile" => {
                    config.absence_profile = value.parse().map_err(|_| {
                        format!(
                            "{}:{}: invalid absence_profile",
                            path.display(),
                            line_no + 1
                        )
                    })?;
                    if !(1..=4).contains(&config.absence_profile) {
                        return Err(format!(
                            "{}:{}: absence_profile must be 1..4",
                            path.display(),
                            line_no + 1
                        ));
                    }
                }
                "heartbeat_timeout" => {
                    config.heartbeat_timeout = value
                        .parse::<u64>()
                        .map_err(|_| {
                            format!(
                                "{}:{}: invalid heartbeat_timeout",
                                path.display(),
                                line_no + 1
                            )
                        })?
                        .clamp(15, 3600);
                }
                "statistics_enabled" => {
                    config.statistics_enabled = value.parse().map_err(|_| {
                        format!(
                            "{}:{}: statistics_enabled must be true/false",
                            path.display(),
                            line_no + 1
                        )
                    })?;
                }
                "statistics_retention_days" => {
                    config.statistics_retention_days = match value {
                        "" | "continuous" => None,
                        value => Some(value.parse::<u64>().map_err(|_| {
                            format!(
                                "{}:{}: statistics_retention_days must be continuous or a number",
                                path.display(),
                                line_no + 1
                            )
                        })?.clamp(1, 3650)),
                    };
                }
                "mqtt_enabled" => {
                    config.mqtt_enabled = value.parse().map_err(|_| {
                        format!("{}:{}: mqtt_enabled must be true/false", path.display(), line_no + 1)
                    })?;
                }
                "mqtt_broker" => config.mqtt_broker = value.to_owned(),
                "mqtt_username" => config.mqtt_username = value.to_owned(),
                "mqtt_tls" => {
                    config.mqtt_tls = value.parse().map_err(|_| {
                        format!("{}:{}: mqtt_tls must be true/false", path.display(), line_no + 1)
                    })?;
                }
                "mqtt_client_id" => config.mqtt_client_id = value.to_owned(),
                "mqtt_topic_prefix" => config.mqtt_topic_prefix = value.to_owned(),
                "mqtt_discovery_topic" => config.mqtt_discovery_topic = value.to_owned(),
                "mqtt_discovery_enabled" => {
                    config.mqtt_discovery_enabled = value.parse().map_err(|_| {
                        format!("{}:{}: mqtt_discovery_enabled must be true/false", path.display(), line_no + 1)
                    })?;
                }
                unknown => {
                    return Err(format!(
                        "{}:{}: unknown web setting {unknown}",
                        path.display(),
                        line_no + 1
                    ));
                }
            }
        }
        Ok(config)
    }

    pub fn encode(&self) -> String {
        format!(
            "bind={}\ntoken_file={}\nauth_enabled={}\npresence_mode={}\npresence_bar_enabled={}\nabsence_profile={}\nheartbeat_timeout={}\nstatistics_enabled={}\nstatistics_retention_days={}\nmqtt_enabled={}\nmqtt_broker={}\nmqtt_username={}\nmqtt_tls={}\nmqtt_client_id={}\nmqtt_topic_prefix={}\nmqtt_discovery_topic={}\nmqtt_discovery_enabled={}\n",
            self.bind,
            self.token_file
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            self.auth_enabled,
            self.presence_mode,
            self.presence_bar_enabled,
            self.absence_profile,
            self.heartbeat_timeout,
            self.statistics_enabled,
            self.statistics_retention_days
                .map(|days| days.to_string())
                .unwrap_or_else(|| "continuous".into()),
            self.mqtt_enabled,
            self.mqtt_broker,
            self.mqtt_username,
            self.mqtt_tls,
            self.mqtt_client_id,
            self.mqtt_topic_prefix,
            self.mqtt_discovery_topic,
            self.mqtt_discovery_enabled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::WebConfig;
    use std::{fs, path::PathBuf};

    #[test]
    fn parses_station_only_settings() {
        let path = PathBuf::from(format!("target/web-config-{}.conf", std::process::id()));
        fs::write(
            &path,
            "bind=127.0.0.1:9090\ntoken_file=data/operator.token\nstatistics_enabled=true\nstatistics_retention_days=90\n",
        )
        .unwrap();
        let config = WebConfig::load(&path).unwrap();
        assert_eq!(config.bind.to_string(), "127.0.0.1:9090");
        assert_eq!(
            config.token_file,
            Some(PathBuf::from("data/operator.token"))
        );
        assert!(config.statistics_enabled);
        assert_eq!(config.statistics_retention_days, Some(90));
        assert!(!config.presence_bar_enabled);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_statistics_retention_defaults_to_continuous() {
        let path = PathBuf::from(format!(
            "target/web-config-statistics-retention-{}.conf",
            std::process::id()
        ));
        fs::write(&path, "statistics_enabled=true\n").unwrap();
        let config = WebConfig::load(&path).unwrap();
        assert_eq!(config.statistics_retention_days, None);
        assert!(config.encode().contains("statistics_retention_days=continuous"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_presence_bar_setting() {
        let path = PathBuf::from(format!(
            "target/web-config-presence-{}.conf",
            std::process::id()
        ));
        fs::write(&path, "presence_bar_enabled=true\n").unwrap();
        let config = WebConfig::load(&path).unwrap();
        assert!(config.presence_bar_enabled);
        assert!(config.encode().contains("presence_bar_enabled=true"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_mqtt_settings_without_password_in_web_config() {
        let path = PathBuf::from(format!(
            "target/web-config-mqtt-{}.conf",
            std::process::id()
        ));
        fs::write(
            &path,
            "mqtt_enabled=true\nmqtt_broker=mqtt://broker:1883\nmqtt_username=station\nmqtt_tls=true\nmqtt_client_id=upr-test\nmqtt_topic_prefix=ultimatepr/test\nmqtt_discovery_enabled=true\n",
        )
        .unwrap();
        let config = WebConfig::load(&path).unwrap();
        assert!(config.mqtt_enabled);
        assert_eq!(config.mqtt_broker, "mqtt://broker:1883");
        assert_eq!(config.mqtt_username, "station");
        assert!(config.mqtt_tls);
        assert_eq!(config.mqtt_client_id, "upr-test");
        assert_eq!(config.mqtt_topic_prefix, "ultimatepr/test");
        assert!(config.encode().contains("mqtt_discovery_enabled=true"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_token_path_traversal() {
        let path = PathBuf::from(format!(
            "target/web-config-invalid-{}.conf",
            std::process::id()
        ));
        fs::write(&path, "token_file=../operator.token\n").unwrap();
        assert!(WebConfig::load(&path).is_err());
        let _ = fs::remove_file(path);
    }
}
