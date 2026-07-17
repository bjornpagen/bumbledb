//! The stride-band falsifier (`m2max.cache.16k-pitch-aliasing`; the
//! dense sweep's correction — Insight 16): the shipped 384 B tolerance
//! against the widened 2 KiB twin, laid out side by side in ONE process
//! and exercised interleaved through the real filter-evaluation surface
//! (`view::apply`'s scalar conjunction — an 8-stream lockstep DRAM
//! scan) and the image's own lockstep multi-column fill (8 store
//! streams, the decode scatter's shape).
//!
//! The layout is TRACED, never assumed: every arm asserts its strides'
//! residues before a single span is timed.
//!
//! VERDICT (2026-07-16, this file's falsifiers, re-runnable): the
//! widening is REFUTED — at image-scale pitches the band decays by
//! ~1.5 KiB of residue (1 KiB costs ~1.1× on the tight kernel, 1.00×
//! on the real filter surface, which is retire-bound) and padding a
//! 2 KiB residue inverts (~0.9×). `PAD_TOLERANCE` stays 384; these
//! tests are the permanent falsifier record.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::image::build::image_with_tolerance;
use crate::image::view::{FilterPredicate, apply};
use crate::image::{Column, RelationImage, SET_STRIDE};
use crate::ir::CmpOp;
use bumbledb_theory::TypeDesc;
use bumbledb_theory::schema::FieldId;
use bumbledb_theory::schema::IntervalElement;

/// The shipped rule (production `PAD_TOLERANCE`).
const SHIPPED: usize = 384;
/// The widened twin the reread proposed — measured and refuted here.
const WIDENED: usize = 2048;

/// 8 u64 columns → 8 lockstep streams (the ledger's ≥8-stream regime).
const COLS: usize = 8;

/// `rows × 8 B = 16 MiB + 1 KiB`: the natural stride lands residue
/// 1024 off a 16 KiB multiple — inside the measured ~48 B–2 KiB poison
/// band, outside the shipped 384 B rule. 8 columns × 16 MiB ≈ 128 MiB
/// per image: DRAM tier.
const POISON_ROWS: usize = 2_097_280;
/// `rows × 8 B = 16 MiB` exactly: residue 0, the measured-clean shape,
/// identical under both rules — the healthy control.
const HEALTHY_ROWS: usize = 2_097_152;

/// Columns whose fill values carry the top bit — chosen so predicates
/// `c0 ≤ c1`, `c2 ≤ c3`, `c4 ≤ c5` hold for EVERY row (the conjunction
/// reads all eight streams) while `c6 ≤ c7` fails for every row (no
/// survivor stream competes with the eight loads).
const HIGH: [u64; COLS] = [0, 1 << 63, 0, 1 << 63, 0, 1 << 63, 1 << 63, 0];

