//! Original JavaScript Number/Number.toString semantics with explicit resource bounds.
use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

pub const MAX_FILE: usize = 8_000_000;
pub const MAX_RESULT: usize = 1_000_000;
pub const MAX_DIMENSION: f64 = 10_000.0;
pub fn allowed(path: &str) -> bool {
    [".jpeg", ".jpg", ".png", ".bmp", ".webp"]
        .iter()
        .any(|suffix| path.to_ascii_lowercase().ends_with(suffix))
}
pub fn prefill(reply: &Value) -> Value {
    match reply.as_array() {
        Some(a) if a.len() == 1 => match a[0].as_str() {
            Some(p) if p.len() <= 4096 && std::path::Path::new(p).is_absolute() && allowed(p) => {
                json!([p])
            }
            _ => json!([]),
        },
        _ => json!([]),
    }
}
fn js_space(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
pub fn number(text: &str) -> f64 {
    let text = text.trim_matches(js_space);
    if text.is_empty() {
        return 0.0;
    }
    match text {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    if text.len() >= 2 && text.starts_with('0') {
        let radix = match text.as_bytes()[1] {
            b'x' | b'X' => 16,
            b'o' | b'O' => 8,
            b'b' | b'B' => 2,
            _ => 0,
        };
        if radix != 0 {
            let digits = &text[2..];
            if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
                return f64::NAN;
            }
            // Numbers within the accepted dimension budget are exactly accumulated;
            // very large radix strings are rejected by the explicit upper bound.
            return digits.chars().fold(0.0, |v, c| {
                v * radix as f64 + c.to_digit(radix).unwrap() as f64
            });
        }
    }
    static DECIMAL: OnceLock<Regex> = OnceLock::new();
    let re = DECIMAL.get_or_init(|| {
        Regex::new(r"^[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$").unwrap()
    });
    if !re.is_match(text) {
        return f64::NAN;
    }
    text.parse().unwrap_or(f64::NAN)
}
pub fn dimension(text: &str, name: &str) -> Result<Option<String>, String> {
    if text.trim_matches(js_space).is_empty() {
        return Ok(None);
    }
    let value = number(text);
    if value.is_nan() || value <= 0.0 {
        return Err(format!("{name} must be a positive number."));
    }
    if !value.is_finite() || value > MAX_DIMENSION {
        return Err(format!(
            "{name} must be finite and at most 10,000 for this upload."
        ));
    }
    Ok(Some(ryu_js::Buffer::new().format(value).to_owned()))
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub image: Vec<String>,
    pub width: String,
    pub height: String,
    pub complex: bool,
}
impl Default for Draft {
    fn default() -> Self {
        Self {
            image: vec![],
            width: "80".into(),
            height: String::new(),
            complex: false,
        }
    }
}
impl Draft {
    pub fn budget(&self) -> bool {
        self.image.len() <= 1
            && self
                .image
                .iter()
                .all(|p| p.len() <= 4096 && !p.contains('\0'))
            && self.width.len() <= 256
            && self.height.len() <= 256
    }
}
pub fn decode(body: &[u8]) -> Result<String, &'static str> {
    if body.len() > MAX_RESULT {
        return Err("Service response exceeds 1,000,000 bytes.");
    }
    // Axios' default forcedJSONParsing accepts JSON regardless of Content-Type.
    // Reject non-string JSON: it cannot be copied as exact ASCII text.
    let text =
        std::str::from_utf8(body).map_err(|_| "Service response is not valid UTF-8 text.")?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    match serde_json::from_str::<Box<serde_json::value::RawValue>>(text) {
        Ok(raw) if raw.get().starts_with('"') => serde_json::from_str::<String>(raw.get())
            .map_err(|_| "Service returned text with unsupported unpaired Unicode surrogates."),
        Ok(_) => Err("Service returned JSON that is not text."),
        Err(_) => Ok(text.to_owned()),
    }
}
pub fn fence(text: &str) -> String {
    let max = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let marker = "`".repeat(max.max(2) + 1);
    format!("{marker}\n{text}\n{marker}")
}
