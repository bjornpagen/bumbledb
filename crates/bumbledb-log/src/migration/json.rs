//! One strict JSON reader/writer for every repo-boundary migration artifact:
//! schema snapshots ([`crate::schema_file`]), canonical plans and manifests
//! ([`super::plan`], [`super::manifest`]).
//!
//! Reading is a bounded grammar over exactly the JSON this tooling emits —
//! objects, arrays, strings, `u64` numbers, booleans, null. Numbers wider
//! than u64 and floats are never bare JSON numbers: every scalar value uses
//! the one canonical value spelling below, so no reader ever guesses a
//! float's bits or a bigint's width from decimal text.
//!
//! Writing is deterministic: fixed key order chosen by each renderer, two
//! space indentation, `\n` newlines. Formatting can never change identity —
//! digests hash canonical FRAMES built from parsed data, not JSON text.
//!
//! The one canonical value spelling (shared by schema extension rows, plan
//! literals and seed rows):
//! `{"bool":true}`, `{"u64":"7"}`, `{"i64":"-7"}`,
//! `{"$f64":"<16 lowercase hex canonical bits>"}`, `{"id128":"<32 hex>"}`,
//! `{"string":"…"}`, `{"fixedBytes":"<hex>"}`, `{"intervalU64":["a","b"]}`,
//! `{"intervalI64":["a","b"]}`, `{"intervalF64":["<16hex>","<16hex>"]}`.

use std::collections::BTreeMap;
use std::ops::Index;

use bumbledb::{F64, Id128, Interval, Value};

/// A strict-grammar refusal: the static message names the first offense.
pub(crate) type JsonResult<T> = Result<T, &'static str>;