/// splitmix-style avalanche: fresh data per (seed, column, row).
fn mix(seed: u64, c: usize, i: usize) -> u64 {
    let mut z = seed ^ ((c as u64) << 56) ^ (i as u64);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The word-column start indices, traced from the sealed layout.
fn word_starts(image: &RelationImage) -> Vec<usize> {
    image
        .columns
        .iter()
        .map(|c| match *c {
            Column::Words { start } => start,
            Column::Bytes { .. } => unreachable!("all-u64 fixture"),
        })
        .collect()
}

/// Consecutive same-slab strides in bytes.
fn strides(image: &RelationImage) -> Vec<usize> {
    word_starts(image)
        .windows(2)
        .map(|w| (w[1] - w[0]) * 8)
        .collect()
}

/// The lockstep multi-column fill — per row, one store to every column
/// (the decode scatter's stream shape), timed.
fn fill(image: &mut Arc<RelationImage>, seed: u64) -> Duration {
    let img = Arc::get_mut(image).expect("no view holds the arm between spans");
    let rows = img.row_count;
    let starts = word_starts(img);
    let words = &mut img.words;
    let t = Instant::now();
    for i in 0..rows {
        for (c, &start) in starts.iter().enumerate() {
            words[start + i] = (mix(seed, c, i) >> 1) | HIGH[c % COLS];
        }
    }
    t.elapsed()
}

/// The tight 8-stream lockstep read — the ledger's poison regime
/// (`m2max.cache.16k-pitch-aliasing` measured 8.13 vs 1.78–2.09 ns/row
/// on exactly this shape): one dependent-free load per stream per row,
/// nothing else. This is the layout fix's constructed pathological
/// case; the filtered scan below is the family surface.
#[inline(never)]
fn lockstep_sum(image: &RelationImage) -> (Duration, u64) {
    let col = |i: usize| match image.column(i) {
        crate::image::ColumnView::Words(w) => w,
        crate::image::ColumnView::Bytes(_) => unreachable!("all-u64 fixture"),
    };
    let (c0, c1, c2, c3) = (col(0), col(1), col(2), col(3));
    let (c4, c5, c6, c7) = (col(4), col(5), col(6), col(7));
    let rows = image.row_count();
    let mut acc = 0u64;
    let t = Instant::now();
    for i in 0..rows {
        acc = acc
            .wrapping_add(c0[i])
            .wrapping_add(c1[i])
            .wrapping_add(c2[i])
            .wrapping_add(c3[i])
            .wrapping_add(c4[i])
            .wrapping_add(c5[i])
            .wrapping_add(c6[i])
            .wrapping_add(c7[i]);
    }
    (t.elapsed(), acc)
}

/// The 24-stream twin of [`lockstep_sum`] — the second pitch point:
/// the smallest stride at which a real image can hold ≥8 (here 24)
/// simultaneous DRAM-tier streams, 16× closer to the ledger's
/// small-pitch regime than the 8-column configuration.
#[inline(never)]
fn lockstep_sum24(image: &RelationImage) -> (Duration, u64) {
    let col = |i: usize| match image.column(i) {
        crate::image::ColumnView::Words(w) => w,
        crate::image::ColumnView::Bytes(_) => unreachable!("all-u64 fixture"),
    };
    let c: Vec<&[u64]> = (0..24).map(col).collect();
    let (c0, c1, c2, c3, c4, c5, c6, c7) = (c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]);
    let (c8, c9, c10, c11, c12, c13, c14, c15) =
        (c[8], c[9], c[10], c[11], c[12], c[13], c[14], c[15]);
    let (c16, c17, c18, c19, c20, c21, c22, c23) =
        (c[16], c[17], c[18], c[19], c[20], c[21], c[22], c[23]);
    let rows = image.row_count();
    let mut a0 = 0u64;
    let mut a1 = 0u64;
    let mut a2 = 0u64;
    let t = Instant::now();
    for i in 0..rows {
        a0 = a0
            .wrapping_add(c0[i])
            .wrapping_add(c1[i])
            .wrapping_add(c2[i])
            .wrapping_add(c3[i])
            .wrapping_add(c4[i])
            .wrapping_add(c5[i])
            .wrapping_add(c6[i])
            .wrapping_add(c7[i]);
        a1 = a1
            .wrapping_add(c8[i])
            .wrapping_add(c9[i])
            .wrapping_add(c10[i])
            .wrapping_add(c11[i])
            .wrapping_add(c12[i])
            .wrapping_add(c13[i])
            .wrapping_add(c14[i])
            .wrapping_add(c15[i]);
        a2 = a2
            .wrapping_add(c16[i])
            .wrapping_add(c17[i])
            .wrapping_add(c18[i])
            .wrapping_add(c19[i])
            .wrapping_add(c20[i])
            .wrapping_add(c21[i])
            .wrapping_add(c22[i])
            .wrapping_add(c23[i]);
    }
    (t.elapsed(), a0.wrapping_add(a1).wrapping_add(a2))
}

/// One real filtered scan: four same-atom `FieldsCompare` predicates
/// over the eight columns — no kernel shape, so `apply` runs its scalar
/// conjunction, reading all eight streams per row.
fn scan(
    image: &Arc<RelationImage>,
    preds: &[FilterPredicate],
    buf: Vec<u32>,
) -> (Duration, Vec<u32>) {
    let t = Instant::now();
    let view = apply(image, preds, &[], buf).expect("no measure predicates");
    let dt = t.elapsed();
    assert_eq!(view.len(), 0, "the last predicate rejects every row");
    (dt, view.recycle())
}

