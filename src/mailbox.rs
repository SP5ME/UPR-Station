//! Small durable personal-message store and line protocol for the local mailbox.
use crate::ax25::Call;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const MAX_CONNECTIONS: usize = 2;
const MAX_BODY: usize = 8 * 1024;
const MAX_LINE: usize = 512;

fn utc_clock(timestamp: u64) -> String {
    let seconds = timestamp % 86_400;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageType {
    Personal,
    Bulletin,
    Traffic,
}

impl MessageType {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Personal => "P",
            Self::Bulletin => "B",
            Self::Traffic => "T",
        }
    }

    fn from_storage_code(code: &str) -> Result<Self, String> {
        match code {
            "P" => Ok(Self::Personal),
            "B" => Ok(Self::Bulletin),
            "T" => Ok(Self::Traffic),
            _ => Err("invalid message type".into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub id: u64,
    pub from: Call,
    pub to: Call,
    pub message_type: MessageType,
    pub timestamp: u64,
    pub subject: String,
    pub body: String,
    pub read: bool,
}

pub struct Store {
    directory: PathBuf,
    messages: Vec<Message>,
    next_id: u64,
    warnings: Vec<String>,
}

impl Store {
    pub fn open(config_path: &Path) -> Result<Self, String> {
        let parent = config_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let directory = parent.join("mailbox");
        fs::create_dir_all(&directory).map_err(|e| format!("{}: {e}", directory.display()))?;
        let mut messages = Vec::new();
        let mut warnings = Vec::new();
        for entry in
            fs::read_dir(&directory).map_err(|e| format!("{}: {e}", directory.display()))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.path().extension().is_some_and(|ext| ext == "msg") {
                let text = match fs::read_to_string(entry.path()) {
                    Ok(text) => text,
                    Err(error) => {
                        warnings.push(format!("{}: {error}", entry.path().display()));
                        continue;
                    }
                };
                let mut message = match decode(&text) {
                    Ok(message) => message,
                    Err(error) => {
                        warnings.push(format!("{}: {error}", entry.path().display()));
                        continue;
                    }
                };
                message.read |= directory.join(format!("{:08}.read", message.id)).exists();
                messages.push(message);
            }
        }
        messages.sort_by_key(|m| m.id);
        let next_id = messages
            .iter()
            .map(|m| m.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        Ok(Self {
            directory,
            messages,
            next_id,
            warnings,
        })
    }

    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
    pub fn unread_count(&self) -> usize {
        self.messages.iter().filter(|m| !m.read).count()
    }
    pub fn unread_for(&self, call: &Call) -> usize {
        self.unread_for_calls(std::slice::from_ref(call))
    }
    pub fn unread_for_calls(&self, calls: &[Call]) -> usize {
        self.messages
            .iter()
            .filter(|m| !m.read && calls.iter().any(|call| &m.to == call))
            .count()
    }

    pub fn add(&mut self, from: Call, to: Call, subject: &str, body: &str) -> Result<u64, String> {
        self.add_typed(from, to, MessageType::Personal, subject, body)
    }

    pub fn add_typed(
        &mut self,
        from: Call,
        to: Call,
        message_type: MessageType,
        subject: &str,
        body: &str,
    ) -> Result<u64, String> {
        if self.messages.len() >= 1000 {
            return Err("MailBox message limit reached (1000)".into());
        }
        if subject.chars().count() > 80 || subject.chars().any(char::is_control) {
            return Err("Subject must be at most 80 characters".into());
        }
        if body.len() > MAX_BODY {
            return Err("Message body exceeds 8192 bytes".into());
        }
        // A malformed message file may have been preserved during recovery.
        // Skip occupied filenames so it cannot block all later saves or be
        // overwritten accidentally (notably on Windows, where rename fails).
        let mut id = self.next_id;
        while self.directory.join(format!("{id:08}.msg")).exists() {
            id = id
                .checked_add(1)
                .ok_or("MailBox message ID space exhausted")?;
        }
        let message = Message {
            id,
            from,
            to,
            message_type,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            subject: subject.to_owned(),
            body: body.to_owned(),
            read: false,
        };
        let path = self.directory.join(format!("{id:08}.msg"));
        write_message(&path, &message)?;
        self.messages.push(message);
        self.next_id = id.saturating_add(1);
        Ok(id)
    }

    pub fn mark_read(&mut self, id: u64) -> Result<bool, String> {
        self.set_read(id, true)
    }

    pub fn set_read(&mut self, id: u64, read: bool) -> Result<bool, String> {
        let Some(index) = self.messages.iter().position(|m| m.id == id) else {
            return Ok(false);
        };
        if self.messages[index].read == read {
            return Ok(true);
        }
        let marker = self.directory.join(format!("{id:08}.read"));
        if read {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&marker)
            {
                Ok(file) => file
                    .sync_all()
                    .map_err(|e| format!("Cannot mark MailBox message read: {e}"))?,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(format!("Cannot mark MailBox message read: {error}")),
            }
        } else if let Err(error) = fs::remove_file(&marker)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(format!("Cannot mark MailBox message unread: {error}"));
        }
        self.messages[index].read = read;
        Ok(true)
    }

    /// Permanently remove one message and its optional read marker.
    pub fn delete(&mut self, id: u64) -> Result<bool, String> {
        let Some(index) = self.messages.iter().position(|message| message.id == id) else {
            return Ok(false);
        };
        let path = self.directory.join(format!("{id:08}.msg"));
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(format!("Cannot delete MailBox message: {error}"));
            }
            Err(error) => return Err(format!("Cannot delete MailBox message: {error}")),
        }
        self.messages.remove(index);
        let marker = self.directory.join(format!("{id:08}.read"));
        if let Err(error) = fs::remove_file(marker)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            // The message itself is already gone; an orphan marker is harmless.
            self.warnings
                .push(format!("Cannot remove MailBox read marker: {error}"));
        }
        Ok(true)
    }
}

