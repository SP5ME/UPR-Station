use crate::json;
use std::collections::VecDeque;

/// Bounded UTF-8 stream history; frame boundaries never become text boundaries.
#[derive(Default)]
pub struct History {
    lines: VecDeque<(u64, String, String)>,
    current: String,
    utf8: Vec<u8>,
    cr: bool,
    sequence: u64,
}
impl History {
    pub fn note(&mut self, kind: &str, text: &str) {
        self.finish();
        for line in text.lines() {
            self.push(kind, line.to_owned());
        }
    }
    fn push(&mut self, kind: &str, text: String) {
        self.sequence += 1;
        self.lines.push_back((self.sequence, kind.into(), text));
        while self.lines.len() > 500 {
            self.lines.pop_front();
        }
    }
    fn finish(&mut self) {
        if !self.current.is_empty() {
            let s = std::mem::take(&mut self.current);
            self.push("rx", s);
        }
    }
    pub fn data(&mut self, data: &[u8]) {
        self.utf8.extend_from_slice(data);
        let mut consumed = 0;
        let mut text = String::new();
        while consumed < self.utf8.len() {
            match std::str::from_utf8(&self.utf8[consumed..]) {
                Ok(s) => {
                    text.push_str(s);
                    consumed = self.utf8.len();
                }
                Err(e) => {
                    text.push_str(
                        std::str::from_utf8(&self.utf8[consumed..consumed + e.valid_up_to()])
                            .unwrap_or_default(),
                    );
                    consumed += e.valid_up_to();
                    if let Some(n) = e.error_len() {
                        text.push('\u{fffd}');
                        consumed += n;
                    } else {
                        break;
                    }
                }
            }
        }
        self.utf8.drain(..consumed);
        for c in text.chars() {
            if c == '\n' && self.cr {
                self.cr = false;
                continue;
            }
            self.cr = c == '\r';
            if c == '\r' || c == '\n' {
                let s = std::mem::take(&mut self.current);
                self.push("rx", s);
            } else if !c.is_control() || c == '\t' {
                self.current.push(c);
                if self.current.len() >= 2048 {
                    self.finish();
                }
            }
        }
    }
    pub fn clear(&mut self) {
        self.lines.clear();
        self.current.clear();
        self.utf8.clear();
    }
    pub fn json(&self) -> String {
        let mut rows: Vec<_> = self
            .lines
            .iter()
            .map(|(id, kind, text)| {
                json::object([
                    ("id", id.to_string()),
                    ("kind", json::string(kind)),
                    ("text", json::string(text)),
                ])
            })
            .collect();
        if !self.current.is_empty() {
            rows.push(json::object([
                ("id", (self.sequence + 1).to_string()),
                ("kind", json::string("rx")),
                ("text", json::string(&self.current)),
            ]));
        }
        json::array(rows)
    }
    pub fn text(&self) -> String {
        let mut out = self
            .lines
            .iter()
            .map(|(_, _, s)| s.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if !self.current.is_empty() {
            out.push('\n');
            out.push_str(&self.current);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_utf8_and_crlf() {
        let mut h = History::default();
        h.data(&[0xc5]);
        h.data(&[0xbc, 13]);
        h.data(b"\nhello");
        assert_eq!(h.text(), "ż\nhello");
        for _ in 0..600 {
            h.data(b"x\n");
        }
        assert_eq!(h.lines.len(), 500);
    }
}
