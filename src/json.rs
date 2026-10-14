//! Small output-only JSON encoder. Requests use bounded form fields, not a custom JSON parser.
pub fn string(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
pub fn array(values: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", values.into_iter().collect::<Vec<_>>().join(","))
}
pub fn object(values: impl IntoIterator<Item = (&'static str, String)>) -> String {
    format!(
        "{{{}}}",
        values
            .into_iter()
            .map(|(k, v)| format!("{}:{v}", string(k)))
            .collect::<Vec<_>>()
            .join(",")
    )
}
