//! Bounded list of AX.25 source addresses observed on received KISS data frames.
use crate::ax25::{Call, Frame};
use std::{
    collections::VecDeque,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub const MAX_HEARD: usize = 256;
pub const RETENTION_SECONDS: u64 = 3 * 60 * 60;

#[derive(Clone)]
pub struct Entry {
    pub source: Call,
    pub interface: usize,
    pub port: u8,
    pub frames: u64,
    pub last_ms: u64,
    pub last_utc: u64,
    pub destination: Call,
    pub path: Vec<(Call, bool)>,
    /// Monitor-style preview, kept only in memory; older persisted entries have none.
    pub last_frame: String,
    persisted: bool,
}
impl Entry {
    pub fn age_seconds(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_ms) / 1000
    }
    pub fn age_seconds_at(&self, now_ms: u64, now_utc: u64) -> u64 {
        if self.persisted {
            now_utc.saturating_sub(self.last_utc)
        } else {
            self.age_seconds(now_ms)
        }
    }
    pub fn age_label(&self, now: u64) -> String {
        let seconds = self.age_seconds(now);
        if seconds < 60 {
            format!("{seconds}s")
        } else {
            format!("{}min", seconds / 60)
        }
    }
    pub fn utc(&self) -> String {
        let t = self.last_utc % 86400;
        format!("{:02}:{:02}:{:02}", t / 3600, (t / 60) % 60, t % 60)
    }
    pub fn route(&self) -> String {
        self.path
            .iter()
            .map(|(call, repeated)| format!("{call}{}", if *repeated { "*" } else { "" }))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Default)]
pub struct Heard {
    entries: VecDeque<Entry>,
}
impl Heard {
    pub fn entries(&self) -> &VecDeque<Entry> {
        &self.entries
    }
    pub fn record(&mut self, frame: &Frame, port: u8, local: &Call, now: u64) {
        self.record_on(frame, port, 0, local, now);
    }
    /// Return whether the source was absent from all recent entries, across interfaces and ports.
    pub fn record_on(&mut self, frame: &Frame, port: u8, interface: usize, local: &Call, now: u64) -> bool {
        if frame.source == *local {
            return false;
        }
        let now_utc = Self::current_utc();
        let new_source = !self.entries.iter().any(|entry| {
            entry.source == frame.source
                && now_utc.saturating_sub(entry.last_utc) < RETENTION_SECONDS
        });
        let previous = self
            .entries
            .iter()
            .position(|e| e.source == frame.source && e.port == port && e.interface == interface)
            .and_then(|i| self.entries.remove(i));
        let frames = previous.map_or(1, |e| e.frames.saturating_add(1));
        // One bounded preview per entry, not a second unbounded frame history.
        let mut payload = String::from_utf8_lossy(&frame.info)
            .chars()
            .take(4096)
            .collect::<String>();
        if String::from_utf8_lossy(&frame.info).chars().count() > 4096 {
            payload.push_str(" [preview truncated / podgląd skrócony]");
        }
        let last_frame = format!(
            "{}\n{}",
            frame.summary(),
            payload.replace('\r', "<CR>").replace('\n', "<LF>")
        );
        self.entries.push_front(Entry {
            source: frame.source.clone(),
            interface,
            port,
            frames,
            last_ms: now,
            last_utc: now_utc,
            destination: frame.destination.clone(),
            path: frame.path.clone(),
            last_frame,
            persisted: false,
        });
        self.entries.truncate(MAX_HEARD);
        new_source
    }

