use super::*;

/// The filter-mask decider twin (PRD-I3): test-local masked
/// `filter_eq_u64`, its masked scalar reference, the always-on
/// bit-identity pins, and the `#[ignore]`d interleaved timing twin.
mod filter_mask_twin;

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

/// The gather folds' overflow and sign edges, pinned closed-form:
/// carry saturation (every lane wave overflows every step), duplicate
/// indices on extreme words, and the biased extremes (`i64::MIN`
/// encodes as word 0, `i64::MAX` as `u64::MAX`).
#[test]
fn gather_folds_pin_the_overflow_and_sign_edges() {
    // Carry saturation: 1024 all-ones words + 9 duplicate revisits.
    let values = vec![u64::MAX; 1024];
    let mut indices: Vec<u32> = (0..1024).collect();
    indices.extend(std::iter::repeat_n(7u32, 9));
    let expected = u128::from(u64::MAX) * 1033;
    assert_eq!(fold_sum_u64_idx(&values, 1, 0, &indices), expected);
    // The same words read as biased i64 are all i64::MAX.
    assert_eq!(
        fold_sum_biased_i64_idx(&values, 1, 0, &indices),
        i128::from(i64::MAX) * 1033
    );
    assert_eq!(
        fold_min_max_u64_idx(&values, 1, 0, &indices),
        (u64::MAX, u64::MAX)
    );

    // Signed extremes mixed: word 0 = i64::MIN, word u64::MAX = i64::MAX,
    // word 1<<63 = 0 — an alternation whose naive i128 fold is the pin.
    let words = [0u64, u64::MAX, 1 << 63, 0, u64::MAX, 1 << 63, 0];
    let indices: Vec<u32> = (0..7).collect();
    let naive: i128 = words.iter().map(|&w| i128::from(biased_to_i64(w))).sum();
    assert_eq!(fold_sum_biased_i64_idx(&words, 1, 0, &indices), naive);
    assert_eq!(naive, 3 * i128::from(i64::MIN) + 2 * i128::from(i64::MAX));
    assert_eq!(fold_min_max_u64_idx(&words, 1, 0, &indices), (0, u64::MAX));
}

