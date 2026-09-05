//! The successor float fixture corpus generator (P11; gates `F-GOLDEN`,
//! `F-CANON`, `F-ORDER`, `F-ARITH`, `F-AGG`, `F-CROSS`, `F-INTERVAL`).
//!
//! Deterministic bit-pattern corpora whose EXPECTED values come from the
//! independent bit/rational oracle (`crate::verify::f64_oracle`) — never
//! from the production engine, host floats, or a shared parser. The corpus
//! is consumed three ways: the Rust engine differential, the packed-SDK
//! cross-language fixtures (`F-CROSS` compares canonical BITS across
//! darwin-arm64 / linux-arm64 / linux-x64), and the Lean conformance lane's
//! future float cases.
//!
//! REGENERATION IS DEFERRED TO F3 (chapter 61: no generator execution in
//! F0–F2). The deferred command, to run once at the barrier and check the
//! emitted files in beside the existing conformance corpus:
//!
//! ```text
//! cargo run -p bumbledb-bench -- corpus-float --seed 0xB0B --out fixtures/float
//! ```
//!
//! (The CLI wiring for the `corpus-float` subcommand belongs to the shared
//! `cli/` roster owned by P14/P00 — requested in
//! `implementation/packets/P11.md` under hub/coordination notes; this
//! module exposes the pure generator so the wiring is one match arm.)

use crate::verify::f64_oracle::{
    INF, MAX_FINITE, NAN, NEG_INF, SIGN, canonical, mean_bits, order_key, ref_add, ref_div,
    ref_mul, ref_sub, sum_bits,
};

/// The structured boundary payloads every corpus draws from (chapter 70
/// G02's float roster): zeros, subnormal boundaries, normal boundaries,
/// exact/inexact integer-cast boundaries, infinities and NaN classes.
#[must_use]
pub fn boundary_payloads() -> Vec<u64> {
    vec![
        0,                     // +0
        SIGN,                  // -0 (normalizes)
        1,                     // smallest subnormal
        SIGN | 1,              // negative smallest subnormal
        0x000f_ffff_ffff_ffff, // largest subnormal
        0x0010_0000_0000_0000, // smallest normal
        0x3ff0_0000_0000_0000, // 1.0
        0x3ff0_0000_0000_0001, // nextUp(1.0)
        0x4000_0000_0000_0000, // 2.0
        0x4330_0000_0000_0000, // 2^52
        0x4340_0000_0000_0000, // 2^53 (integer-cast exactness boundary)
        0x4341_c379_37e0_8000, // 1e16
        0x43e0_0000_0000_0000, // 2^63 (i64 boundary)
        0x43f0_0000_0000_0000, // 2^64 (u64 boundary)
        MAX_FINITE,
        SIGN | MAX_FINITE,
        INF,
        NEG_INF,
        NAN,
        0x7ff0_0000_0000_0001, // signaling NaN (normalizes)
        0xfff8_0000_0000_0000, // negative quiet NaN (normalizes)
        0x7fff_ffff_ffff_ffff, // NaN payload (normalizes)
    ]
}

/// One canonicalization case: raw input bits, expected canonical bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonCase {
    pub raw: u64,
    pub expected: u64,
}

/// One ordered pair with the expected order-key relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderCase {
    pub lhs: u64,
    pub rhs: u64,
    pub lhs_key: u64,
    pub rhs_key: u64,
}

/// One arithmetic case with oracle-computed expected bits for the four
/// rounded operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArithCase {
    pub lhs: u64,
    pub rhs: u64,
    pub add: u64,
    pub sub: u64,
    pub mul: u64,
    pub div: u64,
}

/// One aggregate case: canonical input payloads (already deduplicated —
/// dedup precedes accumulation) with oracle sum and mean bits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggCase {
    pub inputs: Vec<u64>,
    pub sum: u64,
    pub mean: Option<u64>,
}

/// A deterministic xorshift-style walk over 64-bit patterns — independent
/// of the corpus RNG used for relational worlds, so the two corpora cannot
/// accidentally share structure.
fn walk(seed: u64, index: u64) -> u64 {
    let mut z = seed
        .wrapping_add(index.wrapping_mul(0x9e37_79b9_7f4a_7c15))
        .wrapping_add(1);
    z = (z ^ (z >> 33)).wrapping_mul(0xff51_afd7_ed55_8ccd);
    z = (z ^ (z >> 33)).wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    z ^ (z >> 33)
}

/// The canonicalization corpus: every boundary payload plus `random`
/// deterministic 64-bit patterns, each with its oracle-normal form.
#[must_use]
pub fn canon_corpus(seed: u64, random: u64) -> Vec<CanonCase> {
    let mut cases: Vec<CanonCase> = boundary_payloads()
        .into_iter()
        .map(|raw| CanonCase {
            raw,
            expected: canonical(raw),
        })
        .collect();
    for index in 0..random {
        let raw = walk(seed, index);
        cases.push(CanonCase {
            raw,
            expected: canonical(raw),
        });
    }
    cases
}

/// The total-order corpus over all boundary pairs.
#[must_use]
pub fn order_corpus() -> Vec<OrderCase> {
    let payloads: Vec<u64> = boundary_payloads().into_iter().map(canonical).collect();
    let mut cases = Vec::new();
    for &lhs in &payloads {
        for &rhs in &payloads {
            cases.push(OrderCase {
                lhs,
                rhs,
                lhs_key: order_key(lhs),
                rhs_key: order_key(rhs),
            });
        }
    }
    cases
}