    /// Load recent entries. Invalid lines and removed interfaces are ignored.
    pub fn load(path: &Path, now_utc: u64, interface_count: usize) -> Self {
        let mut heard = Self::default();
        let Ok(text) = fs::read_to_string(path) else {
            return heard;
        };
        for line in text.lines() {
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != 7 {
                continue;
            }
            let (Ok(interface), Ok(port), Ok(frames), Ok(last_utc)) = (
                fields[1].parse::<usize>(),
                fields[2].parse::<u8>(),
                fields[3].parse::<u64>(),
                fields[4].parse::<u64>(),
            ) else {
                continue;
            };
            if interface >= interface_count
                || frames == 0
                || last_utc == 0
                || now_utc.saturating_sub(last_utc) >= RETENTION_SECONDS
            {
                continue;
            }
            let (Ok(source), Ok(destination)) =
                (fields[0].parse::<Call>(), fields[5].parse::<Call>())
            else {
                continue;
            };
            let mut path_entries = Vec::new();
            let mut valid_path = true;
            for item in fields[6].split_whitespace().filter(|item| !item.is_empty()) {
                let repeated = item.ends_with('*');
                let call = if repeated {
                    &item[..item.len() - 1]
                } else {
                    item
                };
                match call.parse::<Call>() {
                    Ok(call) => path_entries.push((call, repeated)),
                    Err(_) => {
                        valid_path = false;
                        break;
                    }
                }
            }
            if !valid_path {
                continue;
            }
            heard.entries.push_back(Entry {
                source,
                interface,
                port,
                frames,
                last_ms: 0,
                last_utc,
                destination,
                path: path_entries,
                last_frame: String::new(),
                persisted: true,
            });
        }
        heard.entries.truncate(MAX_HEARD);
        heard
    }

