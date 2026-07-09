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
/// multiples +/- 1.
const LENGTHS: &[usize] = &[0, 1, 2, 3, 15, 16, 17, 31, 32, 33, 100, 257];

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

/// PRD 17 (the 00-product unsafe policy): the interval filter
/// compositions — `PointIn`, `AnyPointIn`, and the three Overlaps/Contains
/// shapes — are bit-identical to the scalar reference across the
/// boundary shapes: empty, single, odd lengths, lane ±1.
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
        for (c_start, c_end) in [(0u64, 1u64), (1, 4), (2, 7), (0, u64::MAX), (5, 6)] {
            let (mut kernel, mut reference) = (Vec::new(), Vec::new());
            filter_overlaps_u64(&starts, &ends, c_start, c_end, &mut kernel);
            super::reference::filter_overlaps_u64(&starts, &ends, c_start, c_end, &mut reference);
            assert_eq!(kernel, reference, "overlaps len {len} [{c_start},{c_end})");

            let (mut kernel, mut reference) = (Vec::new(), Vec::new());
            filter_contains_u64(&starts, &ends, c_start, c_end, &mut kernel);
            super::reference::filter_contains_u64(&starts, &ends, c_start, c_end, &mut reference);
            assert_eq!(kernel, reference, "contains len {len} [{c_start},{c_end})");

            let (mut kernel, mut reference) = (Vec::new(), Vec::new());
            filter_within_u64(&starts, &ends, c_start, c_end, &mut kernel);
            super::reference::filter_within_u64(&starts, &ends, c_start, c_end, &mut reference);
            assert_eq!(kernel, reference, "within len {len} [{c_start},{c_end})");
        }
    }
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

/// PRD 03 (docs/perf/): the fold kernels are bit-identical to naive
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

/// Fold-throughput evidence (docs/silicon/06 gate; ignored: a timing
/// test runs only by hand —
/// `cargo test -p bumbledb --release fold_throughput -- --ignored --nocapture`).
/// The gates: ≥ 7 rows/ns exact dense sums on the reference host
/// (bumblebench measured the kernel ceiling at 8.8; scalar-era
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
        #[allow(clippy::cast_precision_loss)] // both far below 2^52
        let rate = (values.len() as u64 * reps) as f64
            / u64::try_from(elapsed.as_nanos().max(1)).expect("short run") as f64;
        println!("{label}: {rate:.2} rows/ns (sink {sink})");
        rate
    };
    let biased = rate_of("fold_sum_biased_i64 dense", &mut || {
        fold_sum_biased_i64(&values, 1, 0, values.len())
    });
    let unsigned = rate_of("fold_sum_u64 dense", &mut || {
        #[allow(clippy::cast_possible_wrap)]
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
