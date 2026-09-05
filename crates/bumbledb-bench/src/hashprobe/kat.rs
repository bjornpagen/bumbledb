//! Known-answer-test loading with typed refusal (HASH-01).
//!
//! The probe never invents expected digests. Official vectors are copied into
//! a small JSON file before the F3 run:
//!
//! - BLAKE3: the pinned upstream `test_vectors/test_vectors.json` from the
//!   BLAKE3 repository at the revision matching the locked `blake3` crate.
//!   Its inputs are length-`N` prefixes of the repeating byte cycle
//!   `0, 1, ..., 250` (that generation rule is reproduced here; the expected
//!   hex is not).
//! - AEGIS-128L: the test vectors of the pinned `aegis` crate revision /
//!   draft-irtf-cfrg-aegis-aead, recorded in the same format with the
//!   zero-key MAC convention documented per entry.
//!
//! A missing or unreadable vector file is **`NotRun` or `Err`**, never a
//! pass. The gate ledger treats `NotRun` as an unsatisfied cell.

use std::fmt::Write as _;
use std::path::Path;

use crate::json::{self, Value};

use super::probe::{Candidate, digest_oneshot};

/// The BLAKE3 official vector input rule: `input[i] = i % 251`.
///
/// # Panics
/// Never in practice: `i % 251` always fits a byte.
#[must_use]
pub fn blake3_vector_input(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| u8::try_from(i % 251).expect("cycle fits a byte"))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KatOutcome {
    /// Every vector in the file matched; carries the vector count.
    Passed(usize),
    /// The file was read but at least one digest mismatched.
    Failed(String),
    /// No vector file was supplied. Recorded, never upgraded to a pass.
    NotRun(String),
}

impl KatOutcome {
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Passed(count) => format!("passed ({count} vectors)"),
            Self::Failed(detail) => format!("FAILED — {detail}"),
            Self::NotRun(reason) => format!("NotRun — {reason}"),
        }
    }

    pub fn push_json(&self, out: &mut String) {
        match self {
            Self::Passed(count) => {
                let _ = write!(out, "{{\"status\":\"Passed\",\"vectors\":{count}}}");
            }
            Self::Failed(detail) => {
                out.push_str("{\"status\":\"Failed\",\"detail\":");
                json::push_str_lit(out, detail);
                out.push('}');
            }
            Self::NotRun(reason) => {
                out.push_str("{\"status\":\"NotRun\",\"reason\":");
                json::push_str_lit(out, reason);
                out.push('}');
            }
        }
    }
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err(format!("odd hex length {}", hex.len()));
    }
    (0..hex.len() / 2)
        .map(|i| {
            u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).map_err(|e| format!("hex byte {i}: {e}"))
        })
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Vector file format (authored by hand from the pinned upstream sources):
/// `{"blake3":[{"input_len":N,"hash":"<hex, >= 32 bytes>"}, ...]}`. The input
/// is regenerated from the official byte-cycle rule; only expected output hex
/// is copied.
///
/// # Errors
/// Unreadable/malformed files are errors — the caller reports them instead of
/// skipping the check.
pub fn verify_blake3_file(path: &Path) -> Result<KatOutcome, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("KAT vector file {}: {e}", path.display()))?;
    let parsed = json::parse(&text).map_err(|e| format!("KAT parse {}: {e}", path.display()))?;
    let vectors = parsed
        .get("blake3")
        .and_then(Value::as_arr)
        .ok_or_else(|| format!("{}: no `blake3` vector array", path.display()))?;
    if vectors.is_empty() {
        return Err(format!("{}: zero vectors is not a check", path.display()));
    }
    let mut checked = 0usize;
    for (index, vector) in vectors.iter().enumerate() {
        let len = vector
            .get("input_len")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("vector {index}: missing input_len"))?;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "vector lengths are small nonnegative integers by format contract"
        )]
        let len = len as usize;
        let expected_hex = vector
            .get("hash")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("vector {index}: missing hash"))?;
        let expected = hex_decode(expected_hex).map_err(|e| format!("vector {index}: {e}"))?;
        if expected.len() < 32 {
            return Err(format!(
                "vector {index}: expected hash shorter than 32 bytes ({})",
                expected.len()
            ));
        }
        let input = blake3_vector_input(len);
        let got = digest_oneshot(Candidate::Blake3Full32, &input);
        if got != expected[..32] {
            return Ok(KatOutcome::Failed(format!(
                "vector {index} (input_len {len}): got {}, expected {}",
                hex_encode(&got),
                hex_encode(&expected[..32])
            )));
        }
        checked += 1;
    }
    Ok(KatOutcome::Passed(checked))
}
