use super::*;

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

const LENGTHS: &[usize] = &[
    0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 257, 1023, 4099,
];

/// Runs `kernel` and `twin` into fresh outputs that both start with one
/// sentinel position, so appending (never overwriting) is checked too.
fn same_survivors(kernel: impl Fn(&mut Vec<u32>), twin: impl Fn(&mut Vec<u32>)) {
    let (mut got, mut want) = (vec![u32::MAX], vec![u32::MAX]);
    kernel(&mut got);
    twin(&mut want);
    assert_eq!(got, want);
}

#[test]
fn u64_filters_match_their_twins_at_every_level() {
    let mut rng = Lcg(42);
    for level in every_level() {
        for &len in LENGTHS {
            let col: Vec<u64> = (0..len)
                .map(|_| match rng.next() % 8 {
                    0 => 0,
                    1 => u64::MAX,
                    n => n % 4,
                })
                .collect();
            for needle in [0u64, 1, 2, 3, u64::MAX] {
                same_survivors(
                    |out| filter::eq_u64(level, &col, needle, out),
                    |out| reference::filter_eq_u64(&col, needle, out),
                );
            }
            for (lo, hi) in [(0u64, 2u64), (1, 1), (3, u64::MAX), (u64::MAX, 0)] {
                same_survivors(
                    |out| filter::range_u64(level, &col, lo, hi, out),
                    |out| reference::filter_range_u64(&col, lo, hi, out),
                );
            }
        }
    }
}

#[test]
fn u8_filter_matches_its_twin_at_every_level() {
    let mut rng = Lcg(7);
    for level in every_level() {
        for &len in LENGTHS {
            let col: Vec<u8> = (0..len)
                .map(|_| u8::try_from(rng.next() % 3).expect("small"))
                .collect();
            for needle in [0u8, 1, 2, 255] {
                same_survivors(
                    |out| filter::eq_u8(level, &col, needle, out),
                    |out| reference::filter_eq_u8(&col, needle, out),
                );
            }
        }
    }
}

#[test]
fn interval_filters_match_their_twins_at_every_level() {
    let mut rng = Lcg(1717);
    for level in every_level() {
        for &len in LENGTHS {
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
                same_survivors(
                    |out| filter::point_in_u64(level, &starts, &ends, point, out),
                    |out| reference::filter_point_in_u64(&starts, &ends, point, out),
                );
            }
            for points in [&[][..], &[3][..], &[0, 4][..], &[1, 2, 5, u64::MAX - 1][..]] {
                same_survivors(
                    |out| filter::any_point_in_u64(level, &starts, &ends, points, out),
                    |out| reference::filter_any_point_in_u64(&starts, &ends, points, out),
                );
            }
        }
    }
}

#[test]
fn point_in_is_half_open_at_both_boundaries() {
    let starts = [10u64; 9];
    let ends = [20u64; 9];
    let mut out = Vec::new();
    filter_point_in_u64(&starts, &ends, 10, &mut out);
    assert_eq!(out, (0..9).collect::<Vec<u32>>(), "p == start is in");
    out.clear();
    filter_point_in_u64(&starts, &ends, 20, &mut out);
    assert!(out.is_empty(), "p == end is out");
}

#[test]
#[should_panic(expected = "an interval column pair")]
fn interval_filters_refuse_unequal_columns() {
    let mut out = Vec::new();
    filter_point_in_u64(&[1, 2, 3], &[4, 5], 2, &mut out);
}

fn fold_words(rng: &mut Lcg, len: usize) -> Vec<u64> {
    (0..len)
        .map(|_| match rng.next() % 6 {
            0 => 0,
            1 => u64::MAX,
            2 => 1 << 63,
            3 => (1 << 63) - 1,
            _ => rng.next(),
        })
        .collect()
}

