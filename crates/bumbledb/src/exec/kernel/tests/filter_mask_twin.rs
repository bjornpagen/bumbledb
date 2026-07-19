//! The filter-mask decider twin (PRD-I3; the delete-fork analysis,
//! `docs/prds/incremental-images/prd-I3-decider-twin.md`): prices the
//! validity-mask tax at its CHEAPEST surface — `filter_eq_u64`, the
//! representative of the six survivor-producing filter kernels, the
//! shape where the one-op-per-chunk claim lives — so the gated delete
//! fork reopens (if it ever does) with the number already in hand.
//!
//! **The honest framing.** This twin BANKS the mask-fork answer; the
//! mask PRD stays unwritten. Nothing here ships: the masked body is
//! test-local by placement law (`scripts/check-asm.sh` audits the
//! release disassembly of shipped kernels, and a gated fork ships
//! nothing — no new public items enter `exec/kernel/`). The bench-side
//! delete lane (`cold_containment_walk_delete`) and this twin exist
//! together so the fork's trigger is a MEASUREMENT, not an argument.
//!
//! **Arms** (interleaved in one process, the pins.rs/stride_ab idiom):
//! - **A (shipped):** `filter_eq_u64(col, v, out)` as-is.
//! - **A′ (local twin):** a line-for-line transcription of A with one
//!   forced substitution — a SAFE cursor-write where the shipped body's
//!   `write_survivor_bits` uses a documented `get_unchecked_mut` (the
//!   engine law: no new `unsafe` outside the sanctioned modules, and a
//!   test-local twin is not one). Every masked arm shares this write
//!   path, so **B/A′ isolates the mask AND at identical codegen**, and
//!   A′/A prices the safe-write substitution itself. If A′/A reads
//!   materially above the noise band, the Measure phase escalates
//!   before any verdict is drawn from B/A.
//! - **B (masked, all-live):** `filter_eq_u64_masked` with the
//!   validity bitmap all-ones — the pure tax at density 1.0 (the
//!   u64-word bitmap extraction per 4-lane chunk + the AND + the second
//!   load stream; zero semantic effect — B's output is asserted
//!   bit-identical to A's).
//! - **C (masked, holed):** the same kernel at 1/64 and 1/8 dead,
//!   uniform-random — the realistic tombstone regime; checks the
//!   survivor-write path under holes (asserted against the masked
//!   scalar reference).
//!
//! Two tiers — L2-resident (~2 MB column) and DRAM (~100 MB) — at
//! selectivities 1% and 50% (survivor-write-bound vs scan-bound). Five
//! fresh-data blocks × three interleaved passes each, arm order
//! rotating per pass (drift cancellation, TAGE discipline); the
//! reported figure per arm is the min of the five block medians.
//!
//! **Invocation (the Measure phase owns execution):**
//! `scripts/measure.sh cargo test --release -p bumbledb filter_mask_twin -- --ignored --nocapture`
//! — the measurement mutex and clock-proxy discipline inherited from
//! the pins.rs pattern. The test prints ratios and asserts only arm
//! agreement; the verdict is read from the measured run, NEVER asserted
//! as a timing assertion. No number from this file is claimed anywhere
//! until that run lands — pending-measurement until then.
//!
//! **The decision rule (recorded verbatim with the result):**
//! - DISSOLVES the fork's kernel-tax half: B/A ≤ ~1.03 on both tiers
//!   (inside the harness's demonstrated noise band — cf. the stride_ab
//!   filter-surface 1.00×). The filter surface's share of the re-earn
//!   bill becomes a formality; the fork's real cost concentrates in the
//!   already-priced dense→gathered conversion (8.8 vs 4.0–4.6 rows/ns,
//!   `exec/kernel.rs`), and the argument moves to workload arithmetic.
//! - CONFIRMS the fork's death: B/A ≥ ~1.10 at either tier. The mask is
//!   expensive at its cheapest surface; compact-on-delete wins outright
//!   and the mask design dies without anyone touching folds, Allen, or
//!   NEON.
//! - In between: escalate to the fold-surface twin
//!   (`fold_sum_u64_dense` vs `fold_sum_u64_idx` over a live-position
//!   list at densities 1.0/0.99/0.9/0.5) before any design decision.
//!
//! **Honesty clause:** a favorable filter result does NOT clear the
//! fold/Identity surface — that degradation is real and already
//! measured at ~2×. This is the cheap FIRST decider because only one of
//! its outcomes is survivable for the mask route.

use std::simd::prelude::*;
use std::time::Instant;

use super::filter_eq_u64;

