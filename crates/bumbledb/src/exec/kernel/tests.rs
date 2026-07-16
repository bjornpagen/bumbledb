use super::*;

/// A deterministic LCG so the property sweeps are reproducible.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
}

/// Lengths that stress lane boundaries: empty, single, odd, lane
/// multiples +/- 1 for both kernel widths (4 × u64 chunks, 16 × u8),
/// plus lengths past the chunk loop's unroll horizon.
const LENGTHS: &[usize] = &[
    0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 257, 1023, 4099,
];

#[test]
fn u64_kernels_match_the_scalar_reference_bit_for_bit() {
    let mut rng = Lcg(42);
    for &len in LENGTHS {
        // Narrow value range forces plenty of matches; extremes too.
        let col: Vec<u64> = (0..len)
            .map(|_| match rng.next() % 8 {
                0 => 0,
                1 => u64::MAX,
                n => n % 4,
            })
            .collect();
        for needle in [0u64, 1, 2, 3, u64::MAX] {
            let (mut kernel, mut reference) = (Vec::new(), Vec::new());
            filter_eq_u64(&col, needle, &mut kernel);
            super::reference::filter_eq_u64(&col, needle, &mut reference);
            assert_eq!(kernel, reference, "eq len {len} needle {needle}");
        }
        for (lo, hi) in [(0u64, 2u64), (1, 1), (3, u64::MAX), (u64::MAX, 0)] {
            let (mut kernel, mut reference) = (Vec::new(), Vec::new());
            filter_range_u64(&col, lo, hi, &mut kernel);
            super::reference::filter_range_u64(&col, lo, hi, &mut reference);
            assert_eq!(kernel, reference, "range len {len} {lo}..={hi}");
        }
    }
}

#[test]
fn u8_kernel_matches_the_scalar_reference() {
    let mut rng = Lcg(7);
    for &len in LENGTHS {
        let col: Vec<u8> = (0..len)
            .map(|_| u8::try_from(rng.next() % 3).expect("small"))
            .collect();
        for needle in [0u8, 1, 2, 255] {
            let (mut kernel, mut reference) = (Vec::new(), Vec::new());
            filter_eq_u8(&col, needle, &mut kernel);
            super::reference::filter_eq_u8(&col, needle, &mut reference);
            assert_eq!(kernel, reference, "u8 eq len {len} needle {needle}");
        }
    }
}

/// PRD 17 (the 00-product unsafe policy): the membership filter
/// compositions — `PointIn` and `AnyPointIn` — are bit-identical to the
/// scalar reference across the boundary shapes: empty, single, odd
/// lengths, lane ±1.
#[test]
fn interval_filter_compositions_match_the_scalar_reference_bit_for_bit() {
    let mut rng = Lcg(1717);
    for &len in LENGTHS {
        // Interval columns with heavy boundary mass: starts around small
        // values, ends strictly greater, extremes included.
        let starts: Vec<u64> = (0..len)
            .map(|_| match rng.next() % 8 {
                0 => 0,
                1 => u64::MAX - 1,
                n => n % 6,
            })
            .collect();
        let ends: Vec<u64> = starts
            .iter()
            .map(|s| match rng.next() % 4 {
                0 => s.saturating_add(1).max(1),
                1 => u64::MAX,
                n => s.saturating_add(n + 1).max(1),
            })
            .collect();
        for point in [0u64, 1, 2, 5, u64::MAX - 1, u64::MAX] {
            let (mut kernel, mut reference) = (Vec::new(), Vec::new());
            filter_point_in_u64(&starts, &ends, point, &mut kernel);
            super::reference::filter_point_in_u64(&starts, &ends, point, &mut reference);
            assert_eq!(kernel, reference, "point_in len {len} point {point}");
        }
        for points in [&[][..], &[3][..], &[0, 4][..], &[1, 2, 5, u64::MAX - 1][..]] {
            let (mut kernel, mut reference) = (Vec::new(), Vec::new());
            filter_any_point_in_u64(&starts, &ends, points, &mut kernel);
            super::reference::filter_any_point_in_u64(&starts, &ends, points, &mut reference);
            assert_eq!(kernel, reference, "any_point_in len {len} {points:?}");
        }
    }
}