    pub fn prune(&mut self, now_utc: u64, interface_count: usize) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| {
            entry.interface < interface_count
                && now_utc.saturating_sub(entry.last_utc) < RETENTION_SECONDS
        });
        before != self.entries.len()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let text = self
            .entries
            .iter()
            .map(|entry| {
                let route = entry
                    .path
                    .iter()
                    .map(|(call, repeated)| format!("{call}{}", if *repeated { "*" } else { "" }))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    entry.source,
                    entry.interface,
                    entry.port,
                    entry.frames,
                    entry.last_utc,
                    entry.destination,
                    route
                )
            })
            .collect::<String>();
        crate::storage::write_atomic(path, text.as_bytes())
    }

    pub fn current_utc() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn frame(source: &str) -> Frame {
        Frame {
            source: source.parse().unwrap(),
            destination: "OTHER".parse().unwrap(),
            path: vec![("DIGI".parse().unwrap(), true)],
            command: true,
            response: false,
            control: 3,
            extended: false,
            pid: Some(240),
            info: vec![],
        }
    }
    #[test]
    fn tracks_sources_ssids_counts_routes_and_ports() {
        let mut h = Heard::default();
        let local = "LOCAL".parse().unwrap();
        h.record(&frame("PEER"), 0, &local, 1000);
        h.record(&frame("PEER-1"), 0, &local, 2000);
        h.record(&frame("PEER"), 0, &local, 4000);
        assert_eq!(h.entries.len(), 2);
        let e = &h.entries[0];
        assert_eq!(e.source.to_string(), "PEER");
        assert_eq!(e.frames, 2);
        assert_eq!(e.age_seconds(6500), 2);
        assert_eq!(e.age_label(65000), "1min");
        assert_eq!(e.route(), "DIGI*");
        assert_eq!(e.destination.to_string(), "OTHER");
        h.record(&frame("PEER"), 1, &local, 5000);
        assert_eq!(h.entries.len(), 3);
        h.record(&frame("LOCAL"), 0, &local, 6000);
        assert_eq!(h.entries.len(), 3);
    }
    #[test]
    fn new_source_detection_spans_interfaces_ports_and_retention() {
        let mut heard = Heard::default();
        let local = "LOCAL".parse().unwrap();
        assert!(!heard.record_on(&frame("LOCAL"), 0, 0, &local, 0));
        assert!(heard.record_on(&frame("PEER-1"), 0, 0, &local, 0));
        assert!(!heard.record_on(&frame("PEER-1"), 0, 0, &local, 1));
        assert!(!heard.record_on(&frame("PEER-1"), 1, 1, &local, 2));
        assert!(heard.record_on(&frame("PEER-2"), 0, 0, &local, 3));
        heard.entries[1].last_utc = Heard::current_utc() - RETENTION_SECONDS;
        // A still-recent reception on the first interface suppresses duplicates.
        assert!(!heard.record_on(&frame("PEER-1"), 1, 1, &local, 4));
        for entry in &mut heard.entries {
            entry.last_utc = Heard::current_utc() - RETENTION_SECONDS;
        }
        // No prune is required before a returning source can be detected.
        assert!(heard.record_on(&frame("PEER-1"), 0, 0, &local, 5));
        assert!(!heard.record_on(&frame("PEER-1"), 1, 1, &local, 6));
    }

    #[test]
    fn loaded_sources_are_not_new_but_evicted_sources_are() {
        let root = crate::test_temp_dir("mheard-new-source");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("mheard.dat");
        let local = "LOCAL".parse().unwrap();
        let mut heard = Heard::default();
        assert!(heard.record_on(&frame("PEER"), 0, 0, &local, 0));
        heard.save(&path).unwrap();
        let mut loaded = Heard::load(&path, Heard::current_utc(), 2);
        assert!(!loaded.record_on(&frame("PEER"), 1, 1, &local, 1));
        for n in 0..MAX_HEARD {
            loaded.record_on(&frame(&format!("T{n}")), 0, 0, &local, 2);
        }
        assert!(loaded.record_on(&frame("PEER"), 0, 0, &local, 3));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn oldest_entries_are_evicted() {
        let mut h = Heard::default();
        let local = "LOCAL".parse().unwrap();
        for n in 0..300 {
            h.record(&frame(&format!("T{n}")), 0, &local, n);
        }
        assert_eq!(h.entries.len(), MAX_HEARD);
        assert_eq!(h.entries.front().unwrap().source.to_string(), "T299");
        assert_eq!(h.entries.back().unwrap().source.to_string(), "T44");
    }
    #[test]
    fn persists_recent_entries_and_discards_old_or_invalid_records() {
        let root = crate::test_temp_dir("ultimatepr-mheard");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let path = root.join("mheard.dat");
        let now = 20_000;
        let mut h = Heard::default();
        let local = "LOCAL".parse().unwrap();
        h.record_on(&frame("PEER"), 2, 0, &local, 5000);
        h.entries[0].last_utc = now - 60;
        h.save(&path).unwrap();
        fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(
                format!(
                    "BROKEN\tline\nPEER\t0\t2\t1\t{}\tOTHER\t\n",
                    now - RETENTION_SECONDS
                )
                .as_bytes(),
            )
            .unwrap();
        let loaded = Heard::load(&path, now, 1);
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].source.to_string(), "PEER");
        assert_eq!(loaded.entries[0].frames, 1);
        assert_eq!(loaded.entries[0].age_seconds_at(0, now), 60);
        assert!(loaded.entries[0].last_frame.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn keeps_latest_packet_frame_preview_with_full_path_and_bounded_payload() {
        let mut heard = Heard::default();
        let local = "LOCAL".parse().unwrap();
        let mut packet = frame("PEER");
        packet.path = ["DIGA", "DIGB", "DIGC", "DIGD"]
            .into_iter()
            .map(|call| (call.parse().unwrap(), true))
            .collect();
        packet.path.push(("UNUSED".parse().unwrap(), false));
        packet.control = 0x46; // I N(S)=3 N(R)=2, modulo 8.
        packet.info = b"Hello\r\n<packet>".to_vec();
        heard.record_on(&packet, 2, 1, &local, 1000);
        let entry = &heard.entries()[0];
        assert_eq!(entry.route(), "DIGA* DIGB* DIGC* DIGD* UNUSED");
        assert!(entry.last_frame.contains("I N(S)=3 N(R)=2"));
        assert!(entry.last_frame.contains("DIGA* DIGB* DIGC* DIGD* UNUSED"));
        assert!(entry.last_frame.contains("Hello<CR><LF><packet>"));
        packet.control = 0x41; // RR, no information field.
        packet.info.clear();
        heard.record_on(&packet, 2, 1, &local, 2000);
        assert_eq!(heard.entries().len(), 1);
        assert!(heard.entries()[0].last_frame.contains("RR N(R)=2"));
        assert!(!heard.entries()[0].last_frame.contains("Hello"));
        packet.control = 0x2f; // SABM.
        heard.record_on(&packet, 2, 1, &local, 3000);
        assert!(heard.entries()[0].last_frame.contains("SABM"));
        packet.info = "ż".repeat(5000).into_bytes();
        heard.record_on(&packet, 2, 1, &local, 4000);
        assert_eq!(heard.entries()[0].last_frame.matches('ż').count(), 4096);
        assert!(heard.entries()[0].last_frame.contains("podgląd skrócony"));
    }
}