/// The u64 kernel width, transcribed from the shipped kernel.
const LANES: usize = 4;

/// One 4-lane chunk's validity bits (a 4-bit group never straddles a
/// bitmap word: chunk bases are multiples of 4 and 64 ≡ 0 mod 4, so the
/// extraction is one shift + one mask per chunk).
const LANE_MASK: u64 = (1 << LANES) - 1;

/// The needle. Top bit clear, so every non-matching cell (which carries
/// the top bit by construction) is a guaranteed mismatch.
const NEEDLE: u64 = 7;

/// splitmix-style avalanche: fresh data per (seed, index) — the TAGE
/// discipline (`m2max.predict.tage-memorizes-benchmarks`).
fn mix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The safe cursor-write twin of the shipped `write_survivor_bits`:
/// same branchless cursor advance, but an indexed store in place of the
/// documented `get_unchecked_mut` (no new unsafe outside the sanctioned
/// modules). The A′ arm exists exactly to price this substitution.
fn write_survivors(out: &mut [u32], mut write: usize, mut pos: u32, bits: u64) -> (usize, u32) {
    for lane in 0..LANES {
        out[write] = pos;
        write += usize::from((bits >> lane) & 1 != 0);
        pos = pos.wrapping_add(1);
    }
    (write, pos)
}

/// A′ — the local line-for-line twin of [`filter_eq_u64`] (the shipped
/// chunk loop, the shipped scalar tail), differing only in the safe
/// cursor-write. Bit-identity with the shipped kernel is pinned in the
/// always-on property test below.
#[inline(never)]
fn filter_eq_u64_local(col: &[u64], value: u64, out: &mut Vec<u32>) {
    let needle = Simd::<u64, LANES>::splat(value);
    let start = out.len();
    out.resize(start + col.len(), 0);
    let mut write = start;
    let mut pos = 0u32;
    let (chunks, tail) = col.as_chunks::<LANES>();
    let tail_start = col.len() - tail.len();
    for chunk in chunks {
        let bits = Simd::from_array(*chunk).simd_eq(needle).to_bitmask();
        (write, pos) = write_survivors(out, write, pos, bits);
    }
    for (i, &item) in tail.iter().enumerate() {
        out[write] = u32::try_from(tail_start + i).expect("positions fit u32");
        write += usize::from(item == value);
    }
    out.truncate(write);
}

/// B/C — the masked candidate, exactly the fork's proposed shape: the
/// shipped body plus `bits &= validity_bits(chunk)` fused after
/// `to_bitmask()` (one shift, one AND, one extra load stream), and a
/// per-lane validity test in the scalar tail. `validity` is a u64-word
/// bitmap, bit `i` set ⇔ position `i` live. Test-local: this body never
/// enters the shipped kernel surface while the fork is gated.
#[inline(never)]
fn filter_eq_u64_masked(col: &[u64], value: u64, validity: &[u64], out: &mut Vec<u32>) {
    assert!(
        validity.len() * 64 >= col.len(),
        "one validity bit per position"
    );
    let needle = Simd::<u64, LANES>::splat(value);
    let start = out.len();
    out.resize(start + col.len(), 0);
    let mut write = start;
    let mut pos = 0u32;
    let (chunks, tail) = col.as_chunks::<LANES>();
    let tail_start = col.len() - tail.len();
    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        let base = chunk_idx * LANES;
        let mut bits = Simd::from_array(*chunk).simd_eq(needle).to_bitmask();
        // The fork's whole question, one fused line.
        bits &= (validity[base >> 6] >> (base & 63)) & LANE_MASK;
        (write, pos) = write_survivors(out, write, pos, bits);
    }
    for (i, &item) in tail.iter().enumerate() {
        let position = tail_start + i;
        out[write] = u32::try_from(position).expect("positions fit u32");
        let live = (validity[position >> 6] >> (position & 63)) & 1 != 0;
        write += usize::from(item == value && live);
    }
    out.truncate(write);
}

/// The masked scalar reference — the independent differential oracle
/// (house law: every kernel variant answers to a scalar twin,
/// bit-identical).
fn filter_eq_u64_masked_reference(col: &[u64], value: u64, validity: &[u64], out: &mut Vec<u32>) {
    for (i, &item) in col.iter().enumerate() {
        if item == value && (validity[i >> 6] >> (i & 63)) & 1 != 0 {
            out.push(u32::try_from(i).expect("positions fit u32"));
        }
    }
}

