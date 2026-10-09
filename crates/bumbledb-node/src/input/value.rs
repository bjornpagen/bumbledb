//! JSON value encoding: 64-bit integers as canonical decimal strings, `f64`
//! as its 16-hex-digit IEEE bit pattern (so -0, NaN and the infinities are
//! exact), bytes as lowercase hex, UUIDs as canonical hyphenated text.
use bumbledb::schema::{IntervalElement, ValueType};
use bumbledb::{F64, FixedIntervalElement, Interval, Uuid, Value};
use napi_derive::napi;
use serde::Deserialize;

/// A `u64` spelled as canonical decimal (no sign, no leading zeros).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct U64Text(pub u64);

impl TryFrom<String> for U64Text {
    type Error = String;
    fn try_from(text: String) -> Result<Self, String> {
        let digits = text.as_bytes();
        let canonical = !digits.is_empty()
            && digits.iter().all(u8::is_ascii_digit)
            && (digits.len() == 1 || digits[0] != b'0');
        canonical
            .then(|| text.parse().ok())
            .flatten()
            .map(Self)
            .ok_or_else(|| format!("expected a canonical decimal u64, got {text:?}"))
    }
}

/// An `i64` spelled as canonical decimal (`-` only for negatives, no
/// leading zeros, no `-0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct I64Text(pub i64);

impl TryFrom<String> for I64Text {
    type Error = String;
    fn try_from(text: String) -> Result<Self, String> {
        let magnitude = text.strip_prefix('-').unwrap_or(&text);
        let digits = magnitude.as_bytes();
        let canonical = !digits.is_empty()
            && digits.iter().all(u8::is_ascii_digit)
            && (digits.len() == 1 || digits[0] != b'0')
            && text != "-0";
        canonical
            .then(|| text.parse().ok())
            .flatten()
            .map(Self)
            .ok_or_else(|| format!("expected a canonical decimal i64, got {text:?}"))
    }
}

/// An `f64` spelled as exactly 16 lowercase hex digits of its bit pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct F64Bits(pub F64);

impl TryFrom<String> for F64Bits {
    type Error = String;
    fn try_from(text: String) -> Result<Self, String> {
        (text.len() == 16
            && text
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')))
        .then(|| u64::from_str_radix(&text, 16).ok())
        .flatten()
        .map(|bits| Self(F64::from(f64::from_bits(bits))))
        .ok_or_else(|| format!("expected 16 lowercase hex digits of f64 bits, got {text:?}"))
    }
}

/// Bytes spelled as lowercase hex, two digits per byte.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct HexBytes(pub Box<[u8]>);

impl TryFrom<String> for HexBytes {
    type Error = String;
    fn try_from(text: String) -> Result<Self, String> {
        fn nibble(byte: u8) -> Option<u8> {
            match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                _ => None,
            }
        }
        let refuse = || format!("expected lowercase hex bytes, got {text:?}");
        if !text.len().is_multiple_of(2) {
            return Err(refuse());
        }
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&[high, low]| Some(nibble(high)? << 4 | nibble(low)?))
            .collect::<Option<Box<[u8]>>>()
            .map(Self)
            .ok_or_else(refuse)
    }
}

/// A UUID spelled as canonical lowercase hyphenated text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct UuidText(pub Uuid);

impl TryFrom<String> for UuidText {
    type Error = String;
    fn try_from(text: String) -> Result<Self, String> {
        let refuse = || format!("expected a canonical lowercase hyphenated UUID, got {text:?}");
        let id = Uuid::parse_str(&text).map_err(|_| refuse())?;
        let mut buffer = Uuid::encode_buffer();
        (*id.hyphenated().encode_lower(&mut buffer) == *text)
            .then_some(Self(id))
            .ok_or_else(refuse)
    }
}