/// The measure scan (20-query-ir § the measure) — the one gather+subtract shape — is
/// bit-identical to the scalar reference across the boundary shapes,
/// range extremes included, and both report the SAME first ray position
/// when a ray is present.
#[test]
fn duration_filter_matches_the_scalar_reference_bit_for_bit() {
    let mut rng = Lcg(1010);
    for &len in LENGTHS {
        for with_ray in [false, true] {
            let starts: Vec<u64> = (0..len)
                .map(|_| match rng.next() % 8 {
                    0 => 0,
                    1 => u64::MAX - 3,
                    n => n % 6,
                })
                .collect();
            let ends: Vec<u64> = starts
                .iter()
                .map(|s| {
                    if with_ray && rng.next().is_multiple_of(5) {
                        u64::MAX // the ray: no finite measure
                    } else {
                        s.saturating_add(rng.next() % 7 + 1).min(u64::MAX - 1)
                    }
                })
                .collect();
            for (lo, hi) in [(0u64, 2u64), (1, 1), (3, u64::MAX), (1, 0), (0, u64::MAX)] {
                let (mut kernel, mut reference) = (Vec::new(), Vec::new());
                let kernel_result = filter_duration_range_u64(&starts, &ends, lo, hi, &mut kernel);
                let reference_result = super::reference::filter_duration_range_u64(
                    &starts,
                    &ends,
                    lo,
                    hi,
                    &mut reference,
                );
                assert_eq!(
                    kernel_result, reference_result,
                    "ray verdict len {len} {lo}..={hi}"
                );
                if kernel_result.is_ok() {
                    assert_eq!(kernel, reference, "duration len {len} {lo}..={hi}");
                }
            }
        }
    }
}

/// The measure boundary values, pinned at the kernel level: `[x, x+1)`
/// measures 1, `[MIN, MAX−1)` measures `MAX−1`, and `end == MAX` is the
/// typed ray refusal, not a value.
#[test]
fn duration_filter_boundary_intervals_and_the_ray() {
    let starts = [10u64, 0, 7];
    let ends = [11u64, u64::MAX - 1, u64::MAX];
    let mut out = Vec::new();
    // The ray at position 2 outranks any survivors.
    assert_eq!(
        filter_duration_range_u64(&starts, &ends, 0, u64::MAX, &mut out),
        Err(2)
    );
    out.clear();
    assert_eq!(
        filter_duration_range_u64(&starts[..2], &ends[..2], 1, 1, &mut out),
        Ok(())
    );
    assert_eq!(out, vec![0], "[x, x+1) measures exactly 1");
    out.clear();
    assert_eq!(
        filter_duration_range_u64(
            &starts[..2],
            &ends[..2],
            u64::MAX - 1,
            u64::MAX - 1,
            &mut out
        ),
        Ok(())
    );
    assert_eq!(out, vec![1], "[MIN, MAX-1) measures MAX-1");
}

/// The membership boundary rule, pinned at the kernel level: `p == start`
/// survives, `p == end` does not (half-open, `10-data-model.md`).
#[test]
fn point_in_is_half_open_at_both_boundaries() {
    let starts = [10u64, 10, 10];
    let ends = [20u64, 20, 20];
    let mut out = Vec::new();
    filter_point_in_u64(&starts, &ends, 10, &mut out);
    assert_eq!(out, vec![0, 1, 2], "p == start is in");
    out.clear();
    filter_point_in_u64(&starts, &ends, 20, &mut out);
    assert!(out.is_empty(), "p == end is out");
}

