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
#[cfg_attr(miri, ignore)]
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
#[cfg_attr(miri, ignore)]
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
#[cfg_attr(miri, ignore)]
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

fn interval(start: u64, end: u64) -> bumbledb_theory::Interval<u64> {
    bumbledb_theory::Interval::new(start, end).expect("nonempty")
}

fn masks_under_test(rng: &mut Lcg) -> Vec<AllenMask> {
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
    masks.extend((0..16).map(|_| AllenMask::new((rng.next() & 0x1FFF) as u16).expect("13-bit")));
    masks
}

use bumbledb_theory::allen::{AllenMask, Basic};

/// Codes equal the `classify` decision tree at every level, so the signature
/// tables (portable hash and NEON `tbl`) are cross-checked against the tree.
#[test]
fn allen_codes_match_the_classifier_at_every_level() {
    let mut rng = Lcg(0xA11E);
    for level in every_level() {
        for &len in ALLEN_LENGTHS {
            let (a_s, a_e, b_s, b_e) = allen_corpus(len, &mut rng);
            let mut codes = Vec::new();
            allen::code_batch(level, &a_s, &a_e, &b_s, &b_e, &mut codes);
            let mut twin = vec![0u8; len];
            reference::allen_codes(&a_s, &a_e, &b_s, &b_e, &mut twin);
            assert_eq!(codes, twin, "{} len {len}", level_name(level));
            for (c_s, c_e) in [(3u64, 9u64), (0, u64::MAX), (5, 6)] {
                allen::code_batch_const(level, &a_s, &a_e, c_s, c_e, &mut codes);
                reference::allen_codes_const(&a_s, &a_e, c_s, c_e, &mut twin);
                assert_eq!(codes, twin, "{} const len {len}", level_name(level));
            }
        }
    }
}

