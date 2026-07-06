//! Tiny hand-rolled JSON emission helpers (docs/architecture/50-validation.md; the
//! dependency quarantine forbids serde). Shared by the trace writer and
//! the report's JSON renderer — one escaping rule, one number format.

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
}