#[test]
fn results_preserve_ascending_position_order() {
    let col: Vec<u64> = (0..1000).map(|i| i % 5).collect();
    let mut out = Vec::new();
    filter_eq_u64(&col, 3, &mut out);
    assert!(out.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(out.len(), 200);
}

/// The fold kernels are bit-identical to naive
/// folds across strides, boundary words, duplicate and reversed
/// indices, and lane-boundary lengths.
#[test]
fn fold_kernels_match_the_naive_folds_bit_for_bit() {
    let mut rng = Lcg(99);
    for &len in LENGTHS {
        for &stride in &[1usize, 2, 3, 5] {
            for &offset in &[0usize, 1] {
                if stride == 1 && offset > 0 {
                    continue;
                }
                let slots = len * stride + offset + 1;
                let values: Vec<u64> = (0..slots)
                    .map(|_| match rng.next() % 6 {
                        0 => 0,
                        1 => u64::MAX,
                        2 => 1 << 63,       // i64 0
                        3 => (1 << 63) - 1, // i64 -1's neighbor
                        _ => rng.next(),
                    })
                    .collect();
                // Indices with duplicates, reversed order, and gaps.
                let mut indices: Vec<u32> =
                    (0..len).map(|i| u32::try_from(i).expect("small")).collect();
                indices.reverse();
                if len > 2 {
                    indices.push(1);
                    indices.push(1);
                }

                let at = |i: u32| values[i as usize * stride + offset];
                let naive_sum_i: i128 = indices
                    .iter()
                    .map(|&i| i128::from(super::biased_to_i64(at(i))))
                    .sum();
                let naive_sum_u: u128 = indices.iter().map(|&i| u128::from(at(i))).sum();
                assert_eq!(
                    fold_sum_biased_i64_idx(&values, stride, offset, &indices),
                    naive_sum_i,
                    "len {len} stride {stride} offset {offset}"
                );
                assert_eq!(
                    fold_sum_u64_idx(&values, stride, offset, &indices),
                    naive_sum_u
                );
                if !indices.is_empty() {
                    let naive_min = indices.iter().map(|&i| at(i)).min().expect("nonempty");
                    let naive_max = indices.iter().map(|&i| at(i)).max().expect("nonempty");
                    assert_eq!(
                        fold_min_max_u64_idx(&values, stride, offset, &indices),
                        (naive_min, naive_max)
                    );
                }

                // Contiguous forms over the dense prefix.
                let naive_dense_i: i128 = (0..len)
                    .map(|i| i128::from(super::biased_to_i64(values[i * stride + offset])))
                    .sum();
                let naive_dense_u: u128 = (0..len)
                    .map(|i| u128::from(values[i * stride + offset]))
                    .sum();
                assert_eq!(
                    fold_sum_biased_i64(&values, stride, offset, len),
                    naive_dense_i
                );
                assert_eq!(fold_sum_u64(&values, stride, offset, len), naive_dense_u);
                if len > 0 {
                    let dmin = (0..len)
                        .map(|i| values[i * stride + offset])
                        .min()
                        .expect("nonempty");
                    let dmax = (0..len)
                        .map(|i| values[i * stride + offset])
                        .max()
                        .expect("nonempty");
                    assert_eq!(fold_min_max_u64(&values, stride, offset, len), (dmin, dmax));
                }
            }
        }
    }
}

/// Fold-throughput evidence (a gate; ignored: a timing
/// test runs only by hand —
/// `cargo test -p bumbledb --release fold_throughput -- --ignored --nocapture`).
/// The gates: ≥ 7 rows/ns exact dense sums on the reference host
/// (the measured kernel ceiling is 8.8; scalar-era
/// baseline was 2.45–4.6).
#[test]
#[ignore = "timing evidence, run by hand on the reference host"]
fn fold_throughput_contiguous_sum() {
    // L2-resident: 1M words = 8 MB... use 256k words (2 MB) so the
    // fold measures the execution core, not DRAM (where every
    // parallel kernel converges at ~7.5 rows/ns anyway).
    let values: Vec<u64> = (0..262_144u64).map(|i| i ^ (1 << 63)).collect();
    let rate_of = |label: &str, f: &mut dyn FnMut() -> i128| {
        let mut sink = 0i128;
        for _ in 0..3 {
            sink += f();
        }
        let start = std::time::Instant::now();
        let reps = 400;
        for _ in 0..reps {
            sink += f();
        }
        let elapsed = start.elapsed();
        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )] // both far below 2^52
        let rate = (values.len() as u64 * reps) as f64
            / u64::try_from(elapsed.as_nanos().max(1)).expect("short run") as f64;
        println!("{label}: {rate:.2} rows/ns (sink {sink})");
        rate
    };
    let biased = rate_of("fold_sum_biased_i64 dense", &mut || {
        fold_sum_biased_i64(&values, 1, 0, values.len())
    });
    let unsigned = rate_of("fold_sum_u64 dense", &mut || {
        #[expect(
            clippy::cast_possible_wrap,
            reason = "the benchmark intentionally reinterprets the unsigned result"
        )]
        {
            fold_sum_u64(&values, 1, 0, values.len()) as i128
        }
    });
    assert!(
        biased >= 7.0,
        "exact biased dense sum ≥7 rows/ns, got {biased:.2}"
    );
    assert!(
        unsigned >= 7.0,
        "exact u64 dense sum ≥7 rows/ns, got {unsigned:.2}"
    );
}