/// The arithmetic corpus: boundary pairs plus deterministic random pairs,
/// expectations from the independent rational reference.
#[must_use]
pub fn arith_corpus(seed: u64, random: u64) -> Vec<ArithCase> {
    let payloads: Vec<u64> = boundary_payloads().into_iter().map(canonical).collect();
    let mut cases = Vec::new();
    for &lhs in &payloads {
        for &rhs in &payloads {
            cases.push(arith_case(lhs, rhs));
        }
    }
    for index in 0..random {
        let lhs = canonical(walk(seed, 2 * index));
        let rhs = canonical(walk(seed, 2 * index + 1));
        cases.push(arith_case(lhs, rhs));
    }
    cases
}

fn arith_case(lhs: u64, rhs: u64) -> ArithCase {
    ArithCase {
        lhs,
        rhs,
        add: ref_add(lhs, rhs),
        sub: ref_sub(lhs, rhs),
        mul: ref_mul(lhs, rhs),
        div: ref_div(lhs, rhs),
    }
}

/// The aggregate corpus: the named chapter 11 goldens plus deterministic
/// mixed-sign/mixed-magnitude groups.
///
/// # Panics
/// On a corpus-invariant violation (a golden group the exact reducer
/// refuses).
#[must_use]
pub fn agg_corpus(seed: u64, groups: u64, group_size: usize) -> Vec<AggCase> {
    let mut cases = vec![
        // {1e16, 1, -1e16} cancels exactly to 1.0 before rounding.
        agg_case(vec![
            0x4341_c379_37e0_8000,
            0x3ff0_0000_0000_0000,
            SIGN | 0x4341_c379_37e0_8000,
        ]),
        // {MAX, MAX}: sum overflows, mean is exactly MAX.
        agg_case(vec![MAX_FINITE, MAX_FINITE]),
        // {MIN_SUBNORMAL, MIN_SUBNORMAL}.
        agg_case(vec![1, 1]),
        // Mixed infinities poison; single-sign infinity dominates.
        agg_case(vec![INF, NEG_INF, 0x3ff0_0000_0000_0000]),
        agg_case(vec![INF, 0x3ff0_0000_0000_0000]),
        agg_case(vec![NAN, 0x3ff0_0000_0000_0000]),
        // Exact zero total.
        agg_case(vec![0x3ff0_0000_0000_0000, SIGN | 0x3ff0_0000_0000_0000]),
    ];
    for group in 0..groups {
        let mut inputs: Vec<u64> = (0..group_size)
            .map(|index| {
                canonical(walk(
                    seed ^ group,
                    u64::try_from(index).expect("small group"),
                ))
            })
            .collect();
        inputs.sort_unstable();
        inputs.dedup(); // dedup precedes accumulation, by contract
        cases.push(agg_case(inputs));
    }
    cases
}

fn agg_case(inputs: Vec<u64>) -> AggCase {
    let sum = sum_bits(&inputs);
    let mean = mean_bits(&inputs);
    AggCase { inputs, sum, mean }
}

/// Serialize a corpus deterministically as line-oriented hex — the F3
/// regeneration writes these files; the format is fixed here so the
/// emitted bytes are reviewable and diffable.
#[must_use]
pub fn render_agg(cases: &[AggCase]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for case in cases {
        for (index, bits) in case.inputs.iter().enumerate() {
            if index > 0 {
                out.push(' ');
            }
            let _ = write!(out, "{bits:016x}");
        }
        let _ = write!(out, " -> sum {:016x}", case.sum);
        match case.mean {
            Some(mean) => {
                let _ = write!(out, " mean {mean:016x}");
            }
            None => out.push_str(" mean none"),
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        agg_corpus, arith_corpus, boundary_payloads, canon_corpus, order_corpus, render_agg,
    };
    use crate::verify::f64_oracle::{INF, MAX_FINITE, NAN, canonical, is_canonical};

    #[test]
    fn corpora_are_deterministic_and_canonical() {
        assert_eq!(canon_corpus(7, 64), canon_corpus(7, 64));
        assert_eq!(arith_corpus(7, 16), arith_corpus(7, 16));
        assert_eq!(agg_corpus(7, 8, 5), agg_corpus(7, 8, 5));
        for case in canon_corpus(7, 64) {
            assert!(is_canonical(case.expected));
            assert_eq!(canonical(case.expected), case.expected, "idempotent");
        }
        for case in order_corpus() {
            if case.lhs == case.rhs {
                assert_eq!(case.lhs_key, case.rhs_key);
            } else {
                assert_ne!(case.lhs_key, case.rhs_key, "keys are injective");
            }
        }
    }

    #[test]
    fn the_named_goldens_are_present_with_oracle_expectations() {
        let cases = agg_corpus(7, 0, 0);
        let cancel = &cases[0];
        assert_eq!(cancel.sum, 0x3ff0_0000_0000_0000, "exact cancellation");
        let max_pair = &cases[1];
        assert_eq!(max_pair.sum, INF, "sum overflows once-rounded");
        assert_eq!(max_pair.mean, Some(MAX_FINITE), "mean is exact");
        let poisoned = &cases[3];
        assert_eq!(poisoned.sum, NAN, "mixed infinities are NaN");
        let rendered = render_agg(&cases);
        assert!(rendered.contains("7ff0000000000000"), "the inf golden");
        assert!(
            boundary_payloads().len() >= 20,
            "the boundary roster covers every named class"
        );
    }
}
