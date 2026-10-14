//! Small synchronous Telegram Bot API client.
//!
//! The station uses this client only from a bounded worker thread. The bot
//! token is passed in by the caller and is never included in errors.

use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    sync::{Arc, mpsc},
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chat {
    pub id: String,
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramError(pub String);

impl std::fmt::Display for TelegramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for TelegramError {}

pub struct MessageJob {
    pub token_path: PathBuf,
    pub chat_id: String,
    pub text: String,
}

pub enum Command {
    Send(MessageJob),
    Verify {
        token_path: PathBuf,
        reply: mpsc::SyncSender<Result<String, String>>,
    },
    Discover {
        token_path: PathBuf,
        reply: mpsc::SyncSender<Result<Vec<Chat>, String>>,
    },
    Test {
        token_path: PathBuf,
        chat_id: String,
        reply: mpsc::SyncSender<Result<(), String>>,
    },
}

pub fn spawn_worker(
    stop: Arc<AtomicBool>,
    log_path: PathBuf,
) -> (mpsc::SyncSender<Command>, std::thread::JoinHandle<()>) {
    let (sender, receiver) = mpsc::sync_channel(32);
    let handle = std::thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            let command = match receiver.recv_timeout(Duration::from_millis(200)) {
                Ok(command) => command,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            };
            match command {
                Command::Send(job) => {
                    if let Err(error) = send_job(job) {
                        log_error(&log_path, &error);
                    }
                }
                Command::Verify { token_path, reply } => {
                    let result = client_from_file(&token_path)
                        .and_then(|client| client.verify().map_err(|e| e.to_string()));
                    let _ = reply.send(result);
                }
                Command::Discover { token_path, reply } => {
                    let result = client_from_file(&token_path)
                        .and_then(|client| client.discover().map_err(|e| e.to_string()));
                    let _ = reply.send(result);
                }
                Command::Test {
                    token_path,
                    chat_id,
                    reply,
                } => {
                    let result = client_from_file(&token_path).and_then(|client| {
                        client
                            .send_message(
                                &chat_id,
                                "UltimatePR Station: wiadomość testowa Telegrama.",
                            )
                            .map_err(|e| e.to_string())
                    });
                    let _ = reply.send(result);
                }
            }
        }
    });
    (sender, handle)
}

fn log_error(path: &PathBuf, message: &str) {
    use std::io::Write;
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "Telegram: {message}");
    }
}

fn client_from_file(path: &PathBuf) -> Result<TelegramClient, String> {
    let token = fs::read_to_string(path)
        .map_err(|_| "Brak tokenu bota Telegrama".to_owned())?
        .trim()
        .to_owned();
    TelegramClient::new(token).map_err(|e| e.to_string())
}

fn send_job(job: MessageJob) -> Result<(), String> {
    let client = client_from_file(&job.token_path)?;
    let mut last = String::new();
    for attempt in 0..3 {
        match client.send_message(&job.chat_id, &job.text) {
            Ok(()) => return Ok(()),
            Err(error) => {
                last = error.to_string();
                if attempt < 2 {
                    std::thread::sleep(Duration::from_millis(250 * (attempt + 1)));
                }
            }
        }
    }
    Err(last)
}

pub struct TelegramClient {
    token: String,
}

impl TelegramClient {
    pub fn new(token: String) -> Result<Self, TelegramError> {
        if token.is_empty()
            || token.len() > 256
            || token
                .chars()
                .any(|c| c.is_ascii_control() || c.is_whitespace())
        {
            return Err(TelegramError(
                "Token Telegrama jest pusty lub ma nieprawidłowy format".into(),
            ));
        }
        Ok(Self { token })
    }

    fn call(&self, method: &str, fields: &[(&str, String)]) -> Result<String, TelegramError> {
        let url = format!("https://api.telegram.org/bot{}/{}", self.token, method);
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .build();
        let agent = config.new_agent();
        let response = agent
            .post(&url)
            .send_form(fields.iter().cloned())
            .map_err(|_| TelegramError("Telegram jest chwilowo niedostępny".into()))?;
        let status = response.status();
        let body = response.into_body().read_to_string().unwrap_or_default();
        if !status.is_success() || !json_bool(&body, "ok") {
            let description = json_string(&body, "description")
                .unwrap_or_else(|| "Nieznany błąd API Telegrama".into());
            return Err(TelegramError(format!("Telegram: {description}")));
        }
        Ok(body)
    }