/// The A arm of the predicate-scan reshape twin (perf round T1): the
/// retired 2-lane shape, verbatim — per-lane `mask.test` extracts
/// (vector→GPR transfer + flag-class increment per lane), per-lane
/// `u32::try_from` guard, per-lane `out[write]` bounds check. Kept
/// test-only as the interleaved baseline; the shipped kernel is the
/// 4-lane bitmask shape in `super::filter`.
mod ab_baseline {
    use std::simd::prelude::*;

    const U64_LANES: usize = 2;

    pub fn filter_eq_u64(col: &[u64], value: u64, out: &mut Vec<u32>) {
        let needle = Simd::splat(value);
        push_matching::<U64_LANES>(col, out, |lanes| lanes.simd_eq(needle), |x| x == value);
    }

    pub fn filter_range_u64(col: &[u64], lo: u64, hi: u64, out: &mut Vec<u32>) {
        let lo_v = Simd::splat(lo);
        let hi_v = Simd::splat(hi);
        push_matching::<U64_LANES>(
            col,
            out,
            |lanes| lanes.simd_ge(lo_v) & lanes.simd_le(hi_v),
            |x| (lo..=hi).contains(&x),
        );
    }

    pub fn filter_point_in_u64(starts: &[u64], ends: &[u64], point: u64, out: &mut Vec<u32>) {
        let p = Simd::splat(point);
        let start = out.len();
        out.resize(start + starts.len(), 0);
        let mut write = start;
        let (chunks, tail) = starts.as_chunks::<U64_LANES>();
        let tail_start = starts.len() - tail.len();
        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let base = chunk_idx * U64_LANES;
            let s = Simd::from_array(*chunk);
            let e = Simd::<u64, U64_LANES>::from_slice(&ends[base..base + U64_LANES]);
            let mask = s.simd_le(p) & e.simd_gt(p);
            write = write_survivors(out, write, base, mask);
        }
        for i in tail_start..starts.len() {
            out[write] = u32::try_from(i).expect("positions fit u32");
            write += usize::from(starts[i] <= point && point < ends[i]);
        }
        out.truncate(write);
    }

    fn push_matching<const N: usize>(
        col: &[u64],
        out: &mut Vec<u32>,
        keep: impl Fn(Simd<u64, N>) -> Mask<i64, N>,
        keep1: impl Fn(u64) -> bool,
    ) {
        let start = out.len();
        out.resize(start + col.len(), 0);
        let mut write = start;
        let (chunks, tail) = col.as_chunks::<N>();
        let tail_start = col.len() - tail.len();
        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mask = keep(Simd::from_array(*chunk));
            write = write_survivors(out, write, chunk_idx * N, mask);
        }
        for (i, &item) in tail.iter().enumerate() {
            out[write] = u32::try_from(tail_start + i).expect("positions fit u32");
            write += usize::from(keep1(item));
        }
        out.truncate(write);
    }

    fn write_survivors<const N: usize>(
        out: &mut [u32],
        mut write: usize,
        base: usize,
        mask: Mask<i64, N>,
    ) -> usize {
        for lane in 0..N {
            out[write] = u32::try_from(base + lane).expect("positions fit u32");
            write += usize::from(mask.test(lane));
        }
        write
    }
}

