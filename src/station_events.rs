//! Shared notification settings and bounded events for native and web clients.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationKind {
    ApplicationStarted,
    IncomingConnection,
    OutgoingConnection,
    TerminalMessage,
    Disconnection,
    Mailbox,
    Yapp,
    TncConnected,
    TncDisconnected,
    MheardNewStation,
    Critical,
}

impl NotificationKind {
    pub const ALL: [Self; 11] = [
        Self::ApplicationStarted,
        Self::IncomingConnection,
        Self::OutgoingConnection,
        Self::TerminalMessage,
        Self::Disconnection,
        Self::Mailbox,
        Self::Yapp,
        Self::TncConnected,
        Self::TncDisconnected,
        Self::MheardNewStation,
        Self::Critical,
    ];
    pub const fn key(self) -> &'static str {
        match self {
            Self::ApplicationStarted => "application_started",
            Self::IncomingConnection => "incoming_connection",
            Self::OutgoingConnection => "outgoing_connection",
            Self::TerminalMessage => "terminal_message",
            Self::Disconnection => "disconnection",
            Self::Mailbox => "mailbox",
            Self::Yapp => "yapp",
            Self::TncConnected => "tnc_connected",
            Self::TncDisconnected => "tnc_disconnected",
            Self::MheardNewStation => "mheard_new_station",
            Self::Critical => "critical",
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::ApplicationStarted => "Uruchomienie aplikacji",
            Self::IncomingConnection => "Nowe połączenie przychodzące",
            Self::OutgoingConnection => "Nowe połączenie wychodzące",
            Self::TerminalMessage => "Nowa wiadomość w terminalu",
            Self::Disconnection => "Rozłączenie",
            Self::Mailbox => "Nowa wiadomość MailBox",
            Self::Yapp => "Odebrany plik YAPP",
            Self::TncConnected => "Połączenie z TNC",
            Self::TncDisconnected => "Rozłączenie TNC",
            Self::MheardNewStation => "Nowa stacja w MHeard",
            Self::Critical => "Błąd krytyczny stacji",
        }
    }
}
impl fmt::Display for NotificationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationRule {
    pub in_app: bool,
    pub native: bool,
    pub telegram: bool,
    /// 0 = no sound, 1..=3 = number of short tones.
    pub sound: u8,
}
const fn rule(in_app: bool, native: bool, telegram: bool, sound: u8) -> NotificationRule {
    NotificationRule {
        in_app,
        native,
        telegram,
        sound,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationSettings {
    pub application_started: NotificationRule,
    pub incoming_connection: NotificationRule,
    pub outgoing_connection: NotificationRule,
    pub terminal_message: NotificationRule,
    pub disconnection: NotificationRule,
    pub mailbox: NotificationRule,
    pub yapp: NotificationRule,
    pub tnc_connected: NotificationRule,
    pub tnc_disconnected: NotificationRule,
    pub mheard_new_station: NotificationRule,
    pub critical: NotificationRule,
}
impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            application_started: rule(true, true, false, 1),
            incoming_connection: rule(true, true, false, 1),
            outgoing_connection: rule(true, true, false, 1),
            terminal_message: rule(true, true, false, 1),
            disconnection: rule(true, true, false, 0),
            mailbox: rule(true, true, false, 2),
            yapp: rule(true, true, false, 1),
            tnc_connected: rule(true, true, false, 3),
            tnc_disconnected: rule(true, true, false, 3),
            mheard_new_station: rule(false, false, false, 0),
            critical: rule(true, true, false, 3),
        }
    }
}
impl NotificationSettings {
    pub fn rule(&self, kind: NotificationKind) -> &NotificationRule {
        match kind {
            NotificationKind::ApplicationStarted => &self.application_started,
            NotificationKind::IncomingConnection => &self.incoming_connection,
            NotificationKind::OutgoingConnection => &self.outgoing_connection,
            NotificationKind::TerminalMessage => &self.terminal_message,
            NotificationKind::Disconnection => &self.disconnection,
            NotificationKind::Mailbox => &self.mailbox,
            NotificationKind::Yapp => &self.yapp,
            NotificationKind::TncConnected => &self.tnc_connected,
            NotificationKind::TncDisconnected => &self.tnc_disconnected,
            NotificationKind::MheardNewStation => &self.mheard_new_station,
            NotificationKind::Critical => &self.critical,
        }
    }
    pub fn rule_mut(&mut self, kind: NotificationKind) -> &mut NotificationRule {
        match kind {
            NotificationKind::ApplicationStarted => &mut self.application_started,
            NotificationKind::IncomingConnection => &mut self.incoming_connection,
            NotificationKind::OutgoingConnection => &mut self.outgoing_connection,
            NotificationKind::TerminalMessage => &mut self.terminal_message,
            NotificationKind::Disconnection => &mut self.disconnection,
            NotificationKind::Mailbox => &mut self.mailbox,
            NotificationKind::Yapp => &mut self.yapp,
            NotificationKind::TncConnected => &mut self.tnc_connected,
            NotificationKind::TncDisconnected => &mut self.tnc_disconnected,
            NotificationKind::MheardNewStation => &mut self.mheard_new_station,
            NotificationKind::Critical => &mut self.critical,
        }
    }
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        // Accept the spelling emitted by older web clients while keeping the
        // underscore form as the canonical station.conf representation.
        let rest = key
            .strip_prefix("notification_")
            .ok_or_else(|| format!("Unknown config key: {key}"))?
            .replace("in-app", "in_app");
        if !rest.starts_with("tnc_connected_")
            && !rest.starts_with("tnc_disconnected_")
            && let Some(field) = rest.strip_prefix("tnc_")
        {
            // Legacy notification_tnc_* applied to the former combined TNC
            // status row. Preserve that setting for both new TNC events.
            for kind in [NotificationKind::TncConnected, NotificationKind::TncDisconnected] {
                let rule = self.rule_mut(kind);
                match field {
                    "in_app" => rule.in_app = value.parse().map_err(|_| format!("{key} must be true/false"))?,
                    "native" => rule.native = value.parse().map_err(|_| format!("{key} must be true/false"))?,
                    "telegram" => rule.telegram = value.parse().map_err(|_| format!("{key} must be true/false"))?,
                    "sound" => {
                        rule.sound = value.parse::<u8>().map_err(|_| format!("{key} must be 0-3"))?;
                        if rule.sound > 3 { return Err(format!("{key} must be 0-3")); }
                    }
                    _ => return Err(format!("Invalid notification setting: {key}")),
                }
            }
            return Ok(());
        }
        if let Some(field) = rest.strip_prefix("connection_") {
            for kind in [
                NotificationKind::IncomingConnection,
                NotificationKind::OutgoingConnection,
            ] {
                let rule = self.rule_mut(kind);
                match field {
                    "in_app" => {
                        rule.in_app = value
                            .parse()
                            .map_err(|_| format!("{key} must be true/false"))?
                    }
                    "native" => {
                        rule.native = value
                            .parse()
                            .map_err(|_| format!("{key} must be true/false"))?
                    }
                    "telegram" => {
                        rule.telegram = value
                            .parse()
                            .map_err(|_| format!("{key} must be true/false"))?
                    }
                    "sound" => {
                        rule.sound = value
                            .parse::<u8>()
                            .map_err(|_| format!("{key} must be 0-3"))?;
                        if rule.sound > 3 {
                            return Err(format!("{key} must be 0-3"));
                        }
                    }
                    _ => return Err(format!("Invalid notification setting: {key}")),
                }
            }
            return Ok(());
        }
        let kind = NotificationKind::ALL
            .into_iter()
            .find(|kind| rest.starts_with(&format!("{}_", kind.key())))
            .ok_or_else(|| format!("Invalid notification setting: {key}"))?;
        let field = rest
            .strip_prefix(&format!("{}_", kind.key()))
            .ok_or_else(|| format!("Invalid notification setting: {key}"))?;
        let rule = self.rule_mut(kind);
        match field {
            "in_app" => {
                rule.in_app = value
                    .parse()
                    .map_err(|_| format!("{key} must be true/false"))?
            }
            "native" => {
                rule.native = value
                    .parse()
                    .map_err(|_| format!("{key} must be true/false"))?
            }
            "telegram" => {
                rule.telegram = value
                    .parse()
                    .map_err(|_| format!("{key} must be true/false"))?
            }
            "sound" => {
                rule.sound = value
                    .parse::<u8>()
                    .map_err(|_| format!("{key} must be 0-3"))?;
                if rule.sound > 3 {
                    return Err(format!("{key} must be 0-3"));
                }
            }
            _ => return Err(format!("Invalid notification setting: {key}")),
        }
        Ok(())
    }
    pub fn encode_lines(&self) -> Vec<String> {
        NotificationKind::ALL
            .into_iter()
            .flat_map(|kind| {
                let rule = self.rule(kind);
                [
                    format!("notification_{}_in_app={}", kind.key(), rule.in_app),
                    format!("notification_{}_native={}", kind.key(), rule.native),
                    format!("notification_{}_telegram={}", kind.key(), rule.telegram),
                    format!("notification_{}_sound={}", kind.key(), rule.sound),
                ]
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub id: u64,
    pub time: u64,
    pub kind: NotificationKind,
    pub title: String,
    pub body: String,
    pub sound: u8,
}


#[cfg(test)]
mod tests {
    use super::{NotificationKind, NotificationSettings};

    #[test]
    fn mheard_rule_is_opt_in_and_round_trips_all_channels() {
        let mut settings = NotificationSettings::default();
        let kind = NotificationKind::MheardNewStation;
        assert!(!settings.rule(kind).in_app);
        assert!(!settings.rule(kind).native);
        assert!(!settings.rule(kind).telegram);
        assert_eq!(settings.rule(kind).sound, 0);
        settings.set("notification_tnc_in_app", "true").unwrap();
        assert!(!settings.rule(kind).in_app);
        for channel in ["in_app", "native", "telegram", "sound"] {
            settings.set(&format!("notification_mheard_new_station_{channel}"),
                if channel == "sound" { "2" } else { "true" }).unwrap();
        }
        let mut restored = NotificationSettings::default();
        for line in settings.encode_lines() {
            let (key, value) = line.split_once('=').unwrap();
            restored.set(key, value).unwrap();
        }
        assert_eq!(restored, settings);
        assert!(restored.set("notification_mheard_new_station_sound", "4").is_err());
    }

    #[test]
    fn legacy_connection_rule_applies_to_both_directions() {
        let mut settings = NotificationSettings::default();
        settings
            .set("notification_connection_native", "false")
            .unwrap();
        settings.set("notification_connection_sound", "3").unwrap();
        assert!(!settings.rule(NotificationKind::IncomingConnection).native);
        assert!(!settings.rule(NotificationKind::OutgoingConnection).native);
        assert_eq!(settings.rule(NotificationKind::IncomingConnection).sound, 3);
        assert_eq!(settings.rule(NotificationKind::OutgoingConnection).sound, 3);
    }

    #[test]
    fn legacy_tnc_rule_applies_to_connected_and_disconnected_events() {
        let mut settings = NotificationSettings::default();
        settings.set("notification_tnc_native", "false").unwrap();
        settings.set("notification_tnc_sound", "2").unwrap();
        assert!(!settings.rule(NotificationKind::TncConnected).native);
        assert!(!settings.rule(NotificationKind::TncDisconnected).native);
        assert_eq!(settings.rule(NotificationKind::TncConnected).sound, 2);
        assert_eq!(settings.rule(NotificationKind::TncDisconnected).sound, 2);
    }

    #[test]
    fn legacy_in_app_notification_key_remains_compatible() {
        let mut settings = NotificationSettings::default();
        settings
            .set("notification_application_started_in-app", "true")
            .unwrap();
        assert!(settings
            .rule(NotificationKind::ApplicationStarted)
            .in_app);
        assert!(settings
            .encode_lines()
            .iter()
            .any(|line| line == "notification_application_started_in_app=true"));
    }
}