#[expect(
    clippy::cast_precision_loss,
    reason = "nanosecond spans and row counts sit far below 2^52"
)]
fn ns_per_row(d: Duration, rows: usize) -> f64 {
    d.as_nanos() as f64 / rows as f64
}

fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    xs[xs.len() / 2]
}

/// Alternation depth: 5 fresh-data blocks × 3 A/B pairs each.
const BLOCKS: u64 = 5;
const PAIRS_PER_BLOCK: u64 = 3;

/// The interleaved A/B (ignored: measured evidence, run release through
/// `scripts/measure.sh`). Predicted: ≥3× ns/row on the unpadded shipped
/// rule at the 1 KiB residue; the widened twin at parity with the
/// exact-multiple control; the store-side fill reported alongside.
#[test]
#[ignore = "measured falsifier: run release through scripts/measure.sh"]
#[expect(
    clippy::too_many_lines,
    reason = "the falsifier reads as one protocol: trace, warm, alternate, report"
)]
fn stride_band_ab_falsifier() {
    let field_types = vec![TypeDesc::U64; COLS];
    // Arm A — the shipped rule: residue 1024 passes unpadded (poison).
    let mut poison = image_with_tolerance(&field_types, POISON_ROWS, SHIPPED);
    // Arm B — the widened rule: the same shape pads to the exact multiple.
    let mut cured = image_with_tolerance(&field_types, POISON_ROWS, WIDENED);
    // Control: the exact multiple, identical under both rules.
    let healthy_shipped = image_with_tolerance(&field_types, HEALTHY_ROWS, SHIPPED);
    let mut healthy = image_with_tolerance(&field_types, HEALTHY_ROWS, WIDENED);

    // Trace the layout, never assume it.
    for &s in &strides(&poison) {
        assert_eq!(s % SET_STRIDE, 1024, "arm A sits at the 1 KiB residue");
        assert!(s >= 16 * 1024 * 1024, "DRAM-scale stride");
    }
    for &s in &strides(&cured) {
        assert_eq!(s % SET_STRIDE, 0, "arm B pads to the exact multiple");
    }
    assert_eq!(
        word_starts(&healthy_shipped),
        word_starts(&healthy),
        "exact multiples lay out identically under both rules"
    );
    for &s in &strides(&healthy) {
        assert_eq!(s % SET_STRIDE, 0);
    }
    assert_eq!(
        poison.byte_size(),
        cured.byte_size(),
        "the slack is pre-paid per column: allocation is tolerance-independent"
    );
    drop(healthy_shipped);

    let preds: Vec<FilterPredicate> = (0..4)
        .map(|j| FilterPredicate::FieldsCompare {
            left: FieldId(2 * j),
            right: FieldId(2 * j + 1),
            op: CmpOp::Le,
        })
        .collect();

    // Warm every arm once (untimed): fault the slabs in, settle the
    // allocator; the timed spans then measure steady-state DRAM scans.
    let warm_seed = 0xD6E8_FEB8_6659_FD93;
    fill(&mut poison, warm_seed);
    fill(&mut cured, warm_seed);
    fill(&mut healthy, warm_seed);
    let mut buf = Vec::new();
    for image in [&poison, &cured, &healthy] {
        let (_, b) = scan(image, &preds, std::mem::take(&mut buf));
        buf = b;
        let _ = lockstep_sum(image);
    }

    let mut scan_ratios = Vec::new();
    let mut fill_ratios = Vec::new();
    let mut poison_ns = Vec::new();
    let mut cured_ns = Vec::new();
    let mut healthy_ns = Vec::new();
    let mut kernel_ratios = Vec::new();
    let mut kernel_poison_ns = Vec::new();
    let mut kernel_cured_ns = Vec::new();
    let mut kernel_healthy_ns = Vec::new();
    let mut sink = 0u64;

    for block in 0..BLOCKS {
        // Fresh data per block: the TAGE discipline (the scan loop is
        // near-branchless, but the ledger's law is cheap to obey).
        let seed = 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(block + 1);
        // The store-side lockstep, interleaved, order alternating.
        let (fill_a, fill_b) = if block % 2 == 0 {
            let a = fill(&mut poison, seed);
            let b = fill(&mut cured, seed);
            (a, b)
        } else {
            let b = fill(&mut cured, seed);
            let a = fill(&mut poison, seed);
            (a, b)
        };
        fill(&mut healthy, seed);
        fill_ratios.push(fill_a.as_secs_f64() / fill_b.as_secs_f64());

        for pair in 0..PAIRS_PER_BLOCK {
            let flip = (block * PAIRS_PER_BLOCK + pair) % 2 == 1;
            let (a, b) = if flip {
                let (b, buf2) = scan(&cured, &preds, std::mem::take(&mut buf));
                let (a, buf3) = scan(&poison, &preds, buf2);
                buf = buf3;
                (a, b)
            } else {
                let (a, buf2) = scan(&poison, &preds, std::mem::take(&mut buf));
                let (b, buf3) = scan(&cured, &preds, buf2);
                buf = buf3;
                (a, b)
            };
            let (h, buf2) = scan(&healthy, &preds, std::mem::take(&mut buf));
            buf = buf2;
            // The tight kernel — the pathological case itself,
            // interleaved in the same alternation order.
            let (ka, kb) = if flip {
                let (kb, sb) = lockstep_sum(&cured);
                let (ka, sa) = lockstep_sum(&poison);
                sink = sink.wrapping_add(sa).wrapping_add(sb);
                (ka, kb)
            } else {
                let (ka, sa) = lockstep_sum(&poison);
                let (kb, sb) = lockstep_sum(&cured);
                sink = sink.wrapping_add(sa).wrapping_add(sb);
                (ka, kb)
            };
            let (kh, sh) = lockstep_sum(&healthy);
            sink = sink.wrapping_add(sh);
            kernel_ratios.push(ka.as_secs_f64() / kb.as_secs_f64());
            kernel_poison_ns.push(ns_per_row(ka, POISON_ROWS));
            kernel_cured_ns.push(ns_per_row(kb, POISON_ROWS));
            kernel_healthy_ns.push(ns_per_row(kh, HEALTHY_ROWS));
            println!(
                "block {block} pair {pair} KERNEL: poison {:.2} ns/row, cured {:.2} ns/row, healthy {:.2} ns/row, ratio {:.2}x",
                ns_per_row(ka, POISON_ROWS),
                ns_per_row(kb, POISON_ROWS),
                ns_per_row(kh, HEALTHY_ROWS),
                ka.as_secs_f64() / kb.as_secs_f64(),
            );
            scan_ratios.push(a.as_secs_f64() / b.as_secs_f64());
            poison_ns.push(ns_per_row(a, POISON_ROWS));
            cured_ns.push(ns_per_row(b, POISON_ROWS));
            healthy_ns.push(ns_per_row(h, HEALTHY_ROWS));
            println!(
                "block {block} pair {pair}: poison {:.2} ns/row, cured {:.2} ns/row, healthy {:.2} ns/row, ratio {:.2}x",
                ns_per_row(a, POISON_ROWS),
                ns_per_row(b, POISON_ROWS),
                ns_per_row(h, HEALTHY_ROWS),
                a.as_secs_f64() / b.as_secs_f64(),
            );
        }
    }

    println!(
        "SCAN  poison/cured ratios: median {:.2}x, min {:.2}x, max {:.2}x ({} pairs)",
        median(scan_ratios.clone()),
        scan_ratios.iter().copied().fold(f64::INFINITY, f64::min),
        scan_ratios.iter().copied().fold(0.0, f64::max),
        scan_ratios.len(),
    );
    println!(
        "SCAN  ns/row medians: poison {:.2}, cured {:.2}, healthy {:.2} (cure parity {:.3}x)",
        median(poison_ns),
        median(cured_ns.clone()),
        median(healthy_ns.clone()),
        median(cured_ns) / median(healthy_ns),
    );
    println!(
        "FILL  poison/cured ratios: median {:.2}x, min {:.2}x, max {:.2}x ({} blocks)",
        median(fill_ratios.clone()),
        fill_ratios.iter().copied().fold(f64::INFINITY, f64::min),
        fill_ratios.iter().copied().fold(0.0, f64::max),
        fill_ratios.len(),
    );
    println!(
        "KERNEL poison/cured ratios: median {:.2}x, min {:.2}x, max {:.2}x ({} pairs)",
        median(kernel_ratios.clone()),
        kernel_ratios.iter().copied().fold(f64::INFINITY, f64::min),
        kernel_ratios.iter().copied().fold(0.0, f64::max),
        kernel_ratios.len(),
    );
    println!(
        "KERNEL ns/row medians: poison {:.2}, cured {:.2}, healthy {:.2} (cure parity {:.3}x, sink {sink})",
        median(kernel_poison_ns),
        median(kernel_cured_ns.clone()),
        median(kernel_healthy_ns.clone()),
        median(kernel_cured_ns) / median(kernel_healthy_ns),
    );
}