/// The predicate-scan reshape twin (perf round T1), interleaved in one
/// process: A = the retired 2-lane per-lane-extract shape
/// ([`ab_baseline`]), B = the shipped 4-lane bitmask shape. Regimes per
/// the ledger's law: L1 (64 KB), L2 (2 MB), the 24–50 MiB resurgence
/// band (32 MB), DRAM (256 MB). Many A/B alternations per cell; the
/// per-pair ratio distribution is the result (absolute numbers are void
/// under co-tenancy). Run by hand under the machine mutex:
/// `scripts/measure.sh cargo test -p bumbledb --release
/// filter_ab_predicate_scan_reshape -- --ignored --nocapture`.
#[test]
#[ignore = "timing evidence, run by hand on the reference host"]
#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn filter_ab_predicate_scan_reshape() {
    let mut rng = Lcg(0xAB_2026);
    // (label, items) — items are u64 words, 8 B each.
    let cells: &[(&str, usize)] = &[
        ("L1   64KB", 8_192),
        ("L2    2MB", 262_144),
        ("band 32MB", 4_194_304),
        ("DRAM256MB", 33_554_432),
    ];
    // Selectivity via value domain: values uniform in 0..domain, the
    // needle/range picks ~1/domain of them.
    for &(label, items) in cells {
        for &(sel_label, domain) in &[("50%", 2u64), ("2%", 50u64)] {
            let col: Vec<u64> = (0..items).map(|_| rng.next() % domain).collect();
            let starts: Vec<u64> = col.iter().map(|&v| v * 10).collect();
            let ends: Vec<u64> = starts.iter().map(|&s| s + 10).collect();
            let mut out: Vec<u32> = Vec::with_capacity(items);
            // Reps sized so each span is >= ~1 ms (sub-µs single shots
            // are a protocol violation).
            let reps = (2_000_000 / items).max(1);
            let mut time = |f: &mut dyn FnMut(&mut Vec<u32>)| {
                let t0 = std::time::Instant::now();
                for _ in 0..reps {
                    out.clear();
                    f(&mut out);
                }
                let dt = t0.elapsed().as_nanos();
                std::hint::black_box(&out);
                u64::try_from(dt).expect("short span") as f64 / (items * reps) as f64
            };
            let mut report = |name: &str,
                              a: &mut dyn FnMut(&mut Vec<u32>),
                              b: &mut dyn FnMut(&mut Vec<u32>)| {
                // Exactness first: the twins' survivor sets are
                // bit-for-bit identical on this cell's data.
                let (mut out_a, mut out_b) = (Vec::new(), Vec::new());
                a(&mut out_a);
                b(&mut out_b);
                assert_eq!(out_a, out_b, "the twins diverge: {name}");
                drop((out_a, out_b));
                // Warm both arms once, then alternate.
                let _ = time(a);
                let _ = time(b);
                let mut ratios: Vec<f64> = Vec::new();
                for _ in 0..12 {
                    let ta = time(a);
                    let tb = time(b);
                    ratios.push(ta / tb);
                }
                ratios.sort_by(f64::total_cmp);
                let median = ratios[ratios.len() / 2];
                let lo = ratios[1];
                let hi = ratios[ratios.len() - 2];
                println!(
                    "{label} {sel_label:>3} {name:<9} A/B median {median:.3} (p10 {lo:.3}, p90 {hi:.3})"
                );
            };
            report(
                "eq_u64",
                &mut |out| ab_baseline::filter_eq_u64(&col, 1, out),
                &mut |out| filter_eq_u64(&col, 1, out),
            );
            report(
                "range_u64",
                &mut |out| ab_baseline::filter_range_u64(&col, 0, 0, out),
                &mut |out| filter_range_u64(&col, 0, 0, out),
            );
            report(
                "point_in",
                &mut |out| ab_baseline::filter_point_in_u64(&starts, &ends, 5, out),
                &mut |out| filter_point_in_u64(&starts, &ends, 5, out),
            );
        }
    }
}

/// The configuration kernel's boundary corpus: four endpoint streams of
/// length `len` with heavy boundary mass — a small domain so adjacency
/// and equal endpoints occur constantly, nesting by construction, and
/// rays (`end == u64::MAX`, the point-domain law) mixed in. The leading
/// pairs pin the named shapes (adjacent, nested, equal, rays) whenever
/// `len` admits them.
fn allen_corpus(len: usize, rng: &mut Lcg) -> (Vec<u64>, Vec<u64>, Vec<u64>, Vec<u64>) {
    const MAX: u64 = u64::MAX;
    let named: &[(u64, u64, u64, u64)] = &[
        (0, 5, 5, 9),     // adjacent (meets)
        (5, 9, 0, 5),     // adjacent (met-by)
        (0, 10, 3, 7),    // nested (contains)
        (3, 7, 0, 10),    // nested (during)
        (2, 6, 2, 6),     // equal
        (3, MAX, 7, MAX), // two rays
        (0, 5, 5, MAX),   // meets a ray
        (2, MAX, 2, 6),   // started-by, bounded inside a ray
    ];
    let (mut a_s, mut a_e) = (Vec::with_capacity(len), Vec::with_capacity(len));
    let (mut b_s, mut b_e) = (Vec::with_capacity(len), Vec::with_capacity(len));
    for i in 0..len {
        let (x_s, x_e, y_s, y_e) = if i < named.len() {
            named[i]
        } else {
            let mut draw = || {
                let s = rng.next() % 12;
                match rng.next() % 4 {
                    0 => (s, MAX), // a ray flavor per few pairs
                    n => (s, s + 1 + n % 12),
                }
            };
            let ((x_s, x_e), (y_s, y_e)) = (draw(), draw());
            (x_s, x_e, y_s, y_e)
        };
        a_s.push(x_s);
        a_e.push(x_e);
        b_s.push(y_s);
        b_e.push(y_e);
    }
    (a_s, a_e, b_s, b_e)
}

