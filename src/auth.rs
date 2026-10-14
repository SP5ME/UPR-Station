use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug)]
pub struct Auth {
    path: PathBuf,
    username: String,
    salt: String,
    password: String,
    pub force_change: bool,
    sessions: HashMap<String, String>,
    next: u64,
}

impl Auth {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        if !path.exists() {
            let mut auth = Self {
                path,
                username: "admin".into(),
                salt: String::new(),
                password: String::new(),
                force_change: false,
                sessions: HashMap::new(),
                next: 0,
            };
            auth.set_password("packet")?;
            auth.persist()?;
            return Ok(auth);
        }
        let mut values = HashMap::new();
        let text =
            String::from_utf8(crate::storage::read_secret(&path).map_err(|e| e.to_string())?)
                .map_err(|_| "Invalid operator account")?;
        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=') {
                values.insert(k.trim(), v.trim().to_owned());
            }
        }
        let username = values
            .get("username")
            .cloned()
            .filter(|v| !v.is_empty())
            .ok_or("Invalid operator account")?;
        let salt = values
            .get("salt")
            .cloned()
            .ok_or("Invalid operator account")?;
        let password = values
            .get("password")
            .cloned()
            .ok_or("Invalid operator account")?;
        Ok(Self {
            path,
            username,
            salt,
            password,
            force_change: values.get("force_change").is_some_and(|v| v == "true"),
            sessions: HashMap::new(),
            next: 0,
        })
    }
    fn set_password(&mut self, password: &str) -> Result<(), String> {
        self.salt = seed(&format!("{}:{}", self.username, now()));
        self.password = digest(&self.salt, password);
        Ok(())
    }
    fn persist(&self) -> Result<(), String> {
        let text = format!(
            "username={}\nsalt={}\npassword={}\nforce_change={}\n",
            self.username, self.salt, self.password, self.force_change
        );
        crate::storage::write_secret_atomic(&self.path, text.as_bytes()).map_err(|e| e.to_string())
    }
    pub fn login(&mut self, username: &str, password: &str) -> Option<(String, bool)> {
        if username != self.username || digest(&self.salt, password) != self.password {
            return None;
        }
        self.next = self.next.wrapping_add(1);
        let token = digest(
            &seed(&format!("{}:{}:{}", self.username, now(), self.next)),
            password,
        );
        self.sessions.insert(token.clone(), username.to_owned());
        Some((token, self.force_change))
    }
    pub fn valid_session(&self, token: &str) -> bool {
        self.sessions.contains_key(token)
    }
    pub fn logout(&mut self, token: &str) {
        self.sessions.remove(token);
    }
    pub fn update(&mut self, token: &str, username: &str, password: &str) -> Result<(), String> {
        if !self.valid_session(token) {
            return Err("Unauthorized".into());
        }
        if username.trim().is_empty() || username.len() > 64 {
            return Err("Invalid login".into());
        }
        self.apply_update(username, password)?;
        // Changing the account invalidates other sessions, but the operator
        // who just completed the change must remain logged in.
        self.sessions
            .insert(token.to_owned(), self.username.clone());
        Ok(())
    }
    pub fn update_without_session(&mut self, username: &str, password: &str) -> Result<(), String> {
        if username.trim().is_empty() || username.len() > 64 {
            return Err("Invalid login".into());
        }
        self.apply_update(username, password)
    }
    fn apply_update(&mut self, username: &str, password: &str) -> Result<(), String> {
        self.username = username.trim().to_owned();
        self.set_password(password)?;
        self.force_change = false;
        self.sessions.clear();
        self.persist()
    }
    pub fn username(&self) -> &str {
        &self.username
    }
}

pub type SharedAuth = std::sync::Arc<Mutex<Auth>>;
fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
fn seed(text: &str) -> String {
    digest("ultimatepr-auth-seed", text)
}
fn digest(salt: &str, value: &str) -> String {
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update([0]);
    h.update(value.as_bytes());
    let mut bytes: [u8; 32] = h.finalize().into();
    for _ in 0..20_000 {
        let mut round = Sha256::new();
        round.update(bytes);
        round.update(salt.as_bytes());
        bytes = round.finalize().into();
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::Auth;
    #[test]
    fn creates_and_logs_in() {
        let path = std::path::PathBuf::from(format!("target/auth-{}.conf", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut a = Auth::load(path.clone()).unwrap();
        let (s, force) = a.login("admin", "packet").unwrap();
        assert!(!force && a.valid_session(&s));
        a.update(&s, "operator", "x").unwrap();
        assert!(a.valid_session(&s));
        assert!(a.login("operator", "x").is_some());
        let _ = std::fs::remove_file(path);
    }
}
