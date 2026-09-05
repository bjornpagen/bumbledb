//! The canonical bounded named-scalar record — the core codec for the
//! log's declared `CommandResult` slot (C01; chapter 30: "`CommandResult`
//! is bounded caller-declared scalar metadata … its grammar and digest
//! contribution are fixed by the command codec"; PROMPT: the core owns the
//! result primitive and the log imports it literally).
//!
//! One record is a set of `name → scalar` entries. The canonical spelling
//! sorts entries by ascending UTF-8 name bytes, refuses duplicate names,
//! and reuses the canonical row codec's scalar tags and payload encodings
//! (tags 0/1/2/3/4/5/8 — booleans, integers, canonical F64 payload bits,
//! strings, bytes, Id128) so no second value vocabulary exists. Interval
//! values are not scalars and refuse. The EMPTY record is the empty byte
//! string — exactly the log's existing `CommandResult::empty()`; a
//! nonempty record opens with its own domain-separating family magic and
//! layout counter, so result bytes cannot alias any other frame family.
//!
//! Determinism: the bytes are a pure function of the entry set and encode
//! identically regardless of caller entry order. The command digest covers
//! them verbatim; strict decode refuses noncanonical spellings (unsorted
//! or duplicate names, alternative float payloads, trailing bytes) rather
//! than normalizing wire input. Physical bytes are provisional until the
//! F3 format freeze (C12); a change bumps [`LAYOUT`].

use crate::work::WorkError;
use crate::{F64, Id128, Value, WorkContext};

/// The result frame's family magic; no other frame family shares it.
pub const FAMILY: &[u8] = b"bumbledb.result.v1\0";

/// The frame layout counter; strict decode refuses any other value.
pub const LAYOUT: u16 = 1;

/// The one frame kind under this family.
const RECORD: u8 = 1;

/// Bounded entry names: nonempty UTF-8, at most this many bytes.
pub const MAX_NAME_BYTES: usize = 128;

/// `family ‖ layout(u16) ‖ kind(u8) ‖ entry count(u32)`.
const HEADER_LEN: usize = 19 + 2 + 1 + 4;

const TAG_BOOL: u8 = 0;
const TAG_U64: u8 = 1;
const TAG_I64: u8 = 2;
const TAG_F64: u8 = 3;
const TAG_STRING: u8 = 4;
const TAG_BYTES: u8 = 5;
const TAG_ID128: u8 = 8;

/// Every refusal of the result-record codec, encode and strict decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultError {
    Work(WorkError),
    /// The value at entry `entry` (caller order on encode, wire order on
    /// decode) is not a scalar (intervals are positions, not results).
    NonScalar {
        entry: usize,
    },
    /// An empty or overlong name — see [`MAX_NAME_BYTES`]. (Encode checks
    /// the caller's entry; decode checks the wire span.)
    InvalidName {
        entry: usize,
    },
    /// Two entries share one name; a record is a set.
    DuplicateName {
        entry: usize,
    },
    /// The encoded record exceeds the caller's byte budget.
    Budget {
        needed: usize,
        budget: usize,
    },
    // Strict decode refusals:
    LimitExceeded,
    Family,
    Layout {
        got: u16,
    },
    Kind {
        got: u8,
    },
    Truncated {
        at: usize,
    },
    TrailingBytes {
        at: usize,
    },
    Tag {
        at: usize,
        got: u8,
    },
    /// Entry names must be strictly increasing by UTF-8 bytes.
    Unordered {
        entry: usize,
    },
    /// A count field claims more entries than the bytes could hold.
    InvalidCount,
    InvalidBool {
        entry: usize,
    },
    /// A float payload is negative zero or a noncanonical NaN; wire input
    /// is refused, never normalized.
    NonCanonicalFloat {
        entry: usize,
    },
    InvalidUtf8 {
        entry: usize,
    },
    LengthOverflow,
    Allocation,
}

impl From<WorkError> for ResultError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

impl std::fmt::Display for ResultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical result record: {self:?}")
    }
}
impl std::error::Error for ResultError {}