/// Lengths that stress the configuration kernel's window widths (8 for
/// codes, 16 for the mask `tbl`): lane multiples ±1 for both, plus the
/// small-batch scalar fallbacks.
const ALLEN_LENGTHS: &[usize] = &[0, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 32, 33, 100, 257];

/// The unsafe-allowlist law for the configuration kernel:
/// `allen_code_batch` is bit-identical to the scalar reference AND to
/// PRD 03's `classify` (the reference is the decision tree; the kernel
/// is the signature table — the test cross-checks table against tree)
/// across randomized inputs including every boundary shape: adjacent,
/// nested, equal, rays, lane-multiple ±1 lengths.
#[test]
fn allen_code_batch_matches_reference_and_classify_bit_for_bit() {
    let mut rng = Lcg(0xA11E);
    for &len in ALLEN_LENGTHS {
        let (a_s, a_e, b_s, b_e) = allen_corpus(len, &mut rng);
        let mut kernel = Vec::new();
        allen_code_batch(&a_s, &a_e, &b_s, &b_e, &mut kernel);
        let mut reference = vec![0u8; len];
        super::reference::allen_codes(&a_s, &a_e, &b_s, &b_e, &mut reference);
        assert_eq!(kernel, reference, "codes len {len}");
        for i in 0..len {
            let a = crate::interval::Interval::<u64>::new(a_s[i], a_e[i]).expect("nonempty");
            let b = crate::interval::Interval::<u64>::new(b_s[i], b_e[i]).expect("nonempty");
            assert_eq!(
                kernel[i],
                crate::allen::classify(a, b) as u8,
                "classify at {i} of len {len}: {:?} vs {:?}",
                (a_s[i], a_e[i]),
                (b_s[i], b_e[i]),
            );
        }
        // The constant-operand reference against classify too (its live
        // dispatch reader is the non-aarch64 build; here it is oracle-
        // checked on every target).
        let (c_s, c_e) = (3u64, 9u64);
        let c = crate::interval::Interval::<u64>::new(c_s, c_e).expect("nonempty");
        let mut reference_const = vec![0u8; len];
        super::reference::allen_codes_const(&a_s, &a_e, c_s, c_e, &mut reference_const);
        for i in 0..len {
            let a = crate::interval::Interval::<u64>::new(a_s[i], a_e[i]).expect("nonempty");
            assert_eq!(reference_const[i], crate::allen::classify(a, c) as u8);
        }
    }
}

/// `allen_filter_batch` (codes + broadcast mask → keep bytes) is
/// bit-identical to the scalar reference across the 13 singletons, the
/// workload composites, and randomized masks — and its keep byte equals
/// `mask.contains(classify(...))` per pair.
#[test]
fn allen_filter_batch_matches_reference_across_masks() {
    use crate::allen::{AllenMask, Basic};
    let mut rng = Lcg(0x13F1);
    let mut masks: Vec<AllenMask> = Basic::ALL
        .iter()
        .map(|b| AllenMask::new(b.bit()).expect("singleton"))
        .collect();
    masks.extend([
        AllenMask::INTERSECTS,
        AllenMask::COVERS,
        AllenMask::DISJOINT,
        AllenMask::EMPTY,
        AllenMask::FULL,
    ]);
    for _ in 0..16 {
        masks.push(AllenMask::new((rng.next() & 0x1FFF) as u16).expect("13-bit"));
    }
    for &len in ALLEN_LENGTHS {
        let (a_s, a_e, b_s, b_e) = allen_corpus(len, &mut rng);
        let mut codes = Vec::new();
        allen_code_batch(&a_s, &a_e, &b_s, &b_e, &mut codes);
        for &mask in &masks {
            let mut kernel = Vec::new();
            allen_filter_batch(&codes, mask, &mut kernel);
            let mut reference = vec![0u8; len];
            super::reference::allen_keep(&codes, mask.bits(), &mut reference);
            assert_eq!(
                kernel,
                reference,
                "keep len {len} mask {:#06x}",
                mask.bits()
            );
            for i in 0..len {
                let a = crate::interval::Interval::<u64>::new(a_s[i], a_e[i]).expect("nonempty");
                let b = crate::interval::Interval::<u64>::new(b_s[i], b_e[i]).expect("nonempty");
                assert_eq!(
                    kernel[i] != 0,
                    mask.contains(crate::allen::classify(a, b)),
                    "membership at {i} of len {len} mask {:#06x}",
                    mask.bits(),
                );
            }
        }
    }
}