/// A column at `sel_pct`% selectivity: matching cells carry [`NEEDLE`],
/// the rest carry the top bit (guaranteed mismatch), fresh per seed.
fn fill_column(col: &mut [u64], seed: u64, sel_pct: u64) {
    for (i, cell) in col.iter_mut().enumerate() {
        let r = mix(seed ^ (i as u64));
        *cell = if r % 100 < sel_pct {
            NEEDLE
        } else {
            r | (1 << 63)
        };
    }
}

/// A validity bitmap with uniform-random dead bits at rate
/// `1/dead_one_in` (all-live for `dead_one_in == 0`).
fn validity_map(seed: u64, rows: usize, dead_one_in: u64) -> Vec<u64> {
    let mut map = vec![u64::MAX; rows.div_ceil(64).max(1)];
    if dead_one_in == 0 {
        return map;
    }
    for i in 0..rows {
        if mix(seed ^ 0xD00D_F00D ^ (i as u64)).is_multiple_of(dead_one_in) {
            map[i >> 6] &= !(1 << (i & 63));
        }
    }
    map
}

/// Median of a sorted-on-demand f64 sample.
fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(f64::total_cmp);
    xs[xs.len() / 2]
}

/// The always-on bit-identity pins (house law — these run in the NORMAL
/// suite, no timing): A′ matches the shipped kernel exactly; the masked
/// kernel under an all-ones bitmap matches the shipped kernel exactly
/// (the mask at density 1.0 is a no-op); the masked kernel under
/// arbitrary bitmaps matches the masked scalar reference exactly.
#[test]
fn masked_kernel_and_local_twin_match_the_references_bit_for_bit() {
    // The boundary-stressing length matrix (the sibling property tests'
    // discipline): lane multiples ± 1 for the 4-lane width, bitmap-word
    // boundaries (64) ± 1, and lengths past the unroll horizon.
    const LENGTHS: &[usize] = &[
        0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 63, 64, 65, 100, 127, 128, 129, 257, 1023, 4099,
    ];
    for (case, &len) in LENGTHS.iter().enumerate() {
        let seed = 0xC0FF_EE00 ^ (case as u64);
        let mut col = vec![0u64; len];
        fill_column(&mut col, seed, 25);
        // A′ == A.
        let (mut shipped, mut local) = (Vec::new(), Vec::new());
        filter_eq_u64(&col, NEEDLE, &mut shipped);
        filter_eq_u64_local(&col, NEEDLE, &mut local);
        assert_eq!(shipped, local, "A' vs A at len {len}");
        // masked(all-live) == A.
        let all_live = validity_map(seed, len, 0);
        let mut masked_live = Vec::new();
        filter_eq_u64_masked(&col, NEEDLE, &all_live, &mut masked_live);
        assert_eq!(shipped, masked_live, "B(all-live) vs A at len {len}");
        // masked(holed) == masked reference, across hole densities
        // including all-dead (every-bit case via dead_one_in = 1).
        for dead_one_in in [1u64, 2, 8, 64] {
            let validity = validity_map(seed ^ dead_one_in, len, dead_one_in);
            let (mut masked, mut reference) = (Vec::new(), Vec::new());
            filter_eq_u64_masked(&col, NEEDLE, &validity, &mut masked);
            filter_eq_u64_masked_reference(&col, NEEDLE, &validity, &mut reference);
            assert_eq!(
                masked, reference,
                "C vs reference at len {len}, 1/{dead_one_in} dead"
            );
        }
    }
}

/// Alternation depth, the `stride_ab` discipline: 5 fresh-data blocks ×
/// 3 interleaved passes each; the reported figure per arm is the min of
/// the 5 block medians.
const BLOCKS: usize = 5;
const PAIRS_PER_BLOCK: usize = 3;

/// The arm roster, in rotation order.
const ARMS: [&str; 5] = [
    "A shipped",
    "A' local",
    "B all-live",
    "C 1/64 dead",
    "C 1/8 dead",
];