    pub fn verify(&self) -> Result<String, TelegramError> {
        let body = self.call("getMe", &[])?;
        Ok(json_string_after(&body, "username", "result").unwrap_or_else(|| "bot".into()))
    }

    pub fn discover(&self) -> Result<Vec<Chat>, TelegramError> {
        let body = self.call(
            "getUpdates",
            &[
                ("timeout", "0".into()),
                ("allowed_updates", "[\"message\",\"channel_post\"]".into()),
            ],
        )?;
        let mut chats = Vec::new();
        let mut seen = HashSet::new();
        let mut offset = 0;
        while let Some(pos) = body[offset..].find("\"chat\"") {
            let start = offset + pos;
            let Some(open) = body[start..].find('{').map(|p| start + p) else {
                break;
            };
            let Some(close) = object_end(&body, open) else {
                break;
            };
            let object = &body[open..=close];
            if let Some(id) = json_string(object, "id").or_else(|| json_number(object, "id")) {
                if seen.insert(id.clone()) {
                    let kind = json_string(object, "type").unwrap_or_else(|| "chat".into());
                    let name = json_string(object, "title")
                        .or_else(|| json_string(object, "username"))
                        .or_else(|| json_string(object, "first_name"))
                        .unwrap_or_else(|| id.clone());
                    chats.push(Chat { id, kind, name });
                }
            }
            offset = close + 1;
        }
        Ok(chats)
    }

    pub fn send_message(&self, chat_id: &str, text: &str) -> Result<(), TelegramError> {
        if !valid_chat_id(chat_id) {
            return Err(TelegramError(
                "Nieprawidłowy identyfikator czatu Telegrama".into(),
            ));
        }
        let text = truncate(text, 4096);
        self.call(
            "sendMessage",
            &[("chat_id", chat_id.into()), ("text", text)],
        )?;
        Ok(())
    }
}

pub fn valid_chat_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && (value.starts_with('@') || value.chars().all(|c| c == '-' || c.is_ascii_digit()))
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.into();
    }
    value
        .chars()
        .take(max.saturating_sub(1))
        .collect::<String>()
        + "…"
}

fn json_bool(text: &str, key: &str) -> bool {
    text.contains(&format!("\"{key}\":true"))
}

fn json_number(text: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\":");
    let start = text.find(&marker)? + marker.len();
    let value = text[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .collect::<String>();
    (!value.is_empty()).then_some(value)
}

fn json_string(text: &str, key: &str) -> Option<String> {
    json_string_after(text, key, "")
}

fn json_string_after(text: &str, key: &str, after: &str) -> Option<String> {
    let base = if after.is_empty() {
        text
    } else {
        &text[text.find(after)?..]
    };
    let marker = format!("\"{key}\":\"");
    let start = base.find(&marker)? + marker.len();
    let mut result = String::new();
    let mut escaped = false;
    for ch in base[start..].chars() {
        if escaped {
            result.push(match ch {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '"' => '"',
                '\\' => '\\',
                other => other,
            });
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            return Some(result);
        }
        result.push(ch);
    }
    None
}

fn object_end(text: &str, start: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (index, ch) in text[start..].char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        match ch {
            '"' => quoted = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(start + index);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chat_ids_are_bounded() {
        assert!(valid_chat_id("-100123"));
        assert!(valid_chat_id("@upr_station"));
        assert!(!valid_chat_id("https://example"));
    }
    #[test]
    fn parses_chat_updates() {
        let body = r#"{"ok":true,"result":[{"message":{"chat":{"id":-100,"type":"group","title":"Test"}}}]}"#;
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut offset = 0;
        while let Some(pos) = body[offset..].find("\"chat\"") {
            let start = offset + pos;
            let open = start + body[start..].find('{').unwrap();
            let close = object_end(body, open).unwrap();
            let object = &body[open..=close];
            let id = json_number(object, "id").unwrap();
            if seen.insert(id.clone()) {
                out.push(Chat {
                    id,
                    kind: json_string(object, "type").unwrap(),
                    name: json_string(object, "title").unwrap(),
                });
            }
            offset = close + 1;
        }
        assert_eq!(out[0].name, "Test");
    }
}