fn scalar_payload_len(entry: usize, value: &Value) -> Result<usize, ResultError> {
    match value {
        Value::Bool(_) => Ok(1),
        Value::U64(_) | Value::I64(_) | Value::F64(_) => Ok(8),
        Value::Id128(_) => Ok(16),
        Value::String(text) => text.len().checked_add(8).ok_or(ResultError::LengthOverflow),
        Value::FixedBytes(bytes) => bytes
            .len()
            .checked_add(8)
            .ok_or(ResultError::LengthOverflow),
        Value::IntervalU64(_) | Value::IntervalI64(_) | Value::IntervalF64(_) => {
            Err(ResultError::NonScalar { entry })
        }
    }
}

/// Encodes one declared result record canonically. Entry order does not
/// matter: the canonical spelling sorts by name bytes. An empty entry set
/// encodes as the empty byte string.
///
/// # Errors
/// Refuses non-scalar values, invalid/duplicate names, a record over the
/// byte budget, and stopped work.
/// # Panics
/// Only on programmer-invariant violations (entry widths already checked
/// above); never on caller input.
pub fn encode_result(
    entries: &[(&str, &Value)],
    max_bytes: usize,
    work: &WorkContext,
) -> Result<Vec<u8>, ResultError> {
    if entries.is_empty() {
        return Ok(Vec::new());
    }

    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by_key(|&index| entries[index].0.as_bytes());

    let mut total = HEADER_LEN;
    for (rank, &index) in order.iter().enumerate() {
        work.step(1)?;
        let (name, value) = entries[index];
        if name.is_empty() || name.len() > MAX_NAME_BYTES {
            return Err(ResultError::InvalidName { entry: index });
        }
        if rank > 0 {
            let previous = entries[order[rank - 1]].0;
            if previous.as_bytes() == name.as_bytes() {
                return Err(ResultError::DuplicateName { entry: index });
            }
        }
        let payload = scalar_payload_len(index, value)?;
        total = total
            .checked_add(2)
            .and_then(|n| n.checked_add(name.len()))
            .and_then(|n| n.checked_add(1))
            .and_then(|n| n.checked_add(payload))
            .ok_or(ResultError::LengthOverflow)?;
    }
    if total > max_bytes {
        return Err(ResultError::Budget {
            needed: total,
            budget: max_bytes,
        });
    }
    if u32::try_from(entries.len()).is_err() {
        return Err(ResultError::LengthOverflow);
    }

    work.step(total as u64)?;
    let mut out = Vec::new();
    out.try_reserve_exact(total)
        .map_err(|_| ResultError::Allocation)?;
    out.extend_from_slice(FAMILY);
    out.extend_from_slice(&LAYOUT.to_be_bytes());
    out.push(RECORD);
    out.extend_from_slice(
        &u32::try_from(entries.len())
            .expect("checked above")
            .to_be_bytes(),
    );
    for &index in &order {
        let (name, value) = entries[index];
        out.extend_from_slice(
            &u16::try_from(name.len())
                .expect("bounded by MAX_NAME_BYTES")
                .to_be_bytes(),
        );
        out.extend_from_slice(name.as_bytes());
        match value {
            Value::Bool(v) => {
                out.push(TAG_BOOL);
                out.push(u8::from(*v));
            }
            Value::U64(v) => {
                out.push(TAG_U64);
                out.extend_from_slice(&v.to_be_bytes());
            }
            Value::I64(v) => {
                out.push(TAG_I64);
                out.extend_from_slice(&v.to_be_bytes());
            }
            Value::F64(v) => {
                out.push(TAG_F64);
                out.extend_from_slice(&v.to_be_bytes());
            }
            Value::String(v) => {
                out.push(TAG_STRING);
                out.extend_from_slice(&(v.len() as u64).to_be_bytes());
                out.extend_from_slice(v.as_bytes());
            }
            Value::FixedBytes(v) => {
                out.push(TAG_BYTES);
                out.extend_from_slice(&(v.len() as u64).to_be_bytes());
                out.extend_from_slice(v);
            }
            Value::Id128(v) => {
                out.push(TAG_ID128);
                out.extend_from_slice(v.as_bytes());
            }
            Value::IntervalU64(_) | Value::IntervalI64(_) | Value::IntervalF64(_) => {
                unreachable!("refused while sizing")
            }
        }
    }
    debug_assert_eq!(out.len(), total, "size arithmetic matches the writer");
    Ok(out)
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], ResultError> {
        let end = self
            .at
            .checked_add(len)
            .ok_or(ResultError::LengthOverflow)?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or(ResultError::Truncated { at: self.at })?;
        self.at = end;
        Ok(bytes)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], ResultError> {
        let mut array = [0; N];
        array.copy_from_slice(self.take(N)?);
        Ok(array)
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.at
    }
}