/// The dense filter-position compositions (`allen_filter_columns` and
/// its constant-operand form) produce exactly the scalar
/// classify-and-test survivor positions, ascending, across boundary
/// lengths and masks — including chunk-boundary lengths around the
/// stack chunk width.
#[test]
fn allen_filter_columns_match_the_scalar_survivors_bit_for_bit() {
    use crate::allen::AllenMask;
    let mut rng = Lcg(0xC01);
    let masks = [
        AllenMask::INTERSECTS,
        AllenMask::COVERS,
        AllenMask::DISJOINT,
        AllenMask::EQUALS,
        AllenMask::new(0x0AAA).expect("13-bit"),
    ];
    for &len in &[0usize, 1, 7, 8, 9, 16, 17, 255, 256, 257, 300] {
        let (a_s, a_e, b_s, b_e) = allen_corpus(len, &mut rng);
        for &mask in &masks {
            let naive = |x_s: u64, x_e: u64, y_s: u64, y_e: u64| {
                mask.contains(crate::allen::classify(
                    crate::interval::Interval::<u64>::new(x_s, x_e).expect("nonempty"),
                    crate::interval::Interval::<u64>::new(y_s, y_e).expect("nonempty"),
                ))
            };
            let mut kernel = Vec::new();
            allen_filter_columns(&a_s, &a_e, &b_s, &b_e, mask, &mut kernel);
            let expected: Vec<u32> = (0..len)
                .filter(|&i| naive(a_s[i], a_e[i], b_s[i], b_e[i]))
                .map(|i| u32::try_from(i).expect("small"))
                .collect();
            assert_eq!(
                kernel,
                expected,
                "columns len {len} mask {:#06x}",
                mask.bits()
            );

            let (c_s, c_e) = (3u64, 9u64);
            let mut kernel_const = Vec::new();
            allen_filter_columns_const(&a_s, &a_e, c_s, c_e, mask, &mut kernel_const);
            let expected_const: Vec<u32> = (0..len)
                .filter(|&i| naive(a_s[i], a_e[i], c_s, c_e))
                .map(|i| u32::try_from(i).expect("small"))
                .collect();
            assert_eq!(
                kernel_const,
                expected_const,
                "columns-const len {len} mask {:#06x}",
                mask.bits()
            );
        }
    }
}

/// All ordered pairs of nonempty intervals over an endpoint set: the
/// configuration-class corpus of the exhaustive mask suite.
fn interval_pairs_over(points: &[u64]) -> (Vec<u64>, Vec<u64>, Vec<u64>, Vec<u64>) {
    let mut intervals = Vec::new();
    for (i, &s) in points.iter().enumerate() {
        for &e in &points[i + 1..] {
            intervals.push((s, e));
        }
    }
    let (mut a_s, mut a_e, mut b_s, mut b_e) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for &(x_s, x_e) in &intervals {
        for &(y_s, y_e) in &intervals {
            a_s.push(x_s);
            a_e.push(x_e);
            b_s.push(y_s);
            b_e.push(y_e);
        }
    }
    (a_s, a_e, b_s, b_e)
}

/// The scalar classifier's code per pair — the oracle column of the
/// exhaustive suite.
fn scalar_codes(a_s: &[u64], a_e: &[u64], b_s: &[u64], b_e: &[u64]) -> Vec<u8> {
    (0..a_s.len())
        .map(|i| {
            let a = crate::interval::Interval::<u64>::new(a_s[i], a_e[i]).expect("nonempty");
            let b = crate::interval::Interval::<u64>::new(b_s[i], b_e[i]).expect("nonempty");
            crate::allen::classify(a, b) as u8
        })
        .collect()
}