/// The gather-twin falsifier (timing evidence; ignored — run by hand
/// under the machine mutex:
/// `scripts/measure.sh cargo test -p bumbledb --release
///  gather_fold_scalar_addressed_twin -- --ignored --nocapture`).
///
/// THE GRAVESTONE (2026-07-16, the scalar-addressed gather twin). The
/// prediction: the shipped `Simd::gather_or_default` lowering — the
/// lane addresses live in vector registers, the bounds mask reduces
/// horizontally in-loop (`cmhi`/`uzp1`/`addv.4s`/`fmov`), and a 4-deep
/// `tbnz` ladder extracts each address back through `fmov` to feed
/// `ld1.d` lane loads (at runtime stride the indices additionally
/// enter by scalar `mul` + `fmov`, a full GPR→vector→GPR round trip)
/// — serializes lane issue and throttles the DRAM miss-lane budget
/// (m2max.mem.miss-lanes), so a scalar-addressed twin (addresses in
/// GPRs: `ldr w`/`madd`/direct loads, SIMD accumulation untouched)
/// should win 1.3–2x on DRAM-tier sparse survivors. The measurement
/// REFUTED it: at every DRAM shape every arm sits at the gather wall
/// (~4.3–5.5 ns/idx ≈ m2max.mem.gather-wall's 3.98 under co-tenant
/// ambient) — the `OoO` window spans enough waves that the ladder never
/// limits lane count. What remains is a µop-cost residue in the
/// 0.87–0.98 band (medians of 31 interleaved pairs, 2026-07-16) that
/// is NOT stable evidence: a second build of the same arms moved the
/// checked twin's displaced-s3 ratio from 0.97 to 0.87 (the
/// code-placement lottery), the engine inlines the kernel into both a
/// stride-1-specialized and a runtime-stride shape (no outlined
/// `_idx` symbol survives in the release binary), and against the
/// stride-1-specialized shape (`simd-index-spec-s1`, the scan-pushdown
/// sink's real codegen, itself 0.87–0.97 vs the runtime shape) the
/// checked twin is at parity-to-slight-loss. The unchecked twin
/// reached 0.82–0.90 — the one repeatable single-digit lead, but it
/// forfeits the crucible's deleted-`unsafe` ruling for a wall-bound
/// regime; recorded as a lead, not landed. The full-scalar control
/// confirms the crucible packet Q2: scalar `cmp`/`csel` min/max is
/// 1.62–1.75x SLOWER at DRAM tier — the flag-strand MLP halving
/// (m2max.core.flag-strand-mlp) in person. Q2's ADOPT stands. This
/// pin re-runs the experiment: if any challenger ever beats the
/// shipped kernel by the predicted margin at a displaced tier, the
/// gravestone is wrong and this test fails loudly.
#[test]
#[ignore = "timing evidence, run by hand on the reference host"]
#[expect(clippy::too_many_lines, reason = "a self-contained measurement rig")]
fn gather_fold_scalar_addressed_twin() {
    use std::fmt::Write as _;
    use std::simd::prelude::*;
    use std::time::Instant;

    type ArmFn<'x> = &'x dyn Fn(&[u64], &[u32]) -> u128;

    // --- The scalar-addressed challengers (the refuted twin). ---
    fn scalar_addressed_sum_w4(
        values: &[u64],
        stride: usize,
        offset: usize,
        indices: &[u32],
    ) -> u128 {
        let mut lows = Simd::<u64, 4>::splat(0);
        let mut carries = Simd::<u64, 4>::splat(0);
        let (chunks, tail) = indices.as_chunks::<4>();
        for chunk in chunks {
            let v = Simd::<u64, 4>::from_array([
                values[chunk[0] as usize * stride + offset],
                values[chunk[1] as usize * stride + offset],
                values[chunk[2] as usize * stride + offset],
                values[chunk[3] as usize * stride + offset],
            ]);
            let new = lows + v;
            carries -= lows.simd_gt(new).to_simd().cast::<u64>();
            lows = new;
        }
        let mut total: u128 = 0;
        for lane in 0..4 {
            total +=
                u128::from(lows.as_array()[lane]) + (u128::from(carries.as_array()[lane]) << 64);
        }
        for &i in tail {
            total += u128::from(values[i as usize * stride + offset]);
        }
        total
    }
    fn scalar_addressed_sum_w8(
        values: &[u64],
        stride: usize,
        offset: usize,
        indices: &[u32],
    ) -> u128 {
        let mut lows = [Simd::<u64, 4>::splat(0); 2];
        let mut carries = [Simd::<u64, 4>::splat(0); 2];
        let (chunks, tail) = indices.as_chunks::<8>();
        for chunk in chunks {
            for half in 0..2 {
                let c = &chunk[half * 4..half * 4 + 4];
                let v = Simd::<u64, 4>::from_array([
                    values[c[0] as usize * stride + offset],
                    values[c[1] as usize * stride + offset],
                    values[c[2] as usize * stride + offset],
                    values[c[3] as usize * stride + offset],
                ]);
                let new = lows[half] + v;
                carries[half] -= lows[half].simd_gt(new).to_simd().cast::<u64>();
                lows[half] = new;
            }
        }
        let mut total: u128 = 0;
        for half in 0..2 {
            for lane in 0..4 {
                total += u128::from(lows[half].as_array()[lane])
                    + (u128::from(carries[half].as_array()[lane]) << 64);
            }
        }
        for &i in tail {
            total += u128::from(values[i as usize * stride + offset]);
        }
        total
    }
    #[expect(
        unsafe_code,
        reason = "the falsifier's unchecked arm — bounds branches removed to exonerate them"
    )]
    fn scalar_addressed_sum_w4_unchecked(
        values: &[u64],
        stride: usize,
        offset: usize,
        indices: &[u32],
    ) -> u128 {
        let mut lows = Simd::<u64, 4>::splat(0);
        let mut carries = Simd::<u64, 4>::splat(0);
        let (chunks, tail) = indices.as_chunks::<4>();
        for chunk in chunks {
            // SAFETY: the rig generates in-bounds survivors by construction.
            let v = unsafe {
                Simd::<u64, 4>::from_array([
                    *values.get_unchecked(chunk[0] as usize * stride + offset),
                    *values.get_unchecked(chunk[1] as usize * stride + offset),
                    *values.get_unchecked(chunk[2] as usize * stride + offset),
                    *values.get_unchecked(chunk[3] as usize * stride + offset),
                ])
            };
            let new = lows + v;
            carries -= lows.simd_gt(new).to_simd().cast::<u64>();
            lows = new;
        }
        let mut total: u128 = 0;
        for lane in 0..4 {
            total +=
                u128::from(lows.as_array()[lane]) + (u128::from(carries.as_array()[lane]) << 64);
        }
        for &i in tail {
            total += u128::from(values[i as usize * stride + offset]);
        }
        total
    }
    // The shipped lowering with stride hardwired to 1 — the shape the
    // engine's scan-pushdown call site (`fold_sum_u64_idx(col, 1, 0,
    // p)`, sink.rs) actually inlines: the vector multiply folds to
    // `ushll`/`shl` shifts and the scalar-`mul`+`fmov` index entry
    // disappears; only the ladder remains.
    fn simd_index_sum_s1(values: &[u64], indices: &[u32]) -> u128 {
        let mut lows = Simd::<u64, 4>::splat(0);
        let mut carries = Simd::<u64, 4>::splat(0);
        let (chunks, tail) = indices.as_chunks::<4>();
        for chunk in chunks {
            let idx = Simd::<u32, 4>::from_array(*chunk).cast::<usize>();
            let v = Simd::gather_or_default(values, idx);
            let new = lows + v;
            carries -= lows.simd_gt(new).to_simd().cast::<u64>();
            lows = new;
        }
        let mut total: u128 = 0;
        for lane in 0..4 {
            total +=
                u128::from(lows.as_array()[lane]) + (u128::from(carries.as_array()[lane]) << 64);
        }
        for &i in tail {
            total += u128::from(values[i as usize]);
        }
        total
    }
    fn simd_index_min_max_s1(values: &[u64], indices: &[u32]) -> (u64, u64) {
        let mut mins = Simd::<u64, 4>::splat(u64::MAX);
        let mut maxs = Simd::<u64, 4>::splat(u64::MIN);
        let (chunks, tail) = indices.as_chunks::<4>();
        for chunk in chunks {
            let idx = Simd::<u32, 4>::from_array(*chunk).cast::<usize>();
            let v = Simd::gather_or_default(values, idx);
            mins = mins.simd_min(v);
            maxs = maxs.simd_max(v);
        }
        let mut min_scalar = mins.reduce_min();
        let mut max_scalar = maxs.reduce_max();
        for &i in tail {
            let word = values[i as usize];
            min_scalar = min_scalar.min(word);
            max_scalar = max_scalar.max(word);
        }
        (min_scalar, max_scalar)
    }

    // The full-scalar control arm (the pre-crucible shape: adds/cinc
    // carry counting on the flag-port triad).
    fn full_scalar_sum(values: &[u64], stride: usize, offset: usize, indices: &[u32]) -> u128 {
        let mut lo = [0u64; 4];
        let mut hi = [0u64; 4];
        let (chunks, tail) = indices.as_chunks::<4>();
        for chunk in chunks {
            for lane in 0..4 {
                let v = values[chunk[lane] as usize * stride + offset];
                let (s, c) = lo[lane].overflowing_add(v);
                lo[lane] = s;
                hi[lane] += u64::from(c);
            }
        }
        let mut total: u128 = 0;
        for lane in 0..4 {
            total += u128::from(lo[lane]) + (u128::from(hi[lane]) << 64);
        }
        for &i in tail {
            total += u128::from(values[i as usize * stride + offset]);
        }
        total
    }
    fn scalar_addressed_min_max_w4(
        values: &[u64],
        stride: usize,
        offset: usize,
        indices: &[u32],
    ) -> (u64, u64) {
        let mut mins = Simd::<u64, 4>::splat(u64::MAX);
        let mut maxs = Simd::<u64, 4>::splat(u64::MIN);
        let (chunks, tail) = indices.as_chunks::<4>();
        for chunk in chunks {
            let v = Simd::<u64, 4>::from_array([
                values[chunk[0] as usize * stride + offset],
                values[chunk[1] as usize * stride + offset],
                values[chunk[2] as usize * stride + offset],
                values[chunk[3] as usize * stride + offset],
            ]);
            mins = mins.simd_min(v);
            maxs = maxs.simd_max(v);
        }
        let mut min_scalar = mins.reduce_min();
        let mut max_scalar = maxs.reduce_max();
        for &i in tail {
            let word = values[i as usize * stride + offset];
            min_scalar = min_scalar.min(word);
            max_scalar = max_scalar.max(word);
        }
        (min_scalar, max_scalar)
    }
    fn full_scalar_min_max(
        values: &[u64],
        stride: usize,
        offset: usize,
        indices: &[u32],
    ) -> (u64, u64) {
        let mut mins = [u64::MAX; 4];
        let mut maxs = [u64::MIN; 4];
        let (chunks, tail) = indices.as_chunks::<4>();
        for chunk in chunks {
            for lane in 0..4 {
                let word = values[chunk[lane] as usize * stride + offset];
                mins[lane] = mins[lane].min(word);
                maxs[lane] = maxs[lane].max(word);
            }
        }
        let mut min_scalar = mins.iter().copied().min().expect("four lanes");
        let mut max_scalar = maxs.iter().copied().max().expect("four lanes");
        for &i in tail {
            let word = values[i as usize * stride + offset];
            min_scalar = min_scalar.min(word);
            max_scalar = max_scalar.max(word);
        }
        (min_scalar, max_scalar)
    }

    // Ascending sparse survivor sets — the filter-output shape the
    // aggregate sinks actually see (gap span 0: shuffled, the
    // non-engine random-gather diagnostic). Fresh per span (the TAGE
    // law and cache honesty both).
    fn survivors(rng: &mut Lcg, count: usize, gap_span: u64, bound: usize) -> Vec<u32> {
        let mut v = Vec::with_capacity(count);
        if gap_span == 0 {
            for _ in 0..count {
                v.push(u32::try_from(rng.next() % bound as u64).expect("bounded"));
            }
            return v;
        }
        let mut pos = rng.next() % gap_span;
        for _ in 0..count {
            if usize::try_from(pos).expect("u32-bounded") >= bound {
                break;
            }
            v.push(u32::try_from(pos).expect("bounded"));
            pos += 1 + rng.next() % gap_span;
        }
        v
    }

    fn median(mut xs: Vec<f64>) -> f64 {
        xs.sort_by(f64::total_cmp);
        xs[xs.len() / 2]
    }

    let mut rng = Lcg(0x5eed);

    // (label, column words, stride, survivor count, gap span, calls
    // per span). The displaced tiers are the regime the prediction
    // named: ascending survivors whose mean byte-gap (~32 KB, two
    // pages) defeats the stream trackers, so every gather is an
    // un-prefetched DRAM miss — the shape a selective filter over a
    // big column feeds the aggregate sinks (stride 3: the leaf batch's
    // entry-major keys at arity 3). The 512B-gap tier is the
    // prefetch-covered streaming regime; shuffled is the ledger's
    // gather-wall diagnostic (not an engine shape — survivors ascend).
    let tiers: &[(&str, usize, usize, usize, u64, usize)] = &[
        ("DRAM-displaced s1", 1 << 25, 1, 8_000, 8_191, 8),
        ("DRAM-displaced s3", 1 << 25, 3, 8_000, 2_730, 8),
        ("DRAM-stream s1", 1 << 25, 1, 450_000, 127, 1),
        ("DRAM-shuffled s1", 1 << 25, 1, 450_000, 0, 1),
        ("L2-resident s1", 1 << 18, 1, 32_768, 13, 32),
    ];
    for &(label, words, stride, count, gap_span, calls) in tiers {
        let values: Vec<u64> = (0..words as u64)
            .map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15))
            .collect();
        let bound = words / stride;

        // Exactness cross-check across every arm on one shared set.
        let shared = survivors(&mut rng, count, gap_span, bound);
        let want_sum = fold_sum_u64_idx(&values, stride, 0, &shared);
        assert_eq!(
            scalar_addressed_sum_w4(&values, stride, 0, &shared),
            want_sum
        );
        assert_eq!(
            scalar_addressed_sum_w8(&values, stride, 0, &shared),
            want_sum
        );
        assert_eq!(
            scalar_addressed_sum_w4_unchecked(&values, stride, 0, &shared),
            want_sum
        );
        assert_eq!(full_scalar_sum(&values, stride, 0, &shared), want_sum);
        let want_mm = fold_min_max_u64_idx(&values, stride, 0, &shared);
        assert_eq!(
            scalar_addressed_min_max_w4(&values, stride, 0, &shared),
            want_mm
        );
        assert_eq!(full_scalar_min_max(&values, stride, 0, &shared), want_mm);
        if stride == 1 {
            assert_eq!(simd_index_sum_s1(&values, &shared), want_sum);
            assert_eq!(simd_index_min_max_s1(&values, &shared), want_mm);
        }

        // A span: `calls` kernel invocations, each over its own fresh
        // survivor set, generated untimed. Returns (secs, indices).
        let span = |f: ArmFn, sets: &[Vec<u32>]| -> (f64, usize) {
            let start = Instant::now();
            let mut sink = 0u128;
            for set in sets {
                sink = sink.wrapping_add(f(&values, set));
            }
            std::hint::black_box(sink);
            let n: usize = sets.iter().map(Vec::len).sum();
            (start.elapsed().as_secs_f64(), n)
        };

        let sum_shipped = |v: &[u64], s: &[u32]| fold_sum_u64_idx(v, stride, 0, s);
        let sum_w4 = |v: &[u64], s: &[u32]| scalar_addressed_sum_w4(v, stride, 0, s);
        let sum_w8 = |v: &[u64], s: &[u32]| scalar_addressed_sum_w8(v, stride, 0, s);
        let sum_w4u = |v: &[u64], s: &[u32]| scalar_addressed_sum_w4_unchecked(v, stride, 0, s);
        let sum_scalar = |v: &[u64], s: &[u32]| full_scalar_sum(v, stride, 0, s);
        let sum_spec = |v: &[u64], s: &[u32]| simd_index_sum_s1(v, s);
        let mut sum_arms: Vec<(&str, ArmFn)> = vec![
            ("shipped-simd-index", &sum_shipped),
            ("scalar-addr-w4", &sum_w4),
            ("scalar-addr-w8", &sum_w8),
            ("scalar-addr-w4-unchecked", &sum_w4u),
            ("full-scalar", &sum_scalar),
        ];
        if stride == 1 {
            sum_arms.push(("simd-index-spec-s1", &sum_spec));
        }
        let fold_mm = |(lo, hi): (u64, u64)| u128::from(lo) ^ u128::from(hi);
        let mm_shipped = |v: &[u64], s: &[u32]| fold_mm(fold_min_max_u64_idx(v, stride, 0, s));
        let mm_w4 = |v: &[u64], s: &[u32]| fold_mm(scalar_addressed_min_max_w4(v, stride, 0, s));
        let mm_scalar = |v: &[u64], s: &[u32]| fold_mm(full_scalar_min_max(v, stride, 0, s));
        let mm_spec = |v: &[u64], s: &[u32]| fold_mm(simd_index_min_max_s1(v, s));
        let mut mm_arms: Vec<(&str, ArmFn)> = vec![
            ("shipped-simd-index", &mm_shipped),
            ("scalar-addr-w4", &mm_w4),
            ("full-scalar", &mm_scalar),
        ];
        if stride == 1 {
            mm_arms.push(("simd-index-spec-s1", &mm_spec));
        }

        for (op, arms) in [("sum", &sum_arms), ("minmax", &mm_arms)] {
            let pairs = 31;
            let mut ns: Vec<Vec<f64>> = vec![Vec::new(); arms.len()];
            let fresh_sets = |rng: &mut Lcg| -> Vec<Vec<u32>> {
                (0..calls)
                    .map(|_| survivors(rng, count, gap_span, bound))
                    .collect()
            };
            // Warmup: one untimed span per arm.
            for (_, f) in arms {
                span(*f, &fresh_sets(&mut rng));
            }
            for pair in 0..pairs {
                // Interleaved same-session, order alternated per pair.
                let run = |k: usize, ns: &mut Vec<Vec<f64>>, rng: &mut Lcg| {
                    let sets = fresh_sets(rng);
                    let (t, n) = span(arms[k].1, &sets);
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "index counts are far below 2^52"
                    )]
                    ns[k].push(t / n as f64 * 1e9);
                };
                if pair % 2 == 0 {
                    for k in 0..arms.len() {
                        run(k, &mut ns, &mut rng);
                    }
                } else {
                    for k in (0..arms.len()).rev() {
                        run(k, &mut ns, &mut rng);
                    }
                }
            }
            let base = median(ns[0].clone());
            let mut line = format!(
                "{label} {op}: {} {base:.2} ns/idx (absolutes VOID under co-tenancy)",
                arms[0].0
            );
            for (k, (name, _)) in arms.iter().enumerate().skip(1) {
                let m = median(ns[k].clone());
                write!(line, "; {name}/shipped = {:.3}", m / base).expect("String write");
                // The live falsifier: the gravestone says no challenger
                // approaches the predicted 1.3–2x at the displaced
                // tiers (best measured: unchecked at 0.82); a ratio
                // under 0.75 reopens it.
                if label.starts_with("DRAM-displaced") {
                    assert!(
                        m / base > 0.75,
                        "{label} {op}: challenger {name} beats the shipped kernel by >25% \
                         ({m:.2} vs {base:.2} ns/idx) — the gravestone is wrong, reopen it"
                    );
                }
            }
            println!("{line}; pairs {pairs}");
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
        ("tiny    4", 4),
        ("tiny   16", 16),
        ("small  1K", 1_024),
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
        // The constant-operand reference against classify too (its live
        // dispatch reader is the non-aarch64 build; here it is oracle-
        // checked on every target).
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