/// Strictly decodes one canonical result record to owned `(name, scalar)`
/// entries in canonical (ascending-name) order. The empty byte string is
/// the empty record.
///
/// # Errors
/// Every grammar refusal in [`ResultError`]; wire input is never
/// normalized — unsorted names, duplicate names, noncanonical float
/// payloads and trailing bytes all refuse.
pub fn decode_result(
    bytes: &[u8],
    max_bytes: usize,
    work: &WorkContext,
) -> Result<Vec<(Box<str>, Value)>, ResultError> {
    if bytes.len() > max_bytes {
        return Err(ResultError::LimitExceeded);
    }
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    work.input(bytes.len() as u64)?;
    let mut input = Reader { bytes, at: 0 };
    if input.take(FAMILY.len())? != FAMILY {
        return Err(ResultError::Family);
    }
    let layout = u16::from_be_bytes(input.array()?);
    if layout != LAYOUT {
        return Err(ResultError::Layout { got: layout });
    }
    let kind = input.array::<1>()?[0];
    if kind != RECORD {
        return Err(ResultError::Kind { got: kind });
    }
    let count = u32::from_be_bytes(input.array()?) as usize;
    // Minimum entry: name length (2) + one name byte + tag + bool payload.
    if count == 0 || count > input.remaining() / 5 {
        return Err(ResultError::InvalidCount);
    }
    let mut out: Vec<(Box<str>, Value)> = Vec::new();
    out.try_reserve_exact(count)
        .map_err(|_| ResultError::Allocation)?;
    for entry in 0..count {
        work.step(1)?;
        let name_len = usize::from(u16::from_be_bytes(input.array()?));
        if name_len == 0 || name_len > MAX_NAME_BYTES {
            return Err(ResultError::InvalidName { entry });
        }
        let name_bytes = input.take(name_len)?;
        let name =
            std::str::from_utf8(name_bytes).map_err(|_| ResultError::InvalidUtf8 { entry })?;
        if let Some((previous, _)) = out.last() {
            match previous.as_bytes().cmp(name.as_bytes()) {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => {
                    return Err(ResultError::DuplicateName { entry });
                }
                std::cmp::Ordering::Greater => {
                    return Err(ResultError::Unordered { entry });
                }
            }
        }
        let at = input.at;
        let tag = input.array::<1>()?[0];
        let value = match tag {
            TAG_BOOL => match input.array::<1>()?[0] {
                0 => Value::Bool(false),
                1 => Value::Bool(true),
                _ => return Err(ResultError::InvalidBool { entry }),
            },
            TAG_U64 => Value::U64(u64::from_be_bytes(input.array()?)),
            TAG_I64 => Value::I64(i64::from_be_bytes(input.array()?)),
            TAG_F64 => Value::F64(
                F64::from_canonical_be_bytes(input.array()?)
                    .map_err(|_| ResultError::NonCanonicalFloat { entry })?,
            ),
            TAG_STRING => {
                let len = usize::try_from(u64::from_be_bytes(input.array()?))
                    .map_err(|_| ResultError::LengthOverflow)?;
                let span = input.take(len)?;
                work.step(len as u64)?;
                let text =
                    std::str::from_utf8(span).map_err(|_| ResultError::InvalidUtf8 { entry })?;
                let mut owned = String::new();
                owned
                    .try_reserve_exact(text.len())
                    .map_err(|_| ResultError::Allocation)?;
                owned.push_str(text);
                Value::String(owned.into_boxed_str())
            }
            TAG_BYTES => {
                let len = usize::try_from(u64::from_be_bytes(input.array()?))
                    .map_err(|_| ResultError::LengthOverflow)?;
                let span = input.take(len)?;
                work.step(len as u64)?;
                let mut owned = Vec::new();
                owned
                    .try_reserve_exact(span.len())
                    .map_err(|_| ResultError::Allocation)?;
                owned.extend_from_slice(span);
                Value::FixedBytes(owned.into_boxed_slice())
            }
            TAG_ID128 => Value::Id128(Id128::from_bytes(input.array()?)),
            got => return Err(ResultError::Tag { at, got }),
        };
        let mut owned_name = String::new();
        owned_name
            .try_reserve_exact(name.len())
            .map_err(|_| ResultError::Allocation)?;
        owned_name.push_str(name);
        out.push((owned_name.into_boxed_str(), value));
    }
    if input.at != bytes.len() {
        return Err(ResultError::TrailingBytes { at: input.at });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{FAMILY, LAYOUT, MAX_NAME_BYTES, ResultError, decode_result, encode_result};
    use crate::work::ExecutionPolicy;
    use crate::{F64, Id128, Interval, Value, WorkContext};
    use std::time::Duration;

    fn work() -> WorkContext {
        ExecutionPolicy {
            input_bytes: 1_000_000,
            working_bytes: 1_000_000,
            scratch_bytes: 0,
            result_bytes: 0,
            rows: 100_000,
            work_units: 1_000_000,
            timeout: Duration::from_secs(60),
        }
        .start()
        .unwrap()
    }

    const BUDGET: usize = 4096;

    fn full_record() -> Vec<(&'static str, Value)> {
        vec![
            ("ok", Value::Bool(true)),
            ("count", Value::U64(42)),
            ("delta", Value::I64(-7)),
            ("mean", Value::F64(F64::from(2.5))),
            ("label", Value::String("hello".into())),
            (
                "blob",
                Value::FixedBytes(Box::from([0xde, 0xad, 0xbe, 0xef])),
            ),
            ("entity", Value::Id128(Id128::from_bytes([7; 16]))),
        ]
    }

    fn borrow<'a>(record: &'a [(&'static str, Value)]) -> Vec<(&'static str, &'a Value)> {
        record.iter().map(|(name, value)| (*name, value)).collect()
    }

    /// Every scalar round-trips; the decoded order is canonical
    /// (ascending name bytes) regardless of the caller's entry order.
    #[test]
    fn all_scalars_round_trip_in_canonical_order() {
        let record = full_record();
        let bytes = encode_result(&borrow(&record), BUDGET, &work()).expect("encodes");
        let decoded = decode_result(&bytes, BUDGET, &work()).expect("decodes");
        let names: Vec<&str> = decoded.iter().map(|(name, _)| name.as_ref()).collect();
        assert_eq!(
            names,
            vec!["blob", "count", "delta", "entity", "label", "mean", "ok"]
        );
        for (name, value) in &record {
            let found = decoded
                .iter()
                .find(|(decoded_name, _)| decoded_name.as_ref() == *name)
                .expect("present");
            assert_eq!(&found.1, value, "{name}");
        }
    }

    /// Canonical bytes are entry-order independent and deterministic —
    /// the command digest covers them verbatim.
    #[test]
    fn bytes_are_entry_order_independent_and_header_pinned() {
        let record = full_record();
        let mut reversed = record.clone();
        reversed.reverse();
        let a = encode_result(&borrow(&record), BUDGET, &work()).expect("encodes");
        let b = encode_result(&borrow(&reversed), BUDGET, &work()).expect("encodes");
        assert_eq!(a, b);
        assert_eq!(&a[..19], FAMILY);
        assert_eq!(a[19..21], LAYOUT.to_be_bytes());
        assert_eq!(a[21], 1);
        assert_eq!(a[22..26], 7u32.to_be_bytes());
        assert_eq!(FAMILY, b"bumbledb.result.v1\0");
    }

    /// The empty record is the empty byte string, both directions —
    /// exactly the log's `CommandResult::empty()`.
    #[test]
    fn empty_record_is_the_empty_byte_string() {
        assert_eq!(
            encode_result(&[], BUDGET, &work()).expect("encodes"),
            Vec::<u8>::new()
        );
        assert_eq!(
            decode_result(&[], BUDGET, &work()).expect("decodes"),
            Vec::new()
        );
    }

    /// Non-scalar values, invalid and duplicate names, and budget overflow
    /// refuse with typed errors.
    #[test]
    fn encode_refusals_are_typed() {
        let interval = Value::IntervalU64(Interval::new(1, 5).expect("fixture"));
        assert_eq!(
            encode_result(&[("span", &interval)], BUDGET, &work()),
            Err(ResultError::NonScalar { entry: 0 })
        );
        let value = Value::U64(1);
        assert_eq!(
            encode_result(&[("", &value)], BUDGET, &work()),
            Err(ResultError::InvalidName { entry: 0 })
        );
        let long = "n".repeat(MAX_NAME_BYTES + 1);
        assert_eq!(
            encode_result(&[(long.as_str(), &value)], BUDGET, &work()),
            Err(ResultError::InvalidName { entry: 0 })
        );
        let other = Value::U64(2);
        assert!(matches!(
            encode_result(&[("twin", &value), ("twin", &other)], BUDGET, &work()),
            Err(ResultError::DuplicateName { .. })
        ));
        assert!(matches!(
            encode_result(&[("name", &value)], 8, &work()),
            Err(ResultError::Budget { budget: 8, .. })
        ));
    }

    /// Strict decode refuses foreign frames, malformed tags, noncanonical
    /// floats, unsorted/duplicate names and trailing bytes — never
    /// normalizing wire input.
    #[test]
    fn strict_decode_refuses_malformed_frames() {
        let record = full_record();
        let bytes = encode_result(&borrow(&record), BUDGET, &work()).expect("encodes");

        assert_eq!(
            decode_result(&bytes, bytes.len() - 1, &work()),
            Err(ResultError::LimitExceeded)
        );

        let mut forged = bytes.clone();
        forged[0] ^= 1;
        assert_eq!(
            decode_result(&forged, BUDGET, &work()),
            Err(ResultError::Family)
        );

        let mut forged = bytes.clone();
        forged[20] = 9;
        assert_eq!(
            decode_result(&forged, BUDGET, &work()),
            Err(ResultError::Layout { got: 9 })
        );

        let mut forged = bytes.clone();
        forged[21] = 3;
        assert_eq!(
            decode_result(&forged, BUDGET, &work()),
            Err(ResultError::Kind { got: 3 })
        );

        // Truncation refuses at every prefix.
        for cut in [5, 24, bytes.len() - 1] {
            assert!(
                matches!(
                    decode_result(&bytes[..cut], BUDGET, &work()),
                    Err(ResultError::Truncated { .. } | ResultError::InvalidCount)
                ),
                "prefix {cut}"
            );
        }

        let mut forged = bytes.clone();
        forged.push(0);
        assert!(matches!(
            decode_result(&forged, BUDGET, &work()),
            Err(ResultError::TrailingBytes { .. })
        ));

        // Oversized count claims refuse before allocation.
        let mut forged = bytes.clone();
        forged[22..26].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            decode_result(&forged, BUDGET, &work()),
            Err(ResultError::InvalidCount)
        );

        // A negative-zero float payload refuses (canonical bits only).
        let value = Value::F64(F64::ZERO);
        let zero = encode_result(&[("z", &value)], BUDGET, &work()).expect("encodes");
        let mut forged = zero.clone();
        let payload_at = zero.len() - 8;
        forged[payload_at] = 0x80; // -0.0 payload
        assert_eq!(
            decode_result(&forged, BUDGET, &work()),
            Err(ResultError::NonCanonicalFloat { entry: 0 })
        );

        // Unsorted names refuse: swap two single-byte-name entries.
        let a = Value::U64(1);
        let b = Value::U64(2);
        let sorted = encode_result(&[("a", &a), ("b", &b)], BUDGET, &work()).expect("encodes");
        let mut unsorted = sorted.clone();
        // Entries: [len=1]['a'][tag][8B] then [len=1]['b'][tag][8B].
        let first_name = 26 + 2;
        let second_name = first_name + 1 + 9 + 2;
        unsorted[first_name] = b'b';
        unsorted[second_name] = b'a';
        assert_eq!(
            decode_result(&unsorted, BUDGET, &work()),
            Err(ResultError::Unordered { entry: 1 })
        );
        let mut duplicate = sorted;
        duplicate[second_name] = b'a';
        assert_eq!(
            decode_result(&duplicate, BUDGET, &work()),
            Err(ResultError::DuplicateName { entry: 1 })
        );
    }
}