/// The second pitch point (ignored: measured evidence): 24 u64 columns
/// at `rows × 8 B = 4 MiB + 1 KiB` — the smallest stride family at
/// which a real image holds ≥8 simultaneous DRAM-tier streams
/// (24 × 4 MiB ≈ 101 MB, past the SLC), 16× closer to the ledger's
/// small-pitch regime than the 8 × 16 MiB configuration. Healthy
/// control: `rows × 8 B = 4 MiB` exactly (pure pow-2 pitch).
#[test]
#[ignore = "measured falsifier: run release through scripts/measure.sh"]
fn stride_band_ab_falsifier_small_pitch() {
    const ROWS_POISON: usize = 524_416; // ×8 = 4 MiB + 1 KiB
    const ROWS_HEALTHY: usize = 524_288; // ×8 = 4 MiB exactly
    let field_types = vec![TypeDesc::U64; 24];
    let mut poison = image_with_tolerance(&field_types, ROWS_POISON, SHIPPED);
    let mut cured = image_with_tolerance(&field_types, ROWS_POISON, WIDENED);
    let mut healthy = image_with_tolerance(&field_types, ROWS_HEALTHY, WIDENED);
    for &s in &strides(&poison) {
        assert_eq!(s % SET_STRIDE, 1024, "arm A sits at the 1 KiB residue");
        assert_eq!(s, 4 * 1024 * 1024 + 1024, "the 4 MiB pitch point");
    }
    for &s in &strides(&cured) {
        assert_eq!(s % SET_STRIDE, 0, "arm B pads to the exact multiple");
    }
    for &s in &strides(&healthy) {
        assert_eq!(s, 4 * 1024 * 1024, "pure pow-2 control");
    }
    assert_eq!(poison.byte_size(), cured.byte_size(), "allocation unmoved");

    // Warm (fault the slabs), then interleave kernel-only pairs.
    let warm_seed = 0xA076_1D64_78BD_642F;
    fill(&mut poison, warm_seed);
    fill(&mut cured, warm_seed);
    fill(&mut healthy, warm_seed);
    let mut sink = 0u64;
    for image in [&poison, &cured, &healthy] {
        let (_, s) = lockstep_sum24(image);
        sink = sink.wrapping_add(s);
    }
    let mut ratios = Vec::new();
    let mut poison_ns = Vec::new();
    let mut cured_ns = Vec::new();
    let mut healthy_ns = Vec::new();
    for block in 0..BLOCKS {
        let seed = 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(block + 11);
        fill(&mut poison, seed);
        fill(&mut cured, seed);
        fill(&mut healthy, seed);
        for pair in 0..PAIRS_PER_BLOCK {
            let flip = (block * PAIRS_PER_BLOCK + pair) % 2 == 1;
            let (ka, kb) = if flip {
                let (kb, sb) = lockstep_sum24(&cured);
                let (ka, sa) = lockstep_sum24(&poison);
                sink = sink.wrapping_add(sa).wrapping_add(sb);
                (ka, kb)
            } else {
                let (ka, sa) = lockstep_sum24(&poison);
                let (kb, sb) = lockstep_sum24(&cured);
                sink = sink.wrapping_add(sa).wrapping_add(sb);
                (ka, kb)
            };
            let (kh, sh) = lockstep_sum24(&healthy);
            sink = sink.wrapping_add(sh);
            ratios.push(ka.as_secs_f64() / kb.as_secs_f64());
            poison_ns.push(ns_per_row(ka, ROWS_POISON));
            cured_ns.push(ns_per_row(kb, ROWS_POISON));
            healthy_ns.push(ns_per_row(kh, ROWS_HEALTHY));
            println!(
                "block {block} pair {pair} KERNEL24: poison {:.2} ns/row, cured {:.2} ns/row, healthy {:.2} ns/row, ratio {:.2}x",
                ns_per_row(ka, ROWS_POISON),
                ns_per_row(kb, ROWS_POISON),
                ns_per_row(kh, ROWS_HEALTHY),
                ka.as_secs_f64() / kb.as_secs_f64(),
            );
        }
    }
    println!(
        "KERNEL24 poison/cured ratios: median {:.2}x, min {:.2}x, max {:.2}x ({} pairs)",
        median(ratios.clone()),
        ratios.iter().copied().fold(f64::INFINITY, f64::min),
        ratios.iter().copied().fold(0.0, f64::max),
        ratios.len(),
    );
    println!(
        "KERNEL24 ns/row medians: poison {:.2}, cured {:.2}, healthy {:.2} (cure parity {:.3}x, sink {sink})",
        median(poison_ns),
        median(cured_ns.clone()),
        median(healthy_ns.clone()),
        median(cured_ns) / median(healthy_ns),
    );
}

