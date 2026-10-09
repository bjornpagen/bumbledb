//! Batch passes shared by the leaf and pipelined drives: residual compares,
//! Allen residuals and sibling probe-key gathers. Each reads operands through
//! [`BatchRows`], so one body serves both binding layouts.
use super::{
    AllenResidualSpec, BatchBuffers, BatchRows, Counters, ResidualSpec, Source, grow_scratch,
};

/// Keeps the survivors that satisfy every word residual.
pub(super) fn residual_pass<R: BatchRows, C: Counters>(
    specs: &[ResidualSpec],
    sources: &[(Source, Source)],
    rows: &R,
    buf: &mut BatchBuffers,
    node: usize,
    counters: &mut C,
) {
    for (spec, &(lhs, rhs)) in specs.iter().zip(sources) {
        let n = buf.survivors.len();
        grow_scratch(&mut buf.mask, n);
        for k in 0..n {
            let element = buf.survivors[k] as usize;
            let pass = super::compare_wide(
                spec.op,
                spec.width,
                |offset| rows.word(element, lhs, offset),
                |offset| rows.word(element, rhs, offset),
            );
            counters.residual(node, pass);
            buf.mask[k] = u8::from(pass);
        }
        crate::exec::kernel::compact_u32_by_mask(&mut buf.survivors, &buf.mask);
    }
}

/// Keeps the survivors whose interval pairs satisfy every Allen residual.
/// A side that is constant across the batch is classified once.
pub(super) fn allen_pass<R: BatchRows, C: Counters>(
    specs: &[AllenResidualSpec],
    sources: &[(Source, Source)],
    rows: &R,
    buf: &mut BatchBuffers,
    node: usize,
    counters: &mut C,
) {
    for (spec, &(lhs, rhs)) in specs.iter().zip(sources) {
        let n = buf.survivors.len();
        let constant = |source: Source| rows.constant(source, 0).zip(rows.constant(source, 1));
        let filter_mask = match (constant(lhs), constant(rhs)) {
            (None, None) => {
                grow_scratch(&mut buf.allen_gather, 4 * n);
                let (a_starts, rest) = buf.allen_gather[..4 * n].split_at_mut(n);
                let (a_ends, rest) = rest.split_at_mut(n);
                let (b_starts, b_ends) = rest.split_at_mut(n);
                for (k, &element) in buf.survivors[..n].iter().enumerate() {
                    let element = element as usize;
                    a_starts[k] = rows.word(element, lhs, 0);
                    a_ends[k] = rows.word(element, lhs, 1);
                    b_starts[k] = rows.word(element, rhs, 0);
                    b_ends[k] = rows.word(element, rhs, 1);
                }
                crate::exec::kernel::allen_code_batch(
                    a_starts,
                    a_ends,
                    b_starts,
                    b_ends,
                    &mut buf.allen_codes,
                );
                Some(spec.mask)
            }
            (None, Some(fixed)) => {
                classify_against(rows, lhs, fixed, buf);
                Some(spec.mask)
            }
            (Some(fixed), None) => {
                classify_against(rows, rhs, fixed, buf);
                Some(spec.mask.converse())
            }
            (Some((a_start, a_end)), Some((b_start, b_end))) => {
                let code = crate::allen::classify_bounds(&a_start, &a_end, &b_start, &b_end);
                buf.mask.clear();
                buf.mask.resize(n, u8::from(spec.mask.contains(code)));
                None
            }
        };
        if let Some(filter_mask) = filter_mask {
            crate::exec::kernel::allen_filter_batch(&buf.allen_codes, filter_mask, &mut buf.mask);
        }
        for &keep in &buf.mask[..n] {
            counters.residual(node, keep != 0);
        }
        crate::exec::kernel::compact_u32_by_mask(&mut buf.survivors, &buf.mask);
    }
}

/// Allen codes of each survivor's `varying` interval against one fixed pair.
fn classify_against<R: BatchRows>(
    rows: &R,
    varying: Source,
    (b_start, b_end): (u64, u64),
    buf: &mut BatchBuffers,
) {
    let n = buf.survivors.len();
    grow_scratch(&mut buf.allen_gather, 2 * n);
    let (starts, ends) = buf.allen_gather[..2 * n].split_at_mut(n);
    for (k, &element) in buf.survivors.iter().enumerate() {
        starts[k] = rows.word(element as usize, varying, 0);
        ends[k] = rows.word(element as usize, varying, 1);
    }
    crate::exec::kernel::allen_code_batch_const(starts, ends, b_start, b_end, &mut buf.allen_codes);
}

/// Writes each survivor's probe key (`sources.len()` words) and, when
/// `hash`, its hash. Widths 1 to 4 run a const-width body.
pub(super) fn gather_probe_keys<R: BatchRows>(
    rows: &R,
    sources: &[Source],
    survivors: &[u32],
    probe_keys: &mut [u64],
    hashes: &mut [u64],
    hash: bool,
) {
    match (sources.len(), hash) {
        (1, true) => gather_hash_core::<1, R>(rows, sources, survivors, probe_keys, hashes),
        (2, true) => gather_hash_core::<2, R>(rows, sources, survivors, probe_keys, hashes),
        (3, true) => gather_hash_core::<3, R>(rows, sources, survivors, probe_keys, hashes),
        (4, true) => gather_hash_core::<4, R>(rows, sources, survivors, probe_keys, hashes),
        (width, _) => {
            for (k, &element) in survivors.iter().enumerate() {
                let key = &mut probe_keys[k * width..(k + 1) * width];
                for (word, &source) in key.iter_mut().zip(sources) {
                    *word = rows.word(element as usize, source, 0);
                }
                if hash {
                    hashes[k] = crate::exec::colt::hash_key(key);
                }
            }
        }
    }
}

#[expect(
    clippy::inline_always,
    reason = "a monomorphized pure-ALU leaf of the probe hot loop: the swar \
              module's contract (its `bl` would be the cost the dispatch exists \
              to remove)"
)]
#[inline(always)]
fn gather_hash_core<const K: usize, R: BatchRows>(
    rows: &R,
    sources: &[Source],
    survivors: &[u32],
    probe_keys: &mut [u64],
    hashes: &mut [u64],
) {
    let sources: &[Source; K] = sources
        .try_into()
        .expect("the dispatch width is the source count");
    for (k, &element) in survivors.iter().enumerate() {
        let mut key = [0_u64; K];
        for (word, &source) in key.iter_mut().zip(sources) {
            *word = rows.word(element as usize, source, 0);
        }
        probe_keys[k * K..(k + 1) * K].copy_from_slice(&key);
        hashes[k] = crate::exec::colt::hash_key_core::<K>(&key);
    }
}

/// Resolves one element's membership points into `checks`; a `dense` point
/// goes through the finite-probe guard, so a nonfinite F64 matches nothing.
pub(super) fn resolve_points<R: BatchRows>(
    rows: &R,
    element: usize,
    sources: &[super::PointSource],
    checks: &mut Vec<(usize, usize, u64)>,
) {
    checks.clear();
    for &(start_col, end_col, source, dense) in sources {
        let point = rows.word(element, source, 0);
        let point = if dense {
            crate::image::view::dense_probe_word(point)
        } else {
            point
        };
        checks.push((start_col, end_col, point));
    }
}