/// Exhaustive: EVERY Allen mask × EVERY interval configuration class —
/// the vectorized configuration kernel pipeline (code batch, then the
/// broadcast-mask filter batch) agrees with the scalar classifier on
/// every cell (the crucible packet (git ecec1dc3)).
///
/// Domain arithmetic — both axes are total, not sampled:
///   masks: a mask is a 13-bit word, so the space is 2¹³ = 8,192 masks;
///     the loop bound is counted and asserted below.
///   pairs: all ordered pairs of nonempty intervals over the 8-value
///     endpoint set {0, 1, 2, 3, 4, MAX−2, MAX−1, MAX} — C(8,2) = 28
///     intervals (rays included: end == MAX is the point-domain law),
///     28² = 784 pairs. A pair has 4 endpoints, so any configuration
///     class (endpoint order type with ties) needs at most 4 distinct
///     values; the 8-value set realizes every class, at both the small
///     and the unsigned-extreme end — all 13 basics occur (asserted).
/// Cells: 8,192 × 784 = 6,422,528, every one checked.
#[test]
fn exhaustive_all_8192_masks_times_all_configuration_classes() {
    use crate::allen::AllenMask;
    const MAX: u64 = u64::MAX;
    let points = [0u64, 1, 2, 3, 4, MAX - 2, MAX - 1, MAX];
    let (a_s, a_e, b_s, b_e) = interval_pairs_over(&points);
    assert_eq!(a_s.len(), 784, "C(8,2)² = 28² ordered pairs");

    // The vectorized code kernel vs the scalar classifier, per pair.
    let mut codes = Vec::new();
    allen_code_batch(&a_s, &a_e, &b_s, &b_e, &mut codes);
    let scalar = scalar_codes(&a_s, &a_e, &b_s, &b_e);
    assert_eq!(codes, scalar, "configuration codes match the classifier");
    // Every configuration class is present in the corpus.
    for basic in crate::allen::Basic::ALL {
        assert!(
            scalar.contains(&(basic as u8)),
            "class {basic:?} missing from the corpus"
        );
    }

    let mut visited = 0u32;
    let mut keep = Vec::new();
    for bits in 0..=0x1FFF_u16 {
        let mask = AllenMask::new(bits).expect("13-bit range");
        allen_filter_batch(&codes, mask, &mut keep);
        for (i, &code) in scalar.iter().enumerate() {
            assert_eq!(
                keep[i] != 0,
                mask.bits() & (1 << code) != 0,
                "cell (mask {bits:#06x}, pair {i})"
            );
        }
        visited += 1;
    }
    assert_eq!(visited, 8_192, "the full 2^13 mask space was enumerated");
}

/// The Miri-lane representative of the exhaustive mask suite: the same
/// pipeline over the 4-value endpoint grid (C(4,2) = 6 intervals, 36
/// pairs) × the 13 singletons, the workload composites, and 16 stride-
/// sampled masks. Runs everywhere the dispatch is interpretable (the
/// scalar/portable twins under Miri; the NEON path natively).
#[test]
fn allen_representative_masks_agree_with_the_scalar_classifier() {
    use crate::allen::{AllenMask, Basic};
    let points = [0u64, 1, 3, u64::MAX];
    let (a_s, a_e, b_s, b_e) = interval_pairs_over(&points);
    assert_eq!(a_s.len(), 36);
    let mut codes = Vec::new();
    allen_code_batch(&a_s, &a_e, &b_s, &b_e, &mut codes);
    let scalar = scalar_codes(&a_s, &a_e, &b_s, &b_e);
    assert_eq!(codes, scalar);

    let mut masks: Vec<AllenMask> = Basic::ALL
        .iter()
        .map(|b| AllenMask::new(b.bit()).expect("singleton"))
        .collect();
    masks.extend([
        AllenMask::INTERSECTS,
        AllenMask::COVERS,
        AllenMask::COVERED_BY,
        AllenMask::DISJOINT,
        AllenMask::EMPTY,
        AllenMask::FULL,
    ]);
    masks.extend((0..16).map(|i| AllenMask::new(i * 0x0111).expect("13-bit")));
    let mut keep = Vec::new();
    for mask in masks {
        allen_filter_batch(&codes, mask, &mut keep);
        for (i, &code) in scalar.iter().enumerate() {
            assert_eq!(keep[i] != 0, mask.bits() & (1 << code) != 0);
        }
    }
}

#[test]
fn compaction_keeps_exactly_the_masked_items_in_order() {
    // Empty and full survivor sets, plus a mixed mask.
    let mut items: Vec<u32> = (0..10).collect();
    compact_u32_by_mask(&mut items, &[0; 10]);
    assert!(items.is_empty());

    let mut items: Vec<u32> = (0..10).collect();
    compact_u32_by_mask(&mut items, &[1; 10]);
    assert_eq!(items, (0..10).collect::<Vec<u32>>());

    let mut items: Vec<u32> = (0..10).collect();
    let mask = [1u8, 0, 1, 0, 0, 1, 1, 0, 0, 1];
    compact_u32_by_mask(&mut items, &mask);
    assert_eq!(items, vec![0, 2, 5, 6, 9]);
}
