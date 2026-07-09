//! Tiny hand-rolled JSON helpers (docs/architecture/60-validation.md; the
//! dependency quarantine forbids serde). Shared by the trace writer and
//! the report's JSON renderer — one escaping rule, one number format —
//! plus a minimal parser for reading our own `report.json` back (the
//! cross-run merge, docs/silicon/00-baseline-and-harness.md).

use std::fmt::Write as _;

/// Appends `s` as a JSON string literal: quote, backslash, and control
/// characters escaped; non-ASCII passes through as UTF-8 (JSON is UTF-8
/// by definition).
pub fn push_str_lit(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Appends nanoseconds as microseconds with three decimals — the trace
/// writer's timestamp/duration format.
pub fn push_us(out: &mut String, ns: u64) {
    #[allow(clippy::cast_precision_loss)] // ns fit f64 exactly for ~104 days
    let us = ns as f64 / 1000.0;
    let _ = write!(out, "{us:.3}");
}

/// A parsed JSON value — just enough structure to read `report.json`.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(Vec<(String, Value)>),
}

impl Value {
    /// Object field lookup (first match).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Self::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The numeric value, if this is a number.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Num(n) => Some(*n),
            _ => None,
        }
    }

    /// The string value, if this is a string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The element list, if this is an array.
    #[must_use]
    pub fn as_arr(&self) -> Option<&[Value]> {
        match self {
            Self::Arr(items) => Some(items),
            _ => None,
        }
    }

    /// The boolean, if this is one.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Parses one JSON document.
///
/// # Errors
///
/// A message naming the byte offset of the offense.
pub fn parse(text: &str) -> Result<Value, String> {
    let bytes = text.as_bytes();
    let mut pos = 0;
    let value = parse_value(bytes, &mut pos)?;
    skip_ws(bytes, &mut pos);
    if pos != bytes.len() {
        return Err(format!("trailing content at byte {pos}"));
    }
    Ok(value)
}

fn skip_ws(bytes: &[u8], pos: &mut usize) {
    while let Some(b) = bytes.get(*pos) {
        if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
            *pos += 1;
        } else {
            break;
        }
    }
}

fn expect(bytes: &[u8], pos: &mut usize, want: u8) -> Result<(), String> {
    if bytes.get(*pos) == Some(&want) {
        *pos += 1;
        Ok(())
    } else {
        Err(format!("expected `{}` at byte {pos}", want as char))
    }
}

fn parse_value(bytes: &[u8], pos: &mut usize) -> Result<Value, String> {
    skip_ws(bytes, pos);
    match bytes.get(*pos) {
        Some(b'{') => parse_obj(bytes, pos),
        Some(b'[') => parse_arr(bytes, pos),
        Some(b'"') => parse_str(bytes, pos).map(Value::Str),
        Some(b't') => parse_lit(bytes, pos, "true", Value::Bool(true)),
        Some(b'f') => parse_lit(bytes, pos, "false", Value::Bool(false)),
        Some(b'n') => parse_lit(bytes, pos, "null", Value::Null),
        Some(_) => parse_num(bytes, pos),
        None => Err("unexpected end of input".to_owned()),
    }
}

fn parse_lit(bytes: &[u8], pos: &mut usize, lit: &str, value: Value) -> Result<Value, String> {
    if bytes[*pos..].starts_with(lit.as_bytes()) {
        *pos += lit.len();
        Ok(value)
    } else {
        Err(format!("bad literal at byte {pos}"))
    }
}

fn parse_num(bytes: &[u8], pos: &mut usize) -> Result<Value, String> {
    let start = *pos;
    while let Some(b) = bytes.get(*pos) {
        if matches!(b, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
            *pos += 1;
        } else {
            break;
        }
    }
    std::str::from_utf8(&bytes[start..*pos])
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .map(Value::Num)
        .ok_or_else(|| format!("bad number at byte {start}"))
}