fn write_message(path: &Path, message: &Message) -> Result<(), String> {
    let temp = path.with_extension("tmp");
    let text = format!(
        "UPRMAIL2\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n\n{}",
        message.id,
        message.from,
        message.to,
        message.timestamp,
        message.read,
        message.message_type.code(),
        message.subject,
        message.body
    );
    fs::write(&temp, text).map_err(|e| format!("Cannot write MailBox message: {e}"))?;
    fs::rename(&temp, path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        format!("Cannot commit MailBox message: {e}")
    })
}

fn decode(text: &str) -> Result<Message, String> {
    let (header, body) = text.split_once("\n\n").ok_or("missing message body")?;
    let mut lines = header.lines();
    let version = lines.next().ok_or("missing message version")?;
    if version != "UPRMAIL1" && version != "UPRMAIL2" {
        return Err("unsupported MailBox file version".into());
    }
    let id = lines
        .next()
        .ok_or("missing ID")?
        .parse()
        .map_err(|_| "invalid ID")?;
    let from = lines
        .next()
        .ok_or("missing sender")?
        .parse()
        .map_err(|_| "invalid sender")?;
    let to = lines
        .next()
        .ok_or("missing recipient")?
        .parse()
        .map_err(|_| "invalid recipient")?;
    let timestamp = lines
        .next()
        .ok_or("missing timestamp")?
        .parse()
        .map_err(|_| "invalid timestamp")?;
    let read = lines
        .next()
        .ok_or("missing read state")?
        .parse()
        .map_err(|_| "invalid read state")?;
    let message_type = if version == "UPRMAIL2" {
        MessageType::from_storage_code(lines.next().ok_or("missing message type")?)?
    } else {
        MessageType::Personal
    };
    let subject = lines.next().ok_or("missing subject")?.to_owned();
    if lines.next().is_some() {
        return Err("unexpected message metadata".into());
    }
    Ok(Message {
        id,
        from,
        to,
        message_type,
        timestamp,
        subject,
        body: body.to_owned(),
        read,
    })
}