/// The gathered const-operand wrapper (`allen_code_batch_const`, the
/// leaf residual's parent-constant dispatch target) is bit-identical to
/// the four-stream kernel over broadcast b-streams AND to per-pair
/// classify — both orientations of the residual reach it (the swapped
/// one through the converse mask, tested at the executor), so the codes
/// themselves are pinned here across the window-width lengths.
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

/// `allen_filter_batch` (codes + broadcast mask → keep bytes) is
/// bit-identical to the scalar reference across the 13 singletons, the
/// workload composites, and randomized masks — and its keep byte equals
/// `mask.contains(classify(...))` per pair.
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

/// The dense filter-position compositions (`allen_filter_columns` and
/// its constant-operand form) produce exactly the scalar
/// classify-and-test survivor positions, ascending, across boundary
/// lengths and masks — including chunk-boundary lengths around the
/// stack chunk width.
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
            let a = bumbledb_theory::Interval::<u64>::new(a_s[i], a_e[i]).expect("nonempty");
            let b = bumbledb_theory::Interval::<u64>::new(b_s[i], b_e[i]).expect("nonempty");
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
    use bumbledb_theory::allen::AllenMask;
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

/// The Miri-lane representative of the exhaustive mask suite: the same
/// pipeline over the 4-value endpoint grid (C(4,2) = 6 intervals, 36
/// pairs) × the 13 singletons, the workload composites, and 16 stride-
/// sampled masks. Runs everywhere the dispatch is interpretable (the
/// scalar/portable twins under Miri; the NEON path natively).
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