/// The discriminator (ignored: measured evidence): the ledger's own
/// headline stagger — residue 128 B, which even the shipped 384 B rule
/// pads — at the 4 MiB pitch point. If this also measures ~1×, no
/// residue in the band reproduces at image-scale pitches (the poison
/// band is a small-pitch phenomenon the image's own geometry cannot
/// reach); if it measures ≥3×, the band is real here but narrower than
/// 1 KiB. Tolerance 0 constructs the never-padded natural layout.
#[test]
#[ignore = "measured falsifier: run release through scripts/measure.sh"]
fn stride_band_ab_residue_128_discriminator() {
    const ROWS: usize = 524_304; // ×8 = 4 MiB + 128 B
    let field_types = vec![TypeDesc::U64; 24];
    let mut poison = image_with_tolerance(&field_types, ROWS, 0);
    let mut cured = image_with_tolerance(&field_types, ROWS, SHIPPED);
    for &s in &strides(&poison) {
        assert_eq!(s, 4 * 1024 * 1024 + 128, "the ledger's headline stagger");
    }
    for &s in &strides(&cured) {
        assert_eq!(s % SET_STRIDE, 0, "the shipped rule pads 128 B residues");
    }
    let warm_seed = 0xE703_7ED1_A0B4_28DB;
    fill(&mut poison, warm_seed);
    fill(&mut cured, warm_seed);
    let mut sink = 0u64;
    for image in [&poison, &cured] {
        let (_, s) = lockstep_sum24(image);
        sink = sink.wrapping_add(s);
    }
    let mut ratios = Vec::new();
    let mut poison_ns = Vec::new();
    let mut cured_ns = Vec::new();
    for block in 0..BLOCKS {
        let seed = 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(block + 29);
        fill(&mut poison, seed);
        fill(&mut cured, seed);
        for pair in 0..PAIRS_PER_BLOCK {
            let flip = (block * PAIRS_PER_BLOCK + pair) % 2 == 1;
            let (ka, kb) = if flip {
                let (kb, sb) = lockstep_sum24(&cured);
                let (ka, sa) = lockstep_sum24(&poison);
                sink = sink.wrapping_add(sa).wrapping_add(sb);
                (ka, kb)
            } else {
                let (ka, sa) = lockstep_sum24(&poison);
                let (kb, sb) = lockstep_sum24(&cured);
                sink = sink.wrapping_add(sa).wrapping_add(sb);
                (ka, kb)
            };
            ratios.push(ka.as_secs_f64() / kb.as_secs_f64());
            poison_ns.push(ns_per_row(ka, ROWS));
            cured_ns.push(ns_per_row(kb, ROWS));
            println!(
                "block {block} pair {pair} KERNEL24/128B: poison {:.2} ns/row, cured {:.2} ns/row, ratio {:.2}x",
                ns_per_row(ka, ROWS),
                ns_per_row(kb, ROWS),
                ka.as_secs_f64() / kb.as_secs_f64(),
            );
        }
    }
    println!(
        "KERNEL24/128B poison/cured ratios: median {:.2}x, min {:.2}x, max {:.2}x ({} pairs); ns/row medians poison {:.2}, cured {:.2} (sink {sink})",
        median(ratios.clone()),
        ratios.iter().copied().fold(f64::INFINITY, f64::min),
        ratios.iter().copied().fold(0.0, f64::max),
        ratios.len(),
        median(poison_ns),
        median(cured_ns),
    );
}

