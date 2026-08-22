//! already-priced dense→gathered conversion (8.8 vs 4.0–4.6 rows/ns,
//! NEON.
//! engine law: no new `unsafe` outside the sanctioned modules, and a
//! before any verdict is drawn from B/A.
//! list at densities 1.0/0.99/0.9/0.5) before any design decision.

use std::simd::prelude::*;
use std::time::Instant;

use super::filter_eq_u64;

const LANES: usize = 4;

const LANE_MASK: u64 = (1 << LANES) - 1;

const NEEDLE: u64 = 7;

fn mix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The safe cursor-write twin of the shipped `write_survivor_bits`: same
/// branchless cursor advance, but an indexed store in place of the documented
/// `get_unchecked_mut` (no new unsafe outside the sanctioned modules).
fn write_survivors(out: &mut [u32], mut write: usize, mut pos: u32, bits: u64) -> (usize, u32) {
    for lane in 0..LANES {
        out[write] = pos;
        write += usize::from((bits >> lane) & 1 != 0);
        pos = pos.wrapping_add(1);
    }
    (write, pos)
}

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

fn filter_eq_u64_masked_reference(col: &[u64], value: u64, validity: &[u64], out: &mut Vec<u32>) {
    for (i, &item) in col.iter().enumerate() {
        if item == value && (validity[i >> 6] >> (i & 63)) & 1 != 0 {
            out.push(u32::try_from(i).expect("positions fit u32"));
        }
    }
}

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

fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(f64::total_cmp);
    xs[xs.len() / 2]
}

#[test]
fn masked_kernel_and_local_twin_match_the_references_bit_for_bit() {
    const LENGTHS: &[usize] = &[
        0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 63, 64, 65, 100, 127, 128, 129, 257, 1023, 4099,
    ];
    for (case, &len) in LENGTHS.iter().enumerate() {
        let seed = 0xC0FF_EE00 ^ (case as u64);
        let mut col = vec![0u64; len];
        fill_column(&mut col, seed, 25);

        let (mut shipped, mut local) = (Vec::new(), Vec::new());
        filter_eq_u64(&col, NEEDLE, &mut shipped);
        filter_eq_u64_local(&col, NEEDLE, &mut local);
        assert_eq!(shipped, local, "A' vs A at len {len}");

        let all_live = validity_map(seed, len, 0);
        let mut masked_live = Vec::new();
        filter_eq_u64_masked(&col, NEEDLE, &all_live, &mut masked_live);
        assert_eq!(shipped, masked_live, "B(all-live) vs A at len {len}");

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

const BLOCKS: usize = 5;
const PAIRS_PER_BLOCK: usize = 3;

const ARMS: [&str; 5] = [
    "A shipped",
    "A' local",
    "B all-live",
    "C 1/64 dead",
    "C 1/8 dead",
];

#[test]
#[ignore = "measured decider: run release through scripts/measure.sh"]
#[expect(
    clippy::cast_precision_loss,
    reason = "nanosecond spans and row counts sit far below 2^52"
)]
fn filter_mask_twin_shipped_vs_masked() {
    const TIERS: [(&str, usize); 2] = [("l2", 262_144), ("dram", 13_107_200)];

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

            let mut block_medians: [Vec<f64>; ARMS.len()] = Default::default();
            let mut survivors = 0usize;
            for block in 0..BLOCKS {
                let seed = mix((block as u64 + 1) ^ (rows as u64) ^ sel_pct);
                fill_column(&mut col, seed, sel_pct);
                let v64 = validity_map(seed, rows, 64);
                let v8 = validity_map(seed ^ 8, rows, 8);
                let mut block_ns: [Vec<f64>; ARMS.len()] = Default::default();
                for pass in 0..PAIRS_PER_BLOCK {
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