pub(crate) enum Json {
    Null,
    Bool(bool),
    U64(u64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

const NULL: Json = Json::Null;

impl Json {
    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub(crate) fn as_u64(&self) -> Option<u64> {
        match self {
            Self::U64(value) => Some(*value),
            _ => None,
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn as_array(&self) -> Option<&Vec<Json>> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn as_object(&self) -> Option<&BTreeMap<String, Json>> {
        match self {
            Self::Object(value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn get(&self, key: &str) -> Option<&Json> {
        self.as_object().and_then(|object| object.get(key))
    }

    pub(crate) const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

impl Index<&str> for Json {
    type Output = Json;

    fn index(&self, key: &str) -> &Json {
        self.get(key).unwrap_or(&NULL)
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

/// Parse one complete JSON document; trailing bytes refuse.
pub(crate) fn read_tree(raw: &str) -> JsonResult<Json> {
    let mut cur = Cursor {
        bytes: raw.as_bytes(),
        at: 0,
    };
    let value = cur.value()?;
    cur.skip_ws();
    if cur.at != cur.bytes.len() {
        return Err("trailing");
    }
    Ok(value)
}

impl Cursor<'_> {
    fn skip_ws(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn bump(&mut self) -> JsonResult<u8> {
        let byte = self.peek().ok_or("truncated")?;
        self.at += 1;
        Ok(byte)
    }

    fn eat(&mut self, want: u8) -> JsonResult<()> {
        if self.bump()? == want {
            Ok(())
        } else {
            Err("token")
        }
    }

    fn lit(&mut self, want: &[u8]) -> JsonResult<()> {
        if self
            .bytes
            .get(self.at..)
            .is_some_and(|rest| rest.starts_with(want))
        {
            self.at += want.len();
            if matches!(
                self.peek(),
                Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
            ) {
                return Err("token");
            }
            Ok(())
        } else {
            Err("token")
        }
    }

    fn value(&mut self) -> JsonResult<Json> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => Ok(Json::Object(self.object()?)),
            Some(b'[') => Ok(Json::Array(self.array()?)),
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b't') => {
                self.lit(b"true")?;
                Ok(Json::Bool(true))
            }
            Some(b'f') => {
                self.lit(b"false")?;
                Ok(Json::Bool(false))
            }
            Some(b'n') => {
                self.lit(b"null")?;
                Ok(Json::Null)
            }
            Some(b'0'..=b'9') => Ok(Json::U64(self.number()?)),
            _ => Err("value"),
        }
    }

    fn object(&mut self) -> JsonResult<BTreeMap<String, Json>> {
        self.eat(b'{')?;
        self.skip_ws();
        let mut map = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(map);
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.eat(b':')?;
            let value = self.value()?;
            map.insert(key, value);
            self.skip_ws();
            match self.bump()? {
                b',' => {}
                b'}' => return Ok(map),
                _ => return Err("object"),
            }
        }
    }

    fn array(&mut self) -> JsonResult<Vec<Json>> {
        self.eat(b'[')?;
        self.skip_ws();
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(items);
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.bump()? {
                b',' => {}
                b']' => return Ok(items),
                _ => return Err("array"),
            }
        }
    }

    fn string(&mut self) -> JsonResult<String> {
        self.eat(b'"')?;
        let mut out = Vec::new();
        loop {
            match self.bump()? {
                b'"' => {
                    return String::from_utf8(out).map_err(|_| "utf8");
                }
                b'\\' => match self.bump()? {
                    b'"' => out.push(b'"'),
                    b'\\' => out.push(b'\\'),
                    b'/' => out.push(b'/'),
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'b' => out.push(0x08),
                    b'f' => out.push(0x0c),
                    b'u' => {
                        let scalar = self.hex4()?;
                        let ch = char::from_u32(scalar).ok_or("unicode")?;
                        let mut buf = [0; 4];
                        out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                    }
                    _ => return Err("escape"),
                },
                byte if byte < 0x20 => return Err("control"),
                byte => out.push(byte),
            }
        }
    }

    fn hex4(&mut self) -> JsonResult<u32> {
        let mut n = 0u32;
        for _ in 0..4 {
            let digit = match self.bump()? {
                b @ b'0'..=b'9' => u32::from(b - b'0'),
                b @ b'a'..=b'f' => u32::from(b - b'a' + 10),
                b @ b'A'..=b'F' => u32::from(b - b'A' + 10),
                _ => return Err("hex"),
            };
            n = (n << 4) | digit;
        }
        Ok(n)
    }

    fn number(&mut self) -> JsonResult<u64> {
        let start = self.at;
        match self.bump()? {
            b'0' => {
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err("number");
                }
            }
            b'1'..=b'9' => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.at += 1;
                }
            }
            _ => return Err("number"),
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E' | b'-' | b'+')) {
            return Err("number");
        }
        let text = std::str::from_utf8(&self.bytes[start..self.at]).map_err(|_| "number")?;
        text.parse().map_err(|_| "number")
    }
}

// ---------------------------------------------------------------------------
// Shared canonical value spelling.
// ---------------------------------------------------------------------------

/// Parse the one canonical value object. Exactly one arm; the fresh spelling
/// does not exist.
pub(crate) fn parse_value(json: &Json) -> JsonResult<Value> {
    let object = json.as_object().ok_or("value")?;
    if object.len() != 1 {
        return Err("value arm");
    }
    let (kind, body) = object.iter().next().ok_or("value arm")?;
    match kind.as_str() {
        "bool" => Ok(Value::Bool(body.as_bool().ok_or("bool")?)),
        "u64" => Ok(Value::U64(parse_u64(body)?)),
        "i64" => Ok(Value::I64(parse_i64(body)?)),
        "$f64" => Ok(Value::F64(parse_f64_bits(body)?)),
        "id128" => {
            let raw = unhex_exact::<16>(body.as_str().ok_or("id128 hex")?)?;
            Ok(Value::Id128(Id128::from_bytes(raw)))
        }
        "string" => Ok(Value::String(body.as_str().ok_or("string")?.into())),
        "fixedBytes" => Ok(Value::FixedBytes(
            unhex(body.as_str().ok_or("hex")?)?.into_boxed_slice(),
        )),
        "intervalU64" => {
            let pair = pair2(body)?;
            Interval::new(parse_u64(&pair[0])?, parse_u64(&pair[1])?)
                .map(Value::IntervalU64)
                .ok_or("interval")
        }
        "intervalI64" => {
            let pair = pair2(body)?;
            Interval::new(parse_i64(&pair[0])?, parse_i64(&pair[1])?)
                .map(Value::IntervalI64)
                .ok_or("interval")
        }
        "intervalF64" => {
            let pair = pair2(body)?;
            Interval::new(parse_f64_bits(&pair[0])?, parse_f64_bits(&pair[1])?)
                .map(Value::IntervalF64)
                .ok_or("interval")
        }
        _ => Err("unknown value arm"),
    }
}

/// Render the one canonical value object (single line; the caller indents).
pub(crate) fn render_value(out: &mut String, value: &Value) {
    match value {
        Value::Bool(v) => {
            out.push_str(if *v {
                "{\"bool\":true}"
            } else {
                "{\"bool\":false}"
            });
        }
        Value::U64(v) => {
            out.push_str("{\"u64\":\"");
            out.push_str(&v.to_string());
            out.push_str("\"}");
        }
        Value::I64(v) => {
            out.push_str("{\"i64\":\"");
            out.push_str(&v.to_string());
            out.push_str("\"}");
        }
        Value::F64(v) => {
            out.push_str("{\"$f64\":\"");
            push_hex(out, &v.to_be_bytes());
            out.push_str("\"}");
        }
        Value::Id128(v) => {
            out.push_str("{\"id128\":\"");
            push_hex(out, v.as_bytes());
            out.push_str("\"}");
        }
        Value::String(text) => {
            out.push_str("{\"string\":");
            push_string(out, text);
            out.push('}');
        }
        Value::FixedBytes(bytes) => {
            out.push_str("{\"fixedBytes\":\"");
            push_hex(out, bytes);
            out.push_str("\"}");
        }
        Value::IntervalU64(interval) => {
            out.push_str("{\"intervalU64\":[\"");
            out.push_str(&interval.start().to_string());
            out.push_str("\",\"");
            out.push_str(&interval.end().to_string());
            out.push_str("\"]}");
        }
        Value::IntervalI64(interval) => {
            out.push_str("{\"intervalI64\":[\"");
            out.push_str(&interval.start().to_string());
            out.push_str("\",\"");
            out.push_str(&interval.end().to_string());
            out.push_str("\"]}");
        }
        Value::IntervalF64(interval) => {
            out.push_str("{\"intervalF64\":[\"");
            push_hex(out, &interval.start().to_be_bytes());
            out.push_str("\",\"");
            push_hex(out, &interval.end().to_be_bytes());
            out.push_str("\"]}");
        }
    }
}

fn parse_f64_bits(json: &Json) -> JsonResult<F64> {
    let raw = unhex_exact::<8>(json.as_str().ok_or("f64 hex")?)?;
    F64::from_canonical_be_bytes(raw).map_err(|_| "noncanonical f64")
}

pub(crate) fn parse_u64(json: &Json) -> JsonResult<u64> {
    let text = json.as_str().ok_or("decimal string")?;
    if text.starts_with('+') {
        return Err("u64");
    }
    text.parse().map_err(|_| "u64")
}

pub(crate) fn parse_i64(json: &Json) -> JsonResult<i64> {
    let text = json.as_str().ok_or("decimal string")?;
    if text.starts_with('+') {
        return Err("i64");
    }
    text.parse().map_err(|_| "i64")
}

pub(crate) fn pair2(json: &Json) -> JsonResult<&[Json]> {
    let pair = json.as_array().ok_or("pair")?;
    if pair.len() == 2 {
        Ok(pair)
    } else {
        Err("pair")
    }
}

// ---------------------------------------------------------------------------
// Hex and string emission helpers.
// ---------------------------------------------------------------------------

pub(crate) fn unhex(text: &str) -> JsonResult<Vec<u8>> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err("even hex length");
    }
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let hi = hex_nibble(pair[0])?;
            let lo = hex_nibble(pair[1])?;
            Ok((hi << 4) | lo)
        })
        .collect()
}

