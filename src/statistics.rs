use crate::json;
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const DEFAULT_RETENTION_DAYS: u64 = 30;

#[derive(Default, Clone)]
struct Day {
    date: String,
    rx: u64,
    tx: u64,
    sessions: u64,
    stations: BTreeMap<String, u64>,
    interfaces: BTreeMap<String, [u64; 2]>,
    routes: BTreeMap<String, u64>,
}

pub struct Statistics {
    pub enabled: bool,
    path: Option<PathBuf>,
    days: VecDeque<Day>,
    retention_days: Option<u64>,
    last_persist: Instant,
}

fn today() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86400;
    format!("{days}")
}

impl Statistics {
    pub fn load(path: PathBuf, enabled: bool) -> Self {
        Self::load_with_retention(path, enabled, Some(DEFAULT_RETENTION_DAYS))
    }
    pub fn load_with_retention(
        path: PathBuf,
        enabled: bool,
        retention_days: Option<u64>,
    ) -> Self {
        let mut s = Self {
            enabled,
            path: Some(path.clone()),
            days: VecDeque::new(),
            retention_days,
            // Avoid an Instant underflow during the first seconds after boot.
            last_persist: Instant::now()
                .checked_sub(Duration::from_secs(10))
                .unwrap_or_else(Instant::now),
        };
        if enabled {
            if let Ok(text) = fs::read_to_string(&path) {
                for line in text.lines() {
                    let p: Vec<_> = line.split('\t').collect();
                    if p.len() < 5 {
                        continue;
                    }
                    let mut d = Day {
                        date: p[0].into(),
                        rx: p[1].parse().unwrap_or(0),
                        tx: p[2].parse().unwrap_or(0),
                        sessions: p[3].parse().unwrap_or(0),
                        ..Default::default()
                    };
                    for item in p[4].split(';').filter(|x| !x.is_empty()) {
                        if let Some((k, v)) = item.split_once('=') {
                            d.stations.insert(k.into(), v.parse().unwrap_or(0));
                        }
                    }
                    if let Some(items) = p.get(5) {
                        for item in items.split(';').filter(|x| !x.is_empty()) {
                            let q: Vec<_> = item.split(':').collect();
                            if q.len() == 3 {
                                d.interfaces.insert(
                                    q[0].into(),
                                    [q[1].parse().unwrap_or(0), q[2].parse().unwrap_or(0)],
                                );
                            }
                        }
                    }
                    if let Some(items) = p.get(6) {
                        for item in items.split(';').filter(|x| !x.is_empty()) {
                            if let Some((k, v)) = item.split_once('=') {
                                d.routes.insert(k.into(), v.parse().unwrap_or(0));
                            }
                        }
                    }
                    s.days.push_back(d);
                }
            }
            s.prune();
        }
        s
    }
    fn current(&mut self) -> &mut Day {
        let date = today();
        if self.days.back().map(|d| d.date.as_str()) != Some(date.as_str()) {
            self.days.push_back(Day {
                date,
                ..Default::default()
            });
        }
        self.days.back_mut().unwrap()
    }
    fn prune(&mut self) {
        if let Some(retention_days) = self.retention_days {
            while self.days.len() > retention_days as usize {
                self.days.pop_front();
            }
        }
    }
    pub fn set_retention_days(&mut self, retention_days: Option<u64>) {
        self.retention_days = retention_days;
        self.prune();
        self.save(true);
    }
    pub fn retention_days(&self) -> Option<u64> {
        self.retention_days
    }
    fn save(&mut self, force: bool) {
        if !self.enabled {
            return;
        }
        if !force && self.last_persist.elapsed() < Duration::from_secs(5) {
            return;
        }
        self.prune();
        let text = self
            .days
            .iter()
            .map(|d| {
                format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    d.date,
                    d.rx,
                    d.tx,
                    d.sessions,
                    d.stations
                        .iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                        .join(";"),
                    d.interfaces
                        .iter()
                        .map(|(k, v)| format!("{}:{}:{}", k, v[0], v[1]))
                        .collect::<Vec<_>>()
                        .join(";"),
                    d.routes
                        .iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                        .join(";")
                )
            })
            .collect::<String>();
        if let Some(path) = &self.path {
            let _ = crate::storage::write_atomic(path, text.as_bytes());
        }
        self.last_persist = Instant::now();
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.days.clear();
            if let Some(path) = &self.path {
                let _ = fs::remove_file(path);
            }
        } else {
            self.save(true);
        }
    }
    pub fn flush(&mut self) {
        self.save(true);
    }
    pub fn rx(&mut self, station: &str, interface: &str, route: &str) {
        if !self.enabled {
            return;
        }
        let d = self.current();
        d.rx += 1;
        *d.stations.entry(station.into()).or_default() += 1;
        *d.interfaces
            .entry(interface.into())
            .or_default()
            .first_mut()
            .unwrap() += 1;
        if !route.is_empty() {
            *d.routes.entry(route.into()).or_default() += 1;
        }
        self.save(false);
    }
    pub fn tx(&mut self, interface: &str) {
        if !self.enabled {
            return;
        }
        let d = self.current();
        d.tx += 1;
        d.interfaces.entry(interface.into()).or_default()[1] += 1;
        self.save(false);
    }
    pub fn session(&mut self) {
        if self.enabled {
            self.current().sessions += 1;
            self.save(false);
        }
    }
    pub fn snapshot(&self) -> String {
        if !self.enabled {
            return json::object([
                ("enabled", "false".into()),
                ("days", "[]".into()),
                ("stations", "[]".into()),
                ("interfaces", "[]".into()),
            ]);
        }
        let days = json::array(self.days.iter().map(|d| {
            json::object([
                ("date", json::string(&d.date)),
                ("rx", d.rx.to_string()),
                ("tx", d.tx.to_string()),
                ("sessions", d.sessions.to_string()),
                ("stations", d.stations.len().to_string()),
            ])
        }));
        let mut station_totals: BTreeMap<String, u64> = BTreeMap::new();
        let mut interface_totals: BTreeMap<String, [u64; 2]> = BTreeMap::new();
        for d in &self.days {
            for (k, v) in &d.stations {
                *station_totals.entry(k.clone()).or_default() += v;
            }
            for (k, v) in &d.interfaces {
                let x = interface_totals.entry(k.clone()).or_default();
                x[0] += v[0];
                x[1] += v[1];
            }
        }
        let stations = json::array(station_totals.into_iter().map(|(call, frames)| {
            json::object([
                ("call", json::string(&call)),
                ("frames", frames.to_string()),
            ])
        }));
        let interfaces = json::array(interface_totals.into_iter().map(|(name, v)| {
            json::object([
                ("name", json::string(&name)),
                ("rx", v[0].to_string()),
                ("tx", v[1].to_string()),
            ])
        }));
        json::object([
            ("enabled", "true".into()),
            ("days", days),
            ("stations", stations),
            ("interfaces", interfaces),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::Statistics;
    use std::{fs, path::PathBuf};
    #[test]
    fn disabled_stats_delete_history_and_emit_disabled_snapshot() {
        let path = PathBuf::from(format!("target/statistics-{}.json", std::process::id()));
        let mut stats = Statistics::load(path.clone(), true);
        stats.rx("PEER", "TNC", "DIGI*");
        assert!(path.exists());
        stats.set_enabled(false);
        assert!(!path.exists());
        assert!(stats.snapshot().contains("\"enabled\":false"));
        let _ = fs::remove_file(path);
    }
    #[test]
    fn enabled_stats_aggregate_frames_and_interface() {
        let path = PathBuf::from(format!("target/statistics-{}-2.json", std::process::id()));
        let mut stats = Statistics::load(path.clone(), true);
        stats.rx("PEER", "TNC", "");
        stats.rx("PEER", "TNC", "");
        stats.tx("TNC");
        stats.session();
        let output = stats.snapshot();
        assert!(output.contains("\"frames\":2"));
        assert!(output.contains("\"rx\":2"));
        let _ = fs::remove_file(path);
    }
}