#[derive(Default)]
enum Mode {
    #[default]
    Command,
    Recipient,
    Subject {
        to: Call,
    },
    Body {
        to: Call,
        subject: String,
        text: String,
    },
}
#[derive(Default)]
struct Client {
    input: Vec<u8>,
    mode: Mode,
    discard_line: bool,
    disconnect: bool,
}

#[derive(Default)]
pub struct Protocol {
    clients: [Option<Client>; 8],
}

impl Protocol {
    pub fn connected(&mut self, id: usize, call: &Call) -> Vec<u8> {
        if id >= self.clients.len() {
            return Vec::new();
        }
        self.clients[id] = Some(Client::default());
        format!("UltimatePR MailBox {call}\r\nH Help  L List your mail  R <ID> Read  S Send  Q Disconnect\r\n> ").into_bytes()
    }

    pub fn disconnected(&mut self, id: usize) {
        if id < self.clients.len() {
            self.clients[id] = None;
        }
    }
    pub fn take_disconnect(&mut self, id: usize) -> bool {
        self.clients
            .get_mut(id)
            .and_then(Option::as_mut)
            .is_some_and(|client| std::mem::take(&mut client.disconnect))
    }

    pub fn data(
        &mut self,
        id: usize,
        peer: &Call,
        bytes: &[u8],
        store: &mut Store,
    ) -> Vec<Vec<u8>> {
        let Some(client) = self.clients.get_mut(id).and_then(Option::as_mut) else {
            return Vec::new();
        };
        let mut replies = Vec::new();
        for byte in bytes {
            if *byte == b'\n' && client.input.is_empty() {
                continue;
            }
            if *byte == b'\r' || *byte == b'\n' {
                let line = String::from_utf8_lossy(&client.input).into_owned();
                client.input.clear();
                if client.discard_line {
                    client.discard_line = false;
                    replies.push(b"Line too long; discarded.\r\n> ".to_vec());
                    continue;
                }
                let reply = command_line(client, peer, &line, store);
                if !reply.is_empty() {
                    replies.push(reply.into_bytes());
                }
            } else if client.input.len() < MAX_LINE {
                client.input.push(*byte);
            } else {
                client.input.clear();
                client.discard_line = true;
            }
        }
        replies
    }
}

fn prompt(text: &str) -> String {
    format!("{text}\r\n> ")
}