#[test]
fn allen_keep_bytes_match_their_twin_at_every_level() {
    let mut rng = Lcg(0x13F1);
    let masks = masks_under_test(&mut rng);
    for level in every_level() {
        for &len in ALLEN_LENGTHS {
            let (a_s, a_e, b_s, b_e) = allen_corpus(len, &mut rng);
            let mut codes = Vec::new();
            reference::allen_codes(&a_s, &a_e, &b_s, &b_e, {
                codes.resize(len, 0);
                &mut codes
            });
            for &mask in &masks {
                let mut keep = Vec::new();
                allen::filter_batch(level, &codes, mask, &mut keep);
                let mut twin = vec![0u8; len];
                reference::allen_keep(&codes, mask.bits(), &mut twin);
                assert_eq!(
                    keep,
                    twin,
                    "{} len {len} mask {:#06x}",
                    level_name(level),
                    mask.bits()
                );
            }
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn allen_column_filters_match_the_classifier_at_every_level() {
    let mut rng = Lcg(0xC01);
    let masks = masks_under_test(&mut rng);
    for level in every_level() {
        for &len in &[0usize, 1, 7, 8, 9, 16, 17, 255, 256, 257, 300] {
            let (a_s, a_e, b_s, b_e) = allen_corpus(len, &mut rng);
            for &mask in &masks {
                let keeps = |b: &dyn Fn(usize) -> (u64, u64)| -> Vec<u32> {
                    let mut out = vec![u32::MAX];
                    out.extend((0..len).filter_map(|i| {
                        let (y_s, y_e) = b(i);
                        mask.contains(crate::allen::classify(
                            interval(a_s[i], a_e[i]),
                            interval(y_s, y_e),
                        ))
                        .then(|| u32::try_from(i).expect("small"))
                    }));
                    out
                };
                let mut got = vec![u32::MAX];
                allen::filter_columns(level, &a_s, &a_e, &b_s, &b_e, mask, &mut got);
                assert_eq!(
                    got,
                    keeps(&|i| (b_s[i], b_e[i])),
                    "{} len {len}",
                    level_name(level)
                );
                let mut got = vec![u32::MAX];
                allen::filter_columns_const(level, &a_s, &a_e, 3, 9, mask, &mut got);
                assert_eq!(
                    got,
                    keeps(&|_| (3, 9)),
                    "{} const len {len}",
                    level_name(level)
                );
            }
        }
    }
}

#[test]
#[should_panic(expected = "four equal-length endpoint streams")]
fn allen_code_batch_refuses_unequal_streams() {
    let long = vec![1u64; 32];
    let short = vec![2u64; 24];
    let ends: Vec<u64> = long.iter().map(|&s| s + 4).collect();
    let mut codes = Vec::new();
    allen_code_batch(&long, &ends, &short, &ends, &mut codes);
}

#[test]
#[should_panic(expected = "four equal-length endpoint streams")]
fn allen_column_filter_refuses_unequal_columns() {
    let mut out = Vec::new();
    allen_filter_columns(&[1, 2], &[3, 4], &[1], &[2], AllenMask::FULL, &mut out);
}

/// Every ordered pair of intervals over eight points near both ends of the
/// word range: all 13 relations, each against all 8192 masks, at every level.
#[test]
#[cfg_attr(miri, ignore)]
fn exhaustive_masks_times_configuration_classes_at_every_level() {
    const MAX: u64 = u64::MAX;
    let points = [0u64, 1, 2, 3, 4, MAX - 2, MAX - 1, MAX];
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
    assert_eq!(a_s.len(), 784, "C(8,2)^2 ordered pairs");
    let tree: Vec<u8> = (0..a_s.len())
        .map(|i| crate::allen::classify(interval(a_s[i], a_e[i]), interval(b_s[i], b_e[i])) as u8)
        .collect();
    for basic in Basic::ALL {
        assert!(
            tree.contains(&(basic as u8)),
            "{basic:?} missing from the corpus"
        );
    }
    for level in every_level() {
        let mut codes = Vec::new();
        allen::code_batch(level, &a_s, &a_e, &b_s, &b_e, &mut codes);
        assert_eq!(codes, tree, "{}", level_name(level));
        let mut keep = Vec::new();
        for bits in 0..=0x1FFF_u16 {
            allen::filter_batch(
                level,
                &codes,
                AllenMask::new(bits).expect("13-bit"),
                &mut keep,
            );
            for (i, &code) in tree.iter().enumerate() {
                assert_eq!(
                    keep[i] != 0,
                    bits & (1 << code) != 0,
                    "mask {bits:#06x} pair {i}"
                );
            }
        }
    }
}

/// Pooled outputs shrink and grow across batches; every retained byte is
/// rewritten, so results equal fresh outputs.
#[test]
fn allen_pooled_outputs_equal_fresh_outputs() {
    const REUSE_LADDER: &[usize] = &[257, 33, 17, 16, 15, 9, 8, 7, 3, 1, 0, 2, 12, 20, 100];
    let mut rng = Lcg(0x5EED_A11E);
    let mut pooled_codes = vec![0xEE; 300];
    let mut pooled_keep = vec![0xEE; 300];
    for (round, &len) in REUSE_LADDER.iter().enumerate() {
        let (a_s, a_e, b_s, b_e) = allen_corpus(len, &mut rng);
        allen_code_batch(&a_s, &a_e, &b_s, &b_e, &mut pooled_codes);
        let mut fresh_codes = Vec::new();
        allen_code_batch(&a_s, &a_e, &b_s, &b_e, &mut fresh_codes);
        assert_eq!(pooled_codes, fresh_codes, "round {round}");
        let mask = AllenMask::new(0x0AAA).expect("13-bit");
        allen_filter_batch(&pooled_codes, mask, &mut pooled_keep);
        let mut fresh_keep = Vec::new();
        allen_filter_batch(&fresh_codes, mask, &mut fresh_keep);
        assert_eq!(pooled_keep, fresh_keep, "round {round}");
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn compaction_matches_its_twin_at_every_level() {
    let mut rng = Lcg(0xC0);
    for level in every_level() {
        for len in LENGTHS.iter().copied().chain([1_000, 8_192]) {
            let source: Vec<u32> = (0..len)
                .map(|_| u32::try_from(rng.next() >> 32).expect("high half"))
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
                let mut got = source.clone();
                let mut twin = source.clone();
                compact::compact(level, &mut got, &mask);
                reference::compact_u32_by_mask(&mut twin, &mask);
                assert_eq!(got, twin, "{} len {len}", level_name(level));
                mask.push(1);
                let mut longer = source.clone();
                compact::compact(level, &mut longer, &mask);
                assert_eq!(longer, twin, "{} longer mask len {len}", level_name(level));
            }
        }
    }
}

#[test]
#[should_panic(expected = "one keep byte per item")]
fn compaction_refuses_a_short_mask() {
    compact_u32_by_mask(&mut vec![1, 2, 3], &[1, 1]);
}