/// The pooled-reuse contract the retired zero-fills lean on
/// (`allen_code_batch`/`allen_filter_batch` resize without `clear`):
/// one codes/keep pair reused across batches whose lengths shrink and
/// regrow across BOTH NEON window widths (8 for codes, 16 for keep)
/// must stay bit-identical to fresh vectors — a stale retained byte
/// surviving into a shorter batch's window, or a tail the overlapped
/// last window missed, shows up as a divergence here. The existing
/// bit-identity sweeps allocate fresh outputs per call and never see
/// the retained-prefix path.
#[test]
fn allen_pooled_reuse_is_bit_identical_to_fresh_outputs() {
    use bumbledb_theory::allen::AllenMask;
    // Descend from far past the high-water into every fallback regime,
    // then regrow: each transition exercises resize-truncate (stale
    // capacity above n) and resize-grow (zero-fill delta) in turn.
    const REUSE_LADDER: &[usize] = &[257, 33, 17, 16, 15, 9, 8, 7, 3, 1, 0, 2, 12, 20, 100];
    let mut rng = Lcg(0x5EED_A11E);
    let mut pooled_codes = Vec::new();
    let mut pooled_keep = Vec::new();
    // Poison the pools' first high-water fill so any byte NOT overwritten
    // by the classify/membership pass is loud (0xEE is neither a valid
    // code nor a keep byte).
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
        // Every retained byte is a real 0/1 keep or a real 4-bit code —
        // the compaction kernel's debug contract over the read window.
        assert!(pooled_keep.iter().all(|&k| k <= 1));
        assert!(pooled_codes.iter().all(|&c| c < 13));
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

/// The T7 counter-spill falsifier (timing evidence; ignored: runs by
/// hand on the reference host —
/// `scripts/measure.sh cargo test -p bumbledb --release allen_filter_counter -- --ignored --nocapture`).
///
/// Arm A is [`neon::allen_filter_batch_neon_spill_arm`] — the pre-T7
/// kernel verbatim, its countdown routed through `std::hint::black_box`
/// (LLVM materializes that as a stack spill+reload of the counter per
/// 16-code window). Arm B is the shipped [`neon::allen_filter_batch_neon`]
/// with the register-pinned `asm!` back edge. The arms alternate per
/// span inside one process (m2max.method.interleaved-ab) on L1-resident
/// buffers; the kernel has no data-dependent branch, so repetition on
/// fixed data is the valid protocol here (the TAGE caveat's own
/// counter-case), and every span is loop-amortized far above the sub-µs
/// attribution floor.
#[cfg(target_arch = "aarch64")]
#[test]
#[ignore = "timing evidence, run by hand on the reference host"]
#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn allen_filter_counter_spill_ab() {
    // Four distinct 8 KiB code buffers + one keep buffer: ~40 KiB
    // touched, L1-resident on the reference host (128 KiB L1d). Spans
    // loop-amortize CALLS kernel calls (~0.5M lanes, tens of µs).
    const N: usize = 8192;
    const BUFS: usize = 4;
    const CALLS: usize = 64;
    const SPANS: usize = 300;

    let filter_spill = neon::allen_filter_batch_neon_spill_arm;
    let filter_reg = neon::allen_filter_batch_neon;

    let mut rng = Lcg(0xA11E);
    let bufs: Vec<Vec<u8>> = (0..BUFS)
        .map(|_| {
            (0..N)
                .map(|_| u8::try_from(rng.next() % 13).expect("code"))
                .collect()
        })
        .collect();
    let mask_bits = 0b0_0100_0010_0011u16; // an arbitrary nontrivial mask
    let mut keep_a = vec![0u8; N];
    let mut keep_b = vec![0u8; N];

    // Sanity: identical outputs on every buffer.
    for buf in &bufs {
        filter_spill(buf, mask_bits, &mut keep_a);
        filter_reg(buf, mask_bits, &mut keep_b);
        assert_eq!(keep_a, keep_b, "the arms disagree — the twin is void");
    }

    // Interleaved spans: arms alternating A,B,A,B within the one
    // process. Paired per-alternation ratios are the statistic.
    let span = |f: fn(&[u8], u16, &mut [u8]), keep: &mut [u8], salt: usize| {
        let start = std::time::Instant::now();
        for call in 0..CALLS {
            f(&bufs[(call + salt) % BUFS], mask_bits, keep);
        }
        start.elapsed().as_nanos()
    };
    // Warmup both arms.
    for salt in 0..8 {
        span(filter_spill, &mut keep_a, salt);
        span(filter_reg, &mut keep_b, salt);
    }
    let mut ratios: Vec<f64> = Vec::with_capacity(SPANS);
    let (mut min_a, mut min_b) = (u128::MAX, u128::MAX);
    for s in 0..SPANS {
        let a = span(filter_spill, &mut keep_a, s);
        let b = span(filter_reg, &mut keep_b, s);
        min_a = min_a.min(a);
        min_b = min_b.min(b);
        ratios.push(a as f64 / b as f64);
    }
    ratios.sort_by(|x, y| x.partial_cmp(y).expect("finite"));
    let pct = |p: usize| ratios[(ratios.len() - 1) * p / 100];
    let min_ratio = min_a as f64 / min_b as f64;
    println!(
        "allen_filter counter A/B (spill/reg), L1-resident, {SPANS} alternations x {CALLS} calls x {N} codes:"
    );
    println!(
        "  paired ratio p10 {:.3}  p50 {:.3}  p90 {:.3}   min-of-spans ratio {min_ratio:.3}",
        pct(10),
        pct(50),
        pct(90),
    );
    println!(
        "  min span: spill {min_a} ns  reg {min_b} ns  ({:.3} vs {:.3} codes/ns)",
        (N * CALLS) as f64 / min_a as f64,
        (N * CALLS) as f64 / min_b as f64,
    );
}