#[test]
fn folds_match_their_twins_at_every_level() {
    let mut rng = Lcg(99);
    for level in every_level() {
        for &len in LENGTHS {
            for stride in [1usize, 2, 3, 5] {
                for offset in [0usize, 1] {
                    let values = fold_words(&mut rng, len * stride + offset + 1);
                    let mut indices: Vec<u32> =
                        (0..len).map(|i| u32::try_from(i).expect("small")).collect();
                    indices.reverse();
                    if len > 2 {
                        indices.extend([1, 1]);
                    }
                    let context = format!("{} len {len} stride {stride}", level_name(level));
                    assert_eq!(
                        fold::sum_u64(level, &values, stride, offset, len),
                        reference::fold_sum_u64(&values, stride, offset, len),
                        "{context}"
                    );
                    assert_eq!(
                        gather::sum_u64_idx(level, &values, stride, offset, &indices),
                        reference::fold_sum_u64_idx(&values, stride, offset, &indices),
                        "{context}"
                    );
                    if len > 0 {
                        assert_eq!(
                            fold::min_max_u64(level, &values, stride, offset, len),
                            reference::fold_min_max_u64(&values, stride, offset, len),
                            "{context}"
                        );
                        assert_eq!(
                            gather::min_max_u64_idx(level, &values, stride, offset, &indices),
                            reference::fold_min_max_u64_idx(&values, stride, offset, &indices),
                            "{context}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn sums_carry_across_every_lane_and_tail_at_every_level() {
    let values = vec![u64::MAX; 1040];
    let mut indices: Vec<u32> = (0..1024).collect();
    indices.extend(std::iter::repeat_n(7u32, 9));
    for level in every_level() {
        for len in [
            0, 1, 3, 4, 5, 7, 8, 9, 255, 256, 257, 1023, 1024, 1025, 1033,
        ] {
            let expected = u128::from(u64::MAX) * len as u128;
            assert_eq!(
                gather::sum_u64_idx(level, &values, 1, 0, &indices[..len]),
                expected
            );
            assert_eq!(fold::sum_u64(level, &values, 1, 0, len), expected);
        }
    }
}

#[test]
#[should_panic(expected = "assertion failed")]
fn fold_extent_guard_refuses_wrapping_extents() {
    let _ = fold_sum_u64(&[0u64; 1], 1usize << 63, 0, 3);
}

#[test]
#[should_panic(expected = "assertion failed")]
fn fold_min_max_extent_guard_refuses_wrapping_extents() {
    let _ = fold_min_max_u64(&[0u64; 1], 1usize << 63, 0, 3);
}

#[test]
#[should_panic(expected = "gathered index out of bounds")]
fn gathered_sum_refuses_an_out_of_bounds_lane() {
    let values = [1u64, 2, 3, 4, 5, 6, 7, 8];
    let _ = fold_sum_u64_idx(&values, 1, 0, &[0, 1, 8, 2]);
}

#[test]
#[should_panic(expected = "gathered index out of bounds")]
fn gathered_min_max_refuses_an_out_of_bounds_lane() {
    let values = [1u64, 2, 3, 4, 5, 6, 7, 8];
    let _ = fold_min_max_u64_idx(&values, 2, 1, &[0, 1, 4, 2]);
}

#[test]
#[should_panic(expected = "gathered index out of bounds")]
fn gathered_address_overflow_is_out_of_bounds() {
    let values = [1u64, 2, 3, 4];
    let _ = fold_sum_u64_idx(&values, usize::MAX / 2 + 1, 0, &[0, 0, 2, 0]);
}

fn allen_corpus(len: usize, rng: &mut Lcg) -> (Vec<u64>, Vec<u64>, Vec<u64>, Vec<u64>) {
    const MAX: u64 = u64::MAX;
    let named: &[(u64, u64, u64, u64)] = &[
        (0, 5, 5, 9),
        (5, 9, 0, 5),
        (0, 10, 3, 7),
        (3, 7, 0, 10),
        (2, 6, 2, 6),
        (3, MAX, 7, MAX),
        (0, 5, 5, MAX),
        (2, MAX, 2, 6),
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
                    0 => (s, MAX),
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

const ALLEN_LENGTHS: &[usize] = &[0, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 32, 33, 100, 257];

/// The unsafe-allowlist law for the configuration kernel: `allen_code_batch` is
/// bit-identical to the scalar reference AND to `classify` (the reference is
/// the decision tree; the kernel is the signature table — the test cross-checks
/// table against tree) across randomized inputs including every boundary shape:
/// adjacent, nested, equal, rays, lane-multiple ±1 lengths.
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
            let a = bumbledb_theory::Interval::<u64>::new(a_s[i], a_e[i]).expect("nonempty");
            let b = bumbledb_theory::Interval::<u64>::new(b_s[i], b_e[i]).expect("nonempty");
            assert_eq!(
                kernel[i],
                crate::allen::classify(a, b) as u8,
                "classify at {i} of len {len}: {:?} vs {:?}",
                (a_s[i], a_e[i]),
                (b_s[i], b_e[i]),
            );
        }

        // dispatch reader is the non-aarch64 build; here it is oracle-

        let (c_s, c_e) = (3u64, 9u64);
        let c = bumbledb_theory::Interval::<u64>::new(c_s, c_e).expect("nonempty");
        let mut reference_const = vec![0u8; len];
        super::reference::allen_codes_const(&a_s, &a_e, c_s, c_e, &mut reference_const);
        for i in 0..len {
            let a = bumbledb_theory::Interval::<u64>::new(a_s[i], a_e[i]).expect("nonempty");
            assert_eq!(reference_const[i], crate::allen::classify(a, c) as u8);
        }
    }
}

#[test]
fn allen_code_batch_const_matches_the_broadcast_four_stream_kernel() {
    let mut rng = Lcg(0xC0DE);
    for &len in ALLEN_LENGTHS {
        let (a_s, a_e, _, _) = allen_corpus(len, &mut rng);
        for &(c_s, c_e) in &[(3u64, 9u64), (0, u64::MAX), (5, 6)] {
            let mut kernel_const = Vec::new();
            allen_code_batch_const(&a_s, &a_e, c_s, c_e, &mut kernel_const);
            let mut broadcast = Vec::new();
            allen_code_batch(&a_s, &a_e, &vec![c_s; len], &vec![c_e; len], &mut broadcast);
            assert_eq!(kernel_const, broadcast, "const codes len {len}");
            let c = bumbledb_theory::Interval::<u64>::new(c_s, c_e).expect("nonempty");
            for i in 0..len {
                let a = bumbledb_theory::Interval::<u64>::new(a_s[i], a_e[i]).expect("nonempty");
                assert_eq!(kernel_const[i], crate::allen::classify(a, c) as u8);
            }
        }
    }
}

#[test]
fn allen_filter_batch_matches_reference_across_masks() {
    use bumbledb_theory::allen::{AllenMask, Basic};
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
                let a = bumbledb_theory::Interval::<u64>::new(a_s[i], a_e[i]).expect("nonempty");
                let b = bumbledb_theory::Interval::<u64>::new(b_s[i], b_e[i]).expect("nonempty");
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

#[test]
fn allen_filter_columns_match_the_scalar_survivors_bit_for_bit() {
    use bumbledb_theory::allen::AllenMask;
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
                    bumbledb_theory::Interval::<u64>::new(x_s, x_e).expect("nonempty"),
                    bumbledb_theory::Interval::<u64>::new(y_s, y_e).expect("nonempty"),
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

/// The four-stream length equality is release-strength at the dispatch (the
/// NEON core reads 8-word windows through raw pointers from every stream, so
/// the equality is the memory-safety invariant — asserted like the fold
/// kernels' extent guard, not debug-only).
#[test]
#[should_panic(expected = "four equal-length endpoint streams")]
fn allen_code_batch_refuses_unequal_streams() {
    let long = vec![1u64; 32];
    let short = vec![2u64; 24];
    let ends: Vec<u64> = long.iter().map(|&s| s + 4).collect();
    let mut codes = Vec::new();
    allen_code_batch(&long, &ends, &short, &ends, &mut codes);
}

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

fn scalar_codes(a_s: &[u64], a_e: &[u64], b_s: &[u64], b_e: &[u64]) -> Vec<u8> {
    (0..a_s.len())
        .map(|i| {
            let a = bumbledb_theory::Interval::<u64>::new(a_s[i], a_e[i]).expect("nonempty");
            let b = bumbledb_theory::Interval::<u64>::new(b_s[i], b_e[i]).expect("nonempty");
            crate::allen::classify(a, b) as u8
        })
        .collect()
}

#[test]
fn exhaustive_all_8192_masks_times_all_configuration_classes() {
    use bumbledb_theory::allen::AllenMask;
    const MAX: u64 = u64::MAX;
    let points = [0u64, 1, 2, 3, 4, MAX - 2, MAX - 1, MAX];
    let (a_s, a_e, b_s, b_e) = interval_pairs_over(&points);
    assert_eq!(a_s.len(), 784, "C(8,2)² = 28² ordered pairs");

    let mut codes = Vec::new();
    allen_code_batch(&a_s, &a_e, &b_s, &b_e, &mut codes);
    let scalar = scalar_codes(&a_s, &a_e, &b_s, &b_e);
    assert_eq!(codes, scalar, "configuration codes match the classifier");

    for basic in bumbledb_theory::allen::Basic::ALL {
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

#[test]
fn allen_representative_masks_agree_with_the_scalar_classifier() {
    use bumbledb_theory::allen::{AllenMask, Basic};
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
fn allen_pooled_reuse_is_bit_identical_to_fresh_outputs() {
    use bumbledb_theory::allen::AllenMask;

    const REUSE_LADDER: &[usize] = &[257, 33, 17, 16, 15, 9, 8, 7, 3, 1, 0, 2, 12, 20, 100];
    let mut rng = Lcg(0x5EED_A11E);
    let mut pooled_codes = Vec::new();
    let mut pooled_keep = Vec::new();

    pooled_codes.resize(300, 0xEE);
    pooled_keep.resize(300, 0xEE);
    let masks = [
        AllenMask::INTERSECTS,
        AllenMask::COVERS,
        AllenMask::DISJOINT,
        AllenMask::FULL,
    ];
    for (round, &len) in REUSE_LADDER.iter().enumerate() {
        let (a_s, a_e, b_s, b_e) = allen_corpus(len, &mut rng);
        allen_code_batch(&a_s, &a_e, &b_s, &b_e, &mut pooled_codes);
        let mut fresh_codes = Vec::new();
        allen_code_batch(&a_s, &a_e, &b_s, &b_e, &mut fresh_codes);
        assert_eq!(
            pooled_codes, fresh_codes,
            "codes diverge at round {round} len {len}"
        );
        assert_eq!(pooled_codes.len(), len, "codes resize to the pair count");
        let mask = masks[round % masks.len()];
        allen_filter_batch(&pooled_codes, mask, &mut pooled_keep);
        let mut fresh_keep = Vec::new();
        allen_filter_batch(&fresh_codes, mask, &mut fresh_keep);
        assert_eq!(
            pooled_keep,
            fresh_keep,
            "keep diverges at round {round} len {len} mask {:#06x}",
            mask.bits()
        );
        assert_eq!(pooled_keep.len(), len, "keep resizes to the code count");

        assert!(pooled_keep.iter().all(|&k| k <= 1));
        assert!(pooled_codes.iter().all(|&c| c < 13));
    }
}

#[test]
fn compaction_keeps_exactly_the_masked_items_in_order() {
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

/// The unsafe-allowlist law for the compaction kernel: bit-identical to the
/// fully safe-indexed reference across randomized 0/1 masks (the producers'
/// contract) at every selectivity, plus the degenerate shapes — all-zero,
/// all-one, alternating — over the lane-stress lengths and a long tail.
#[test]
fn compaction_matches_the_safe_reference_bit_for_bit() {
    let mut rng = Lcg(0xC0);
    let lengths = LENGTHS.iter().copied().chain([1_000, 8_192]);
    for len in lengths {
        let source: Vec<u32> = (0..len)
            .map(|_| u32::try_from(rng.next() % u64::from(u32::MAX)).expect("bounded"))
            .collect();
        let mut masks: Vec<Vec<u8>> = vec![
            vec![0u8; len],
            vec![1u8; len],
            (0..len).map(|i| u8::from(i % 2 == 0)).collect(),
        ];
        for percent in [1, 10, 50, 90, 99] {
            masks.push(
                (0..len)
                    .map(|_| u8::from(rng.next() % 100 < percent))
                    .collect(),
            );
        }
        for mut mask in masks {
            let mut kernel = source.clone();
            let mut reference = source.clone();
            compact_u32_by_mask(&mut kernel, &mask);
            super::reference::compact_u32_by_mask(&mut reference, &mask);
            assert_eq!(kernel, reference, "len {len}");

            mask.push(1);
            let mut kernel_long = source.clone();
            compact_u32_by_mask(&mut kernel_long, &mask);
            assert_eq!(kernel_long, kernel, "longer mask, len {len}");
        }
    }
}