/// Exactly `N` bytes of lowercase hex; uppercase refuses (one spelling).
pub(crate) fn unhex_exact<const N: usize>(text: &str) -> JsonResult<[u8; N]> {
    if text.len() != N * 2 || text.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return Err("hex width");
    }
    let raw = unhex(text)?;
    let mut out = [0u8; N];
    out.copy_from_slice(&raw);
    Ok(out)
}

fn hex_nibble(byte: u8) -> JsonResult<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("hex byte"),
    }
}

pub(crate) fn push_hex(out: &mut String, bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        out.push(HEX[usize::from(byte >> 4)] as char);
        out.push(HEX[usize::from(byte & 0x0f)] as char);
    }
}

/// Emit a JSON string literal with the writer's one escaping policy.
pub(crate) fn push_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if (ch as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

pub(crate) fn push_indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

#[cfg(test)]
mod tests {
    use bumbledb::{F64, Id128, Interval, Value};

    use super::{parse_value, read_tree, render_value};

    fn roundtrip(value: &Value) {
        let mut out = String::new();
        render_value(&mut out, value);
        let tree = read_tree(&out).expect("rendered value parses");
        assert_eq!(&parse_value(&tree).expect("value arm"), value, "{out}");
    }

    #[test]
    fn every_value_arm_roundtrips_through_the_one_spelling() {
        let one = F64::from_canonical_bits(0x3ff0_0000_0000_0000).unwrap();
        let three = F64::from_canonical_bits(0x4008_0000_0000_0000).unwrap();
        for value in [
            Value::Bool(true),
            Value::U64(u64::MAX),
            Value::I64(i64::MIN),
            Value::F64(one),
            Value::Id128(Id128::from_bytes([0xab; 16])),
            Value::String("π \"quoted\"\n".into()),
            Value::FixedBytes(Box::from(*b"\x00\xff")),
            Value::IntervalU64(Interval::new(1, 5).unwrap()),
            Value::IntervalI64(Interval::new(-5, 5).unwrap()),
            Value::IntervalF64(Interval::new(one, three).unwrap()),
        ] {
            roundtrip(&value);
        }
    }

    #[test]
    fn noncanonical_and_multi_arm_values_refuse() {
        for raw in [
            r#"{"bool":true,"u64":"1"}"#,
            r#"{"$f64":"7ff8000000000001"}"#,
            r#"{"$f64":"3FF0000000000000"}"#,
            r#"{"id128":"abcd"}"#,
            r#"{"u64":"+7"}"#,
            r#"{"intervalF64":["3ff0000000000000","3ff0000000000000"]}"#,
            r#"{"fresh":true}"#,
        ] {
            let tree = read_tree(raw).expect("well-formed json");
            assert!(parse_value(&tree).is_err(), "{raw}");
        }
    }

    #[test]
    fn strict_reader_refuses_trailing_floats_and_bad_numbers() {
        assert!(read_tree("{} ").is_ok());
        assert!(read_tree("{}x").is_err());
        assert!(read_tree("1.5").is_err());
        assert!(read_tree("01").is_err());
        assert!(
            read_tree("-1").is_err(),
            "bare negative numbers do not exist"
        );
    }
}