/// The unsafe-allowlist law for the compaction kernel: bit-identical to
/// the fully safe-indexed reference across randomized 0/1 masks (the
/// producers' contract) at every selectivity, plus the degenerate
/// shapes — all-zero, all-one, alternating — over the lane-stress
/// lengths and a long tail. The mask is allowed to run longer than the
/// items (the kernel's documented tolerance).
#[test]
fn compaction_matches_the_safe_reference_bit_for_bit() {
    let mut rng = Lcg(0xC0 /*mpact*/);
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
            // The longer-mask tolerance: trailing mask bytes past the
            // items are never read.
            mask.push(1);
            let mut kernel_long = source.clone();
            compact_u32_by_mask(&mut kernel_long, &mask);
            assert_eq!(kernel_long, kernel, "longer mask, len {len}");
        }
    }
}

/// Compaction-throughput evidence (a gate; ignored: a timing test runs
/// only by hand, under the measurement mutex —
/// `scripts/measure.sh cargo test -p bumbledb --release compact_throughput -- --ignored --nocapture`).
///
/// The interleaved A/B twin for the triad diet: arm A is the pre-diet
/// safe-indexed shape ([`reference::compact_u32_by_mask`] behind an
/// `#[inline(never)]` wall, its own disassembly subject), arm B the
/// live kernel. L1-resident (8 K items = 32 KB + 8 KB mask), fresh
/// masks per rep (128 distinct windows ≈ 1 M outcomes — past the TAGE
/// capacity edge, `m2max.predict.tage-memorizes-benchmarks`),
/// selectivity swept 1–99 % (the branchless law expects flatness,
/// `m2max.predict.branchless-flat`). Only the same-session interleaved
/// ratio is meaningful under co-tenancy (`m2max.method.interleaved-ab`);
/// the printed ns/item are refill-subtracted estimates, not absolutes.
/// The gate: the live kernel is never slower than the pre-diet shape
/// beyond the ±2 % validity band, at any selectivity.
#[test]
#[ignore = "timing evidence, run by hand on the reference host"]
fn compact_throughput_interleaved_ab() {
    #[inline(never)]
    fn arm_a(items: &mut Vec<u32>, mask: &[u8]) {
        super::reference::compact_u32_by_mask(items, mask);
    }
    #[inline(never)]
    fn arm_b(items: &mut Vec<u32>, mask: &[u8]) {
        compact_u32_by_mask(items, mask);
    }
    const N: usize = 8_192;
    const WINDOWS: usize = 128;
    const REPS_PER_SPAN: usize = 128;
    const PAIRS: usize = 24;
    let source: Vec<u32> = (0..N).map(|i| u32::try_from(i).expect("small")).collect();
    let mut rng = Lcg(0xAB);
    // One timed span: REPS_PER_SPAN × (refill + compact), fresh mask
    // window per rep. Returns ns/item including the refill.
    let span =
        |arm: &mut dyn FnMut(&mut Vec<u32>, &[u8]), items: &mut Vec<u32>, masks: &[Vec<u8>]| {
            let start = std::time::Instant::now();
            let mut sink = 0usize;
            for rep in 0..REPS_PER_SPAN {
                items.clear();
                items.extend_from_slice(&source);
                arm(items, &masks[rep % WINDOWS]);
                sink = sink.wrapping_add(items.len());
            }
            let elapsed = start.elapsed();
            std::hint::black_box(sink);
            #[expect(clippy::cast_precision_loss, reason = "reporting")]
            {
                elapsed.as_nanos() as f64 / (N * REPS_PER_SPAN) as f64
            }
        };
    let refill_only = |items: &mut Vec<u32>| {
        let start = std::time::Instant::now();
        for _ in 0..REPS_PER_SPAN {
            items.clear();
            items.extend_from_slice(&source);
            std::hint::black_box(items.len());
        }
        #[expect(clippy::cast_precision_loss, reason = "reporting")]
        {
            start.elapsed().as_nanos() as f64 / (N * REPS_PER_SPAN) as f64
        }
    };
    println!("sel%  A ns/item  B ns/item  A/B (refill-subtracted, medians of {PAIRS} pairs)");
    for percent in [1u64, 25, 50, 75, 99] {
        let masks: Vec<Vec<u8>> = (0..WINDOWS)
            .map(|_| {
                (0..N)
                    .map(|_| u8::from(rng.next() % 100 < percent))
                    .collect()
            })
            .collect();
        let mut items = source.clone();
        // Warmup both arms.
        let _ = span(&mut arm_a, &mut items, &masks);
        let _ = span(&mut arm_b, &mut items, &masks);
        let refill = {
            let mut samples: Vec<f64> = (0..8).map(|_| refill_only(&mut items)).collect();
            samples.sort_by(f64::total_cmp);
            samples[samples.len() / 2]
        };
        let mut ratios = Vec::with_capacity(PAIRS);
        let (mut a_net, mut b_net) = (Vec::new(), Vec::new());
        for _ in 0..PAIRS {
            let a = span(&mut arm_a, &mut items, &masks) - refill;
            let b = span(&mut arm_b, &mut items, &masks) - refill;
            ratios.push(a / b);
            a_net.push(a);
            b_net.push(b);
        }
        ratios.sort_by(f64::total_cmp);
        a_net.sort_by(f64::total_cmp);
        b_net.sort_by(f64::total_cmp);
        let median = ratios[PAIRS / 2];
        println!(
            "{percent:>3}   {:>8.4}  {:>8.4}  {median:.4} [{:.4} .. {:.4}]",
            a_net[PAIRS / 2],
            b_net[PAIRS / 2],
            ratios[0],
            ratios[PAIRS - 1],
        );
        assert!(
            median >= 0.98,
            "the diet regressed at {percent}% selectivity: A/B median {median:.4}"
        );
    }
}