fn command_line(client: &mut Client, peer: &Call, line: &str, store: &mut Store) -> String {
    let mode = std::mem::take(&mut client.mode);
    match mode {
        Mode::Command => {
            let mut words = line.split_whitespace();
            match words.next().unwrap_or("").to_ascii_uppercase().as_str() {
                "H" | "HELP" => prompt(
                    "H Help\r\nL List your mail\r\nR <ID> Read a message\r\nS Send a message\r\nQ Disconnect",
                ),
                "L" | "LIST" => {
                    let rows = store
                        .messages()
                        .iter()
                        .filter(|m| &m.to == peer)
                        .rev()
                        .take(20)
                        .map(|m| {
                            format!(
                                "{} {} {} {} {} {}",
                                if m.read { "READ" } else { "NEW" },
                                m.id,
                                m.from,
                                m.to,
                                utc_clock(m.timestamp),
                                m.subject
                            )
                        })
                        .collect::<Vec<_>>();
                    if rows.is_empty() {
                        prompt("No messages for you.")
                    } else {
                        prompt(&rows.join("\r\n"))
                    }
                }
                "R" | "READ" => {
                    let Some(id) = words.next().and_then(|v| v.parse::<u64>().ok()) else {
                        return prompt("Usage: R <ID>");
                    };
                    let Some(message) = store
                        .messages()
                        .iter()
                        .find(|m| m.id == id && &m.to == peer)
                        .cloned()
                    else {
                        return prompt("Message not found.");
                    };
                    if let Err(error) = store.mark_read(id) {
                        return prompt(&format!("Storage error: {error}"));
                    }
                    prompt(&format!(
                        "From: {}  To: {}\r\nDate: {}\r\nSubject: {}\r\n\r\n{}",
                        message.from, message.to, message.timestamp, message.subject, message.body
                    ))
                }
                "S" | "SEND" => {
                    client.mode = Mode::Recipient;
                    prompt("Recipient callsign-SSID:")
                }
                "Q" | "QUIT" | "B" | "BYE" => {
                    client.disconnect = true;
                    String::new()
                }
                "" => "> ".into(),
                _ => prompt("Unknown command. Enter H for help."),
            }
        }
        Mode::Recipient => match line.trim().parse::<Call>() {
            Ok(to) => {
                client.mode = Mode::Subject { to };
                prompt("Subject (up to 80 characters):")
            }
            Err(_) => {
                client.mode = Mode::Recipient;
                prompt("Invalid callsign. Recipient callsign-SSID:")
            }
        },
        Mode::Subject { to } => {
            if line.chars().count() > 80 || line.chars().any(char::is_control) {
                client.mode = Mode::Subject { to };
                return prompt("Subject too long. Enter subject (max 80 characters):");
            }
            client.mode = Mode::Body {
                to,
                subject: line.to_owned(),
                text: String::new(),
            };
            prompt("Enter message lines; finish with /EX on a line by itself. Max 8192 bytes.")
        }
        Mode::Body {
            to,
            subject,
            mut text,
        } => {
            if line.eq_ignore_ascii_case("/EX") {
                return match store.add(peer.clone(), to, &subject, &text) {
                    Ok(id) => prompt(&format!("Message stored as #{id}.")),
                    Err(error) => prompt(&format!("Message not stored: {error}")),
                };
            }
            if text.len().saturating_add(line.len()).saturating_add(2) > MAX_BODY {
                client.mode = Mode::Body { to, subject, text };
                return prompt(
                    "Message limit reached; last line discarded. Enter /EX to save existing text.",
                );
            }
            text.push_str(line);
            text.push_str("\r\n");
            client.mode = Mode::Body { to, subject, text };
            "> ".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directory() -> PathBuf {
        let name = std::thread::current()
            .name()
            .unwrap_or("mailbox-test")
            .replace(':', "-");
        let path = crate::test_temp_dir(&format!("ultimatepr-mailbox-{name}"));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path.join("terminal.conf")
    }

    #[test]
    fn messages_persist_and_read_state_survives_restart() {
        let config = directory();
        let mut store = Store::open(&config).unwrap();
        let id = store
            .add(
                "ALPHA".parse().unwrap(),
                "BRAVO-2".parse().unwrap(),
                "Hello",
                "One\r\nTwo",
            )
            .unwrap();
        assert_eq!(store.unread_for(&"BRAVO-2".parse().unwrap()), 1);
        assert!(store.mark_read(id).unwrap());
        drop(store);
        let store = Store::open(&config).unwrap();
        assert_eq!(store.messages()[0].body, "One\r\nTwo");
        assert_eq!(store.unread_count(), 0);
        let _ = fs::remove_dir_all(config.parent().unwrap().join("mailbox"));
        let _ = fs::remove_dir_all(config.parent().unwrap());
    }

    #[test]
    fn selected_message_type_persists_and_legacy_messages_remain_personal() {
        let config = directory();
        let mut store = Store::open(&config).unwrap();
        let id = store
            .add_typed(
                "ALPHA".parse().unwrap(),
                "BRAVO".parse().unwrap(),
                MessageType::Traffic,
                "Radiogram",
                "Contents",
            )
            .unwrap();
        drop(store);
        let mailbox = config.parent().unwrap().join("mailbox");
        let store = Store::open(&config).unwrap();
        assert_eq!(store.messages()[0].id, id);
        assert_eq!(store.messages()[0].message_type, MessageType::Traffic);
        drop(store);

        fs::write(
            mailbox.join("00000042.msg"),
            "UPRMAIL1\n42\nALPHA\nBRAVO\n12345\nfalse\nLegacy\n\nOld format",
        )
        .unwrap();
        let store = Store::open(&config).unwrap();
        let legacy = store
            .messages()
            .iter()
            .find(|message| message.id == 42)
            .unwrap();
        assert_eq!(legacy.message_type, MessageType::Personal);
        let _ = fs::remove_dir_all(config.parent().unwrap());
    }

    #[test]
    fn unread_count_can_cover_operator_and_mailbox_calls() {
        let config = directory();
        let mut store = Store::open(&config).unwrap();
        let operator: Call = "N0CALL".parse().unwrap();
        let mailbox: Call = "N0CALL-8".parse().unwrap();
        store
            .add(
                "ALPHA".parse().unwrap(),
                operator.clone(),
                "For operator",
                "one",
            )
            .unwrap();
        store
            .add(
                "BRAVO".parse().unwrap(),
                mailbox.clone(),
                "For MailBox",
                "two",
            )
            .unwrap();
        assert_eq!(store.unread_for_calls(&[operator.clone(), mailbox]), 2);
        assert_eq!(store.unread_for(&operator), 1);
        let _ = fs::remove_dir_all(config.parent().unwrap());
    }

    #[test]
    fn delete_removes_message_read_marker_and_persists() {
        let config = directory();
        let mut store = Store::open(&config).unwrap();
        let id = store
            .add(
                "ALPHA".parse().unwrap(),
                "BRAVO".parse().unwrap(),
                "Test",
                "Message",
            )
            .unwrap();
        assert!(store.mark_read(id).unwrap());
        let message_path = config
            .parent()
            .unwrap()
            .join("mailbox")
            .join(format!("{id:08}.msg"));
        let read_path = config
            .parent()
            .unwrap()
            .join("mailbox")
            .join(format!("{id:08}.read"));
        assert!(message_path.exists());
        assert!(read_path.exists());

        assert!(store.delete(id).unwrap());
        assert!(!store.delete(id).unwrap());
        assert!(store.messages().is_empty());
        assert!(!message_path.exists());
        assert!(!read_path.exists());
        drop(store);
        assert!(Store::open(&config).unwrap().messages().is_empty());
        let _ = fs::remove_dir_all(config.parent().unwrap());
    }

    #[test]
    fn malformed_message_is_reported_and_does_not_hide_valid_messages() {
        let config = directory();
        let mut store = Store::open(&config).unwrap();
        store
            .add(
                "ALPHA".parse().unwrap(),
                "BRAVO".parse().unwrap(),
                "Valid",
                "Message body",
            )
            .unwrap();
        drop(store);

        let mailbox = config.parent().unwrap().join("mailbox");
        let invalid = mailbox.join("00000002.msg");
        fs::write(&invalid, b"not a valid MailBox message").unwrap();
        let mut store = Store::open(&config).unwrap();
        assert_eq!(store.messages().len(), 1);
        assert_eq!(store.messages()[0].subject, "Valid");
        let warnings = store.take_warnings();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("00000002.msg"));
        let id = store
            .add(
                "ALPHA".parse().unwrap(),
                "BRAVO".parse().unwrap(),
                "Next",
                "Still works",
            )
            .unwrap();
        assert_eq!(id, 3, "ID allocation skips preserved corrupt file");
        assert!(invalid.exists(), "invalid source file is preserved");
        let _ = fs::remove_dir_all(config.parent().unwrap());
    }

    #[test]
    fn remote_send_is_saved_only_after_explicit_end_marker() {
        let config = directory();
        let mut store = Store::open(&config).unwrap();
        let mut protocol = Protocol::default();
        let peer: Call = "PEER-1".parse().unwrap();
        assert!(
            !protocol
                .connected(0, &"LOCAL-1".parse().unwrap())
                .is_empty()
        );
        let replies = protocol.data(
            0,
            &peer,
            b"S\rDEST-2\rtest subject\rline one\r/EX\r",
            &mut store,
        );
        assert_eq!(store.messages().len(), 1);
        assert_eq!(store.messages()[0].from, peer);
        assert_eq!(store.messages()[0].to, "DEST-2".parse().unwrap());
        assert_eq!(store.messages()[0].body, "line one\r\n");
        assert!(replies.last().unwrap().windows(2).any(|v| v == b"> "));
        let _ = fs::remove_dir_all(config.parent().unwrap());
    }

    #[test]
    fn remote_list_displays_explicit_new_and_read_states() {
        let config = directory();
        let mut store = Store::open(&config).unwrap();
        let peer: Call = "PEER-1".parse().unwrap();
        let read_id = store
            .add(
                "ALPHA".parse().unwrap(),
                peer.clone(),
                "Already read",
                "one",
            )
            .unwrap();
        store.mark_read(read_id).unwrap();
        store
            .add("BRAVO".parse().unwrap(), peer.clone(), "Still new", "two")
            .unwrap();

        let mut protocol = Protocol::default();
        protocol.connected(0, &"LOCAL-1".parse().unwrap());
        let rows = String::from_utf8_lossy(&protocol.data(0, &peer, b"L\r", &mut store).concat())
            .into_owned();
        assert!(rows.contains("READ 1 ALPHA PEER-1"), "{rows}");
        assert!(rows.contains("NEW 2 BRAVO PEER-1"), "{rows}");
        let _ = fs::remove_dir_all(config.parent().unwrap());
    }

    #[test]
    fn remote_send_accepts_exact_8192_byte_body() {
        let config = directory();
        let mut store = Store::open(&config).unwrap();
        let mut protocol = Protocol::default();
        let peer: Call = "PEER-1".parse().unwrap();
        protocol.connected(0, &"LOCAL-1".parse().unwrap());
        protocol.data(0, &peer, b"S\rDEST-2\rsubject\r", &mut store);

        let mut body = Vec::new();
        for _ in 0..15 {
            body.extend(std::iter::repeat_n(b'x', 512));
            body.push(b'\r');
        }
        body.extend(std::iter::repeat_n(b'y', 480));
        body.extend_from_slice(b"\r/EX\r");
        protocol.data(0, &peer, &body, &mut store);
        assert_eq!(store.messages().len(), 1);
        assert_eq!(store.messages()[0].body.len(), MAX_BODY);
        let _ = fs::remove_dir_all(config.parent().unwrap());
    }

    #[test]
    fn remote_overflow_discards_only_last_line_and_can_save_prior_text() {
        let config = directory();
        let mut store = Store::open(&config).unwrap();
        let mut protocol = Protocol::default();
        let peer: Call = "PEER-1".parse().unwrap();
        protocol.connected(0, &"LOCAL-1".parse().unwrap());
        protocol.data(0, &peer, b"S\rDEST-2\rsubject\r", &mut store);

        let mut body = Vec::new();
        for _ in 0..15 {
            body.extend(std::iter::repeat_n(b'x', 512));
            body.push(b'\r');
        }
        body.extend(std::iter::repeat_n(b'y', 481));
        body.push(b'\r');
        let response = protocol.data(0, &peer, &body, &mut store);
        assert!(String::from_utf8_lossy(&response.concat()).contains("last line discarded"));
        protocol.data(0, &peer, b"/EX\r", &mut store);
        assert_eq!(store.messages().len(), 1);
        assert_eq!(store.messages()[0].body.len(), 15 * 514);
        let _ = fs::remove_dir_all(config.parent().unwrap());
    }
}
