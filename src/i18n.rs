//! UTF-8 catalogs: English source<TAB>translation. Only local UI text is passed here.
use std::{collections::BTreeMap, fs, path::Path, sync::Arc};
#[derive(Clone)]
pub struct I18n {
    entries: Arc<BTreeMap<String, String>>,
}
impl I18n {
    pub fn load(code: &str, config: &Path) -> Result<Self, String> {
        if code.is_empty()
            || code.len() > 16
            || !code.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
        {
            return Err("Invalid language code".into());
        }
        let mut entries = parse(include_str!("../languages/en.lang"))?;
        let builtin = match code {
            "en" => Some(include_str!("../languages/en.lang")),
            "pl" => Some(include_str!("../languages/pl.lang")),
            _ => None,
        };
        let path = config
            .parent()
            .unwrap_or(Path::new("."))
            .join("languages")
            .join(format!("{code}.lang"));
        let external = match fs::read_to_string(&path) {
            Ok(text) => Some(text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        if builtin.is_none() && external.is_none() {
            return Err(format!("Language not found: {}", path.display()));
        }
        if let Some(catalog) = builtin {
            for (key, value) in parse(catalog)? {
                if !entries.contains_key(&key) {
                    return Err(format!("Unknown translation key: {key}"));
                }
                entries.insert(key, value);
            }
        }
        // User catalogs can outlive a program update. Ignore keys removed from
        // newer UI versions so an obsolete translation cannot prevent startup.
        if let Some(catalog) = external.as_deref() {
            for (key, value) in parse(catalog)? {
                if entries.contains_key(&key) {
                    entries.insert(key, value);
                }
            }
        }
        Ok(Self {
            entries: Arc::new(entries),
        })
    }
    pub fn text(&self, source: &str) -> String {
        if let Some(value) = self.entries.get(source) {
            return value.clone();
        }
        for (pattern, translation) in self.entries.iter().filter(|(k, _)| k.contains("{0}")) {
            if let Some(args) = capture(pattern, source) {
                return expand(translation, &args);
            }
        }
        source.into()
    }
    pub fn lines(&self, source: &str) -> String {
        source
            .lines()
            .map(|line| self.text(line))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
fn tokens(text: &str) -> Result<Vec<usize>, String> {
    let mut ids = vec![];
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let end = rest[start..]
            .find('}')
            .ok_or("Unclosed translation placeholder")?
            + start;
        let id = rest[start + 1..end]
            .parse::<usize>()
            .map_err(|_| "Use numbered placeholders, e.g. {0}")?;
        if id > 9 {
            return Err("Translation placeholder out of range".into());
        }
        ids.push(id);
        rest = &rest[end + 1..];
    }
    ids.sort_unstable();
    Ok(ids)
}
fn parse(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut entries = BTreeMap::new();
    for (line_number, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('\t').ok_or_else(|| {
            format!(
                "Catalog line {}: expected source<TAB>translation",
                line_number + 1
            )
        })?;
        if value.is_empty() {
            continue;
        }
        if value.chars().any(char::is_control) || key.chars().any(char::is_control) {
            return Err("Control characters are not allowed in translations".into());
        }
        if tokens(key)? != tokens(value)? {
            return Err(format!("Translation placeholders differ: {key}"));
        }
        if entries.insert(key.into(), value.into()).is_some() {
            return Err(format!("Duplicate translation key: {key}"));
        }
    }
    Ok(entries)
}
fn capture(pattern: &str, source: &str) -> Option<Vec<String>> {
    let mut rest = source;
    let mut pattern = pattern;
    let mut args = vec![String::new(); 10];
    while let Some(start) = pattern.find('{') {
        rest = rest.strip_prefix(&pattern[..start])?;
        let end = pattern[start..].find('}')? + start;
        let id = pattern[start + 1..end].parse::<usize>().ok()?;
        pattern = &pattern[end + 1..];
        let next = pattern.find('{').unwrap_or(pattern.len());
        let separator = &pattern[..next];
        let n = if separator.is_empty() {
            if !pattern.is_empty() {
                return None;
            }
            rest.len()
        } else {
            rest.find(separator)?
        };
        args[id] = rest[..n].into();
        rest = &rest[n..];
    }
    if rest == pattern { Some(args) } else { None }
}
fn expand(pattern: &str, args: &[String]) -> String {
    let mut result = String::new();
    let mut rest = pattern;
    while let Some(start) = rest.find('{') {
        result.push_str(&rest[..start]);
        let end = rest[start..].find('}').unwrap() + start;
        let id = rest[start + 1..end].parse::<usize>().unwrap();
        result.push_str(&args[id]);
        rest = &rest[end + 1..];
    }
    result.push_str(rest);
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogs_and_fallback() {
        let en = parse(include_str!("../languages/en.lang")).unwrap();
        let pl = parse(include_str!("../languages/pl.lang")).unwrap();
        assert_eq!(en.keys().collect::<Vec<_>>(), pl.keys().collect::<Vec<_>>());
        let i = I18n::load("pl", Path::new("missing/config.conf")).unwrap();
        assert_eq!(i.text("Connected"), "Połączono");
        assert_eq!(i.text("raw peer text"), "raw peer text");
    }
    #[test]
    fn substitutions_are_not_retranslated() {
        let args = capture("Peer {0}, slot {1}", "Peer CONNECT, slot 2").unwrap();
        assert_eq!(expand("Sesja {1}: {0}", &args), "Sesja 2: CONNECT");
        assert!(parse("x {0}\tx {1}").is_err());
        assert!(parse("x\t\u{1b}[31m").is_err());
    }
    #[test]
    fn external_language_needs_no_code_changes_and_falls_back_to_english() {
        let root = std::path::PathBuf::from(format!("target/i18n-test-{}", std::process::id()));
        let folder = root.join("languages");
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("xx.lang");
        fs::write(
            &path,
            "Connected\tTest translation\nObsolete setting label\tOld translation\n",
        )
        .unwrap();
        let language = I18n::load("xx", &root.join("terminal.conf")).unwrap();
        assert_eq!(language.text("Connected"), "Test translation");
        assert_eq!(language.text("Connecting"), "Connecting");
        fs::remove_file(path).unwrap();
        fs::remove_dir(folder).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