fn parse_str(bytes: &[u8], pos: &mut usize) -> Result<String, String> {
    expect(bytes, pos, b'"')?;
    let mut out = String::new();
    loop {
        match bytes.get(*pos) {
            None => return Err("unterminated string".to_owned()),
            Some(b'"') => {
                *pos += 1;
                return Ok(out);
            }
            Some(b'\\') => {
                *pos += 1;
                match bytes.get(*pos) {
                    Some(b'"') => out.push('"'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'/') => out.push('/'),
                    Some(b'n') => out.push('\n'),
                    Some(b'r') => out.push('\r'),
                    Some(b't') => out.push('\t'),
                    Some(b'b') => out.push('\u{8}'),
                    Some(b'f') => out.push('\u{c}'),
                    Some(b'u') => {
                        let hex = bytes
                            .get(*pos + 1..*pos + 5)
                            .ok_or_else(|| "truncated \\u escape".to_owned())?;
                        let code = std::str::from_utf8(hex)
                            .ok()
                            .and_then(|h| u32::from_str_radix(h, 16).ok())
                            .ok_or_else(|| format!("bad \\u escape at byte {pos}"))?;
                        // Surrogates never appear in our own emitter's output.
                        out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        *pos += 4;
                    }
                    _ => return Err(format!("bad escape at byte {pos}")),
                }
                *pos += 1;
            }
            Some(_) => {
                // Consume one UTF-8 scalar (multi-byte sequences pass
                // through intact — we slice on char boundaries).
                let rest = std::str::from_utf8(&bytes[*pos..])
                    .map_err(|_| format!("invalid UTF-8 at byte {pos}"))?;
                let c = rest.chars().next().expect("non-empty");
                out.push(c);
                *pos += c.len_utf8();
            }
        }
    }
}

fn parse_arr(bytes: &[u8], pos: &mut usize) -> Result<Value, String> {
    expect(bytes, pos, b'[')?;
    let mut items = Vec::new();
    skip_ws(bytes, pos);
    if bytes.get(*pos) == Some(&b']') {
        *pos += 1;
        return Ok(Value::Arr(items));
    }
    loop {
        items.push(parse_value(bytes, pos)?);
        skip_ws(bytes, pos);
        match bytes.get(*pos) {
            Some(b',') => *pos += 1,
            Some(b']') => {
                *pos += 1;
                return Ok(Value::Arr(items));
            }
            _ => return Err(format!("expected `,` or `]` at byte {pos}")),
        }
    }
}

fn parse_obj(bytes: &[u8], pos: &mut usize) -> Result<Value, String> {
    expect(bytes, pos, b'{')?;
    let mut fields = Vec::new();
    skip_ws(bytes, pos);
    if bytes.get(*pos) == Some(&b'}') {
        *pos += 1;
        return Ok(Value::Obj(fields));
    }
    loop {
        skip_ws(bytes, pos);
        let key = parse_str(bytes, pos)?;
        skip_ws(bytes, pos);
        expect(bytes, pos, b':')?;
        fields.push((key, parse_value(bytes, pos)?));
        skip_ws(bytes, pos);
        match bytes.get(*pos) {
            Some(b',') => *pos += 1,
            Some(b'}') => {
                *pos += 1;
                return Ok(Value::Obj(fields));
            }
            _ => return Err(format!("expected `,` or `}}` at byte {pos}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(s: &str) -> String {
        let mut out = String::new();
        push_str_lit(&mut out, s);
        out
    }

    #[test]
    fn escaping_covers_the_documented_cases() {
        assert_eq!(lit("plain"), "\"plain\"");
        assert_eq!(lit("a\"b"), "\"a\\\"b\"");
        assert_eq!(lit("a\\b"), "\"a\\\\b\"");
        assert_eq!(lit("a\nb\tc\rd"), "\"a\\nb\\tc\\rd\"");
        assert_eq!(lit("\u{01}"), "\"\\u0001\"");
        // Non-ASCII memo content passes through as UTF-8.
        assert_eq!(lit("héllo — uniq-42"), "\"héllo — uniq-42\"");
    }

    #[test]
    fn microseconds_carry_three_decimals() {
        let mut out = String::new();
        push_us(&mut out, 1234);
        assert_eq!(out, "1.234");
        out.clear();
        push_us(&mut out, 15_000);
        assert_eq!(out, "15.000");
    }

    #[test]
    fn the_parser_round_trips_our_own_emission() {
        let text = r#"{"name":"a\"b","n":-1.5e2,"ok":true,"gone":null,"xs":[1,2,{"y":[]}]}"#;
        let v = parse(text).expect("parses");
        assert_eq!(v.get("name").and_then(Value::as_str), Some("a\"b"));
        assert_eq!(v.get("n").and_then(Value::as_f64), Some(-150.0));
        assert_eq!(v.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(v.get("gone"), Some(&Value::Null));
        let xs = v.get("xs").and_then(Value::as_arr).expect("array");
        assert_eq!(xs.len(), 3);
        assert_eq!(xs[2].get("y"), Some(&Value::Arr(vec![])));

        // Escapes emitted by push_str_lit parse back to the original.
        let mut emitted = String::new();
        push_str_lit(&mut emitted, "a\nb\t\"c\"\\ — héllo \u{01}");
        let back = parse(&emitted).expect("parses");
        assert_eq!(back.as_str(), Some("a\nb\t\"c\"\\ — héllo \u{01}"));

        assert!(parse("{\"a\":1} trailing").is_err());
        assert!(parse("[1,").is_err());
    }
}