/// One literal value as spelled. Intervals are half-open and nonempty.
#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ValueIn {
    Bool {
        value: bool,
    },
    U64 {
        #[napi(ts_type = "string")]
        value: U64Text,
    },
    I64 {
        #[napi(ts_type = "string")]
        value: I64Text,
    },
    F64 {
        #[napi(ts_type = "string")]
        value: F64Bits,
    },
    String {
        value: String,
    },
    Uuid {
        #[napi(ts_type = "string")]
        value: UuidText,
    },
    FixedBytes {
        #[napi(ts_type = "string")]
        value: HexBytes,
    },
    IntervalU64 {
        #[napi(ts_type = "string")]
        start: U64Text,
        #[napi(ts_type = "string")]
        end: U64Text,
    },
    IntervalI64 {
        #[napi(ts_type = "string")]
        start: I64Text,
        #[napi(ts_type = "string")]
        end: I64Text,
    },
    IntervalF64 {
        #[napi(ts_type = "string")]
        start: F64Bits,
        #[napi(ts_type = "string")]
        end: F64Bits,
    },
}

/// A decoded engine value; fields of this type are spelled [`ValueIn`].
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "ValueIn")]
pub struct Literal(pub Value);

impl TryFrom<ValueIn> for Literal {
    type Error = &'static str;
    fn try_from(value: ValueIn) -> Result<Self, &'static str> {
        const EMPTY: &str = "expected a nonempty interval (start < end)";
        Ok(Self(match value {
            ValueIn::Bool { value } => Value::Bool(value),
            ValueIn::U64 { value } => Value::U64(value.0),
            ValueIn::I64 { value } => Value::I64(value.0),
            ValueIn::F64 { value } => Value::F64(value.0),
            ValueIn::String { value } => Value::String(value.into()),
            ValueIn::Uuid { value } => Value::Uuid(value.0),
            ValueIn::FixedBytes { value } => Value::FixedBytes(value.0),
            ValueIn::IntervalU64 { start, end } => {
                Value::IntervalU64(Interval::new(start.0, end.0).ok_or(EMPTY)?)
            }
            ValueIn::IntervalI64 { start, end } => {
                Value::IntervalI64(Interval::new(start.0, end.0).ok_or(EMPTY)?)
            }
            ValueIn::IntervalF64 { start, end } => {
                Value::IntervalF64(Interval::new(start.0, end.0).ok_or(EMPTY)?)
            }
        }))
    }
}

#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum IntervalElementIn {
    U64,
    I64,
    F64,
}

#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum FixedIntervalElementIn {
    U64,
    I64,
}

/// One structural field type.
#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ValueTypeIn {
    Bool,
    U64,
    I64,
    F64,
    String,
    Uuid,
    FixedBytes {
        len: u16,
    },
    Interval {
        element: IntervalElementIn,
    },
    FixedInterval {
        element: FixedIntervalElementIn,
        #[napi(ts_type = "string")]
        width: U64Text,
    },
}

impl From<ValueTypeIn> for ValueType {
    fn from(value: ValueTypeIn) -> Self {
        match value {
            ValueTypeIn::Bool => Self::Bool,
            ValueTypeIn::U64 => Self::U64,
            ValueTypeIn::I64 => Self::I64,
            ValueTypeIn::F64 => Self::F64,
            ValueTypeIn::String => Self::String,
            ValueTypeIn::Uuid => Self::Uuid,
            ValueTypeIn::FixedBytes { len } => Self::FixedBytes { len },
            ValueTypeIn::Interval { element } => Self::Interval {
                element: match element {
                    IntervalElementIn::U64 => IntervalElement::U64,
                    IntervalElementIn::I64 => IntervalElement::I64,
                    IntervalElementIn::F64 => IntervalElement::F64,
                },
            },
            ValueTypeIn::FixedInterval { element, width } => Self::FixedInterval {
                element: match element {
                    FixedIntervalElementIn::U64 => FixedIntervalElement::U64,
                    FixedIntervalElementIn::I64 => FixedIntervalElement::I64,
                },
                width: width.0,
            },
        }
    }
}