/// The band-edge sweep (ignored: measured evidence): residues 128 B –
/// 2 KiB at the 4 MiB pitch point, each unpadded arm (tolerance 0)
/// against the padded cure, interleaved — locates where the poison
/// band ends at image-scale pitches, i.e. whether the shipped 384 B
/// tolerance is already right-sized there.
#[test]
#[ignore = "measured falsifier: run release through scripts/measure.sh"]
fn stride_band_residue_sweep() {
    let field_types = vec![TypeDesc::U64; 24];
    for residue in [128usize, 256, 384, 512, 768, 1024, 1536, 2048] {
        let rows = (4 * 1024 * 1024 + residue) / 8;
        let mut poison = image_with_tolerance(&field_types, rows, 0);
        let mut cured = image_with_tolerance(&field_types, rows, WIDENED);
        for &s in &strides(&poison) {
            assert_eq!(s, 4 * 1024 * 1024 + residue, "the natural stride");
        }
        for &s in &strides(&cured) {
            assert_eq!(s % SET_STRIDE, 0, "the cure pads to the multiple");
        }
        let mut sink = 0u64;
        let mut ratios = Vec::new();
        let mut poison_ns = Vec::new();
        let mut cured_ns = Vec::new();
        for block in 0..2u64 {
            let seed = 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(block + 47 + residue as u64);
            fill(&mut poison, seed);
            fill(&mut cured, seed);
            if block == 0 {
                // Warm pass (untimed ratios discarded via warmup scans).
                let (_, s0) = lockstep_sum24(&poison);
                let (_, s1) = lockstep_sum24(&cured);
                sink = sink.wrapping_add(s0).wrapping_add(s1);
            }
            for pair in 0..PAIRS_PER_BLOCK {
                let flip = (block * PAIRS_PER_BLOCK + pair) % 2 == 1;
                let (ka, kb) = if flip {
                    let (kb, sb) = lockstep_sum24(&cured);
                    let (ka, sa) = lockstep_sum24(&poison);
                    sink = sink.wrapping_add(sa).wrapping_add(sb);
                    (ka, kb)
                } else {
                    let (ka, sa) = lockstep_sum24(&poison);
                    let (kb, sb) = lockstep_sum24(&cured);
                    sink = sink.wrapping_add(sa).wrapping_add(sb);
                    (ka, kb)
                };
                ratios.push(ka.as_secs_f64() / kb.as_secs_f64());
                poison_ns.push(ns_per_row(ka, rows));
                cured_ns.push(ns_per_row(kb, rows));
            }
        }
        println!(
            "SWEEP residue {residue:>4}: median {:.2}x (min {:.2}x, max {:.2}x, {} pairs), poison {:.2} ns/row, cured {:.2} ns/row (sink {sink})",
            median(ratios.clone()),
            ratios.iter().copied().fold(f64::INFINITY, f64::min),
            ratios.iter().copied().fold(0.0, f64::max),
            ratios.len(),
            median(poison_ns),
            median(cured_ns),
        );
    }
}