/// The interleaved decider twin (ignored: measured evidence — run
/// release through `scripts/measure.sh`, see the module doc for the
/// exact invocation and the decision rule). Prints per-arm min-of-5
/// block medians and the ratios the rule reads (B/A, B/A′, C/A′,
/// A′/A); asserts only arm agreement — the timing verdict belongs to
/// the measured run.
#[test]
#[ignore = "measured decider: run release through scripts/measure.sh"]
#[expect(
    clippy::cast_precision_loss,
    reason = "nanosecond spans and row counts sit far below 2^52"
)]
fn filter_mask_twin_shipped_vs_masked() {
    // L2-resident (~2 MB) and DRAM (~100 MB) columns.
    const TIERS: [(&str, usize); 2] = [("l2", 262_144), ("dram", 13_107_200)];
    // Survivor-write-bound (50%) vs scan-bound (1%).
    const SELECTIVITIES: [u64; 2] = [1, 50];

    for (tier, rows) in TIERS {
        let mut col = vec![0u64; rows];
        let all_live = validity_map(0, rows, 0);
        let mut out: Vec<u32> = Vec::with_capacity(rows);

        for sel_pct in SELECTIVITIES {
            // Arm agreement on a fresh draw, before any timing.
            let agreement_seed = 0xA9EE ^ (rows as u64) ^ sel_pct;
            fill_column(&mut col, agreement_seed, sel_pct);
            let v64 = validity_map(agreement_seed, rows, 64);
            let v8 = validity_map(agreement_seed ^ 8, rows, 8);
            {
                let (mut a, mut b) = (Vec::new(), Vec::new());
                filter_eq_u64(&col, NEEDLE, &mut a);
                filter_eq_u64_local(&col, NEEDLE, &mut b);
                assert_eq!(a, b, "A' vs A [{tier} sel {sel_pct}%]");
                b.clear();
                filter_eq_u64_masked(&col, NEEDLE, &all_live, &mut b);
                assert_eq!(a, b, "B vs A [{tier} sel {sel_pct}%]");
                for validity in [&v64, &v8] {
                    let (mut c, mut r) = (Vec::new(), Vec::new());
                    filter_eq_u64_masked(&col, NEEDLE, validity, &mut c);
                    filter_eq_u64_masked_reference(&col, NEEDLE, validity, &mut r);
                    assert_eq!(c, r, "C vs reference [{tier} sel {sel_pct}%]");
                }
            }

            // Per-arm block medians (ns/row), BLOCKS entries each.
            let mut block_medians: [Vec<f64>; ARMS.len()] = Default::default();
            let mut survivors = 0usize;
            for block in 0..BLOCKS {
                let seed = mix((block as u64 + 1) ^ (rows as u64) ^ sel_pct);
                fill_column(&mut col, seed, sel_pct);
                let v64 = validity_map(seed, rows, 64);
                let v8 = validity_map(seed ^ 8, rows, 8);
                let mut block_ns: [Vec<f64>; ARMS.len()] = Default::default();
                for pass in 0..PAIRS_PER_BLOCK {
                    // Arm order rotates per pass: drift cancellation.
                    let rotate = (block * PAIRS_PER_BLOCK + pass) % ARMS.len();
                    for slot in 0..ARMS.len() {
                        let arm = (slot + rotate) % ARMS.len();
                        out.clear();
                        let t = Instant::now();
                        match arm {
                            0 => filter_eq_u64(&col, NEEDLE, &mut out),
                            1 => filter_eq_u64_local(&col, NEEDLE, &mut out),
                            2 => filter_eq_u64_masked(&col, NEEDLE, &all_live, &mut out),
                            3 => filter_eq_u64_masked(&col, NEEDLE, &v64, &mut out),
                            _ => filter_eq_u64_masked(&col, NEEDLE, &v8, &mut out),
                        }
                        let ns = t.elapsed().as_nanos() as f64;
                        survivors = survivors.max(std::hint::black_box(out.len()));
                        block_ns[arm].push(ns / rows as f64);
                    }
                }
                for (arm, ns) in block_ns.into_iter().enumerate() {
                    block_medians[arm].push(median(ns));
                }
            }

            // Min of the 5 block medians per arm — the reported figure.
            let figure: Vec<f64> = block_medians
                .iter()
                .map(|m| m.iter().copied().fold(f64::INFINITY, f64::min))
                .collect();
            for (arm, name) in ARMS.iter().enumerate() {
                println!(
                    "twin [{tier} sel {sel_pct:>2}%] {name:<12}: min-of-{BLOCKS}-medians \
                     {:.4} ns/row (block medians {:?})",
                    figure[arm],
                    block_medians[arm]
                        .iter()
                        .map(|x| (x * 1e4).round() / 1e4)
                        .collect::<Vec<_>>(),
                );
            }
            println!(
                "twin [{tier} sel {sel_pct:>2}%] RATIOS: B/A {:.4}, B/A' {:.4}, \
                 C64/A' {:.4}, C8/A' {:.4}, A'/A {:.4} (max survivors {survivors})",
                figure[2] / figure[0],
                figure[2] / figure[1],
                figure[3] / figure[1],
                figure[4] / figure[1],
                figure[1] / figure[0],
            );
        }
    }
}