/// The falsifier's placement mechanism itself (fast, always-on): the
/// parameterized padder lays the two arms out exactly as the twins
/// claim — the shipped tolerance leaves a 1 KiB residue unpadded, the
/// widened twin pads it to the exact multiple — and the allocation
/// never moves (the slack is pre-paid per column).
#[test]
fn widened_band_pads_the_kilobyte_residue() {
    let field_types = vec![TypeDesc::U64; COLS];
    // 8320 × 8 B = 65 KiB = 4 × 16 KiB + 1 KiB: past the tier gate,
    // residue 1024.
    let rows = 8320;
    let shipped = image_with_tolerance(&field_types, rows, SHIPPED);
    let widened = image_with_tolerance(&field_types, rows, WIDENED);
    for &s in &strides(&shipped) {
        assert_eq!(s % SET_STRIDE, 1024, "the shipped rule passes the residue");
    }
    for &s in &strides(&widened) {
        assert_eq!(s % SET_STRIDE, 0, "the widened rule pads it out");
    }
    assert_eq!(
        shipped.byte_size(),
        widened.byte_size(),
        "allocation unmoved"
    );
}

/// The standard-corpus spot check, traced through the real allocator:
/// the refuted widening would have moved exactly one ledger-corpus
/// layout — Holder at L (its 100,096 B stride sits at residue 1792,
/// where the sweep measured the band already past parity) — every
/// other relation's layout is byte-identical at every scale, and the
/// allocated bytes never change anywhere (the per-column slack is
/// pre-paid). Kept always-on: it pins the tolerance-independence of
/// allocation and the corpus's distance from the band.
///
/// Shapes and row counts transcribed from `bumbledb-bench`
/// (`src/schema.rs`, `corpus_gen::Sizes::of`): postings P with
/// `accounts = P/200`, `holders = accounts/4`, `entries = P/2`,
/// `mandates = accounts × 4`.
#[test]
fn corpus_shapes_move_only_where_the_band_says() {
    use TypeDesc::{I64, Interval, String as Str, U64};
    let iv = Interval {
        element: IntervalElement::I64,
        width: None,
    };
    // (name, field types, [S rows, M rows, L rows])
    let shapes: Vec<(&str, Vec<TypeDesc>, [usize; 3])> = vec![
        ("Holder", vec![U64, Str], [125, 1_250, 12_500]),
        ("Account", vec![U64, U64, U64], [500, 5_000, 50_000]),
        ("Instrument", vec![U64, Str], [512, 512, 512]),
        (
            "JournalEntry",
            vec![U64, U64, I64],
            [50_000, 500_000, 5_000_000],
        ),
        (
            "Posting",
            vec![U64, U64, U64, U64, I64, I64],
            [100_000, 1_000_000, 10_000_000],
        ),
        (
            "PostingTag",
            vec![U64, U64],
            [100_000, 1_000_000, 10_000_000],
        ),
        ("Org", vec![U64, Str], [64, 64, 64]),
        ("OrgParent", vec![U64, U64], [63, 63, 63]),
        ("Mandate", vec![U64, U64, iv], [2_000, 20_000, 200_000]),
    ];
    let mut moved = Vec::new();
    for (name, field_types, rows_by_scale) in &shapes {
        for (scale, &rows) in ["S", "M", "L"].iter().zip(rows_by_scale) {
            let shipped = image_with_tolerance(field_types, rows, SHIPPED);
            let widened = image_with_tolerance(field_types, rows, WIDENED);
            assert_eq!(
                shipped.byte_size(),
                widened.byte_size(),
                "{name}@{scale}: allocation is tolerance-independent"
            );
            if word_starts(&shipped) != word_starts(&widened) {
                moved.push(format!("{name}@{scale}"));
            }
        }
    }
    assert_eq!(
        moved,
        vec!["Holder@L".to_string()],
        "exactly one corpus layout sits in the widened band"
    );
}
