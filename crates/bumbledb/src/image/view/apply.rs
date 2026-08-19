//! The filter-application mechanism (docs/architecture/40-execution.md):
//! evaluates the per-atom conjunction over a warm image into a
//! survivor-position vector.

use std::sync::Arc;

use crate::image::{ColumnView, ColumnWidth, RelationImage};
use crate::ir::WordCmp;

use super::eval::{const_interval, mask_of, resolve, row_holds};
use super::{BoundView, Const, FilterPredicate, OperandAddr, SetConst, View, ViewWordSource};

fn resolve_word(value: &ViewWordSource, params: &[Const]) -> u64 {
    match value {
        ViewWordSource::Word(word) => *word,
        ViewWordSource::Param(param) => match &params[usize::from(param.0)] {
            Const::Word(word) => *word,
            _ => unreachable!("param slice: word param resolves to a word"),
        },
    }
}

/// The single column of a scalar field, through its span (the width
/// dispatch every field→column translation runs).
fn scalar_column(image: &RelationImage, field: OperandAddr) -> ColumnView<'_> {
    image.column(usize::from(image.span(field.field()).first_column))
}

/// The resolved point word of a membership filter. Var-sourced points
/// never reach the view evaluator: they live on the occurrence's
/// `point_vars` and plan validation routes them into membership probes.
fn point_word(point: &ViewWordSource, params: &[Const]) -> u64 {
    resolve_word(point, params)
}

/// The resolved word set behind a set constant (sorted, deduplicated).
fn word_set<'a>(set: &'a SetConst, params: &'a [Const]) -> &'a [u64] {
    match set {
        SetConst::WordSet(words) => words,
        SetConst::ParamSet(param) => match &params[usize::from(param.0)] {
            Const::WordSet(words) => words,
            _ => unreachable!("param slice: set param resolves to a word set"),
        },
    }
}

/// Applies the filter conjunction over a (warm) image, writing survivors
/// into `buf` (caller-owned, reused across executions — capacity is
/// retained). An empty predicate list yields the unfiltered [`View::All`].
///
/// **The filter-order law** (docs/architecture/20-query-ir.md, § the
/// measure): the measure kinds evaluate last, over the survivors of every
/// other predicate of the atom. A ray (`end == MAX`) never survives the
/// comparison — its verdict is Ray, not Fails, and it is never an error
/// HERE: the Kleene verdict algebra (ruled 2026-07-23, R6) raises
/// `MeasureOfRay` only for a complete binding whose folded verdict is
/// Ray, which the prepared query's ray-probe pass renders after the
/// rule loop (`api/prepared/build.rs` § ray probes).
///
/// # Panics
///
/// Only on programmer-invariant violations: an image beyond the u32
/// position space (the 32 GiB map physically bounds live rows roughly an
/// order of magnitude under u32; the validated 10⁷ scale sits far below).
#[must_use]
pub fn apply(
    image: &Arc<RelationImage>,
    predicates: &[FilterPredicate],
    params: &[Const],
    buf: Vec<u32>,
) -> View {
    if !predicates.iter().any(is_measure) {
        return apply_infallible(image, predicates, params, buf);
    }
    // The measure path, correct by order: the infallible predicates run
    // through the ordinary machinery first — over the SAME borrowed
    // list, skipping the measure kinds in place (no partition, no
    // per-build `Vec`, no predicate deep-clones: the steady-state
    // allocation contract, finding 051) — then each measure kind
    // refines their survivors. When EVERY predicate is a measure, the
    // caller's pooled survivor buffer stays in hand and seeds the first
    // refinement instead of being dropped for a fresh allocation.
    let (mut view, mut spare) = if predicates.iter().all(is_measure) {
        (View::Bound(BoundView::All(Arc::clone(image))), buf)
    } else {
        (apply_infallible(image, predicates, params, buf), Vec::new())
    };
    for predicate in predicates.iter().filter(|p| is_measure(p)) {
        view = refine_measure(image, predicate, params, view, &mut spare);
    }
    view
}

/// The measure kinds — evaluated last by the filter-order law, fallibly
/// ([`refine_measure`]); everything else is the infallible conjunction.
fn is_measure(p: &FilterPredicate) -> bool {
    matches!(
        p,
        FilterPredicate::DurationCompare { .. } | FilterPredicate::DurationFieldsCompare { .. }
    )
}

/// One measure predicate over the current survivors. A full view takes
/// the fused dense kernel (subtract + range test + ray test in one
/// stride-1 pass); survivor views refine scalar, position by position.
/// A ray never survives (its verdict is Ray — the ray-probe pass's
/// territory, R6), and never errors here. `spare` is the pooled
/// survivor buffer a `View::All` input consumes (capacity retained
/// across executions — finding 051); survivor inputs refine their own
/// buffer in place and never touch it.
fn refine_measure(
    image: &Arc<RelationImage>,
    predicate: &FilterPredicate,
    params: &[Const],
    view: View,
    spare: &mut Vec<u32>,
) -> View {
    match predicate {
        FilterPredicate::DurationCompare { field, op, value } => {
            let bound = resolve_word(value, params);
            // The order operator as an inclusive duration range — the
            // subtraction feeds the existing range machinery.
            let (lo, hi) = match op {
                crate::ir::OrderCmp::Lt => match bound.checked_sub(1) {
                    Some(hi) => (0, hi),
                    None => (1, 0), // dur < 0: empty (lo > hi keeps nothing)
                },
                crate::ir::OrderCmp::Le => (0, bound),
                crate::ir::OrderCmp::Gt => match bound.checked_add(1) {
                    Some(lo) => (lo, u64::MAX),
                    None => (1, 0), // dur > MAX: empty
                },
                crate::ir::OrderCmp::Ge => (bound, u64::MAX),
            };
            let (starts, ends) = interval_columns(image, *field);
            match view {
                View::Bound(BoundView::All(_)) => {
                    let mut positions = std::mem::take(spare);
                    positions.clear();
                    crate::exec::kernel::filter_duration_range_u64(
                        starts,
                        ends,
                        lo,
                        hi,
                        &mut positions,
                    );
                    View::Bound(BoundView::Survivors {
                        image: Arc::clone(image),
                        positions,
                    })
                }
                View::Bound(BoundView::Survivors {
                    image: view_image,
                    mut positions,
                }) => {
                    let mut cursor = 0usize;
                    for read in 0..positions.len() {
                        let p = positions[read] as usize;
                        let (start, end) = (starts[p], ends[p]);
                        positions[cursor] = positions[read];
                        cursor +=
                            usize::from(end != u64::MAX && lo <= end - start && end - start <= hi);
                    }
                    positions.truncate(cursor);
                    View::Bound(BoundView::Survivors {
                        image: view_image,
                        positions,
                    })
                }
                View::Unbound => unreachable!("apply binds the view it filters"),
            }
        }
        FilterPredicate::DurationFieldsCompare {
            interval,
            op,
            scalar,
        } => {
            // Two varying columns per position — no constant side, no
            // kernel shape (the `FieldsCompare` precedent): scalar over
            // whatever positions survive. The variant dispatch is the
            // TYPE's, exactly as the `DurationCompare` arm above — never
            // an emptiness sentinel reconstructing the erased view
            // (finding 115).
            let (starts, ends) = interval_columns(image, *interval);
            let scalars = match scalar_column(image, *scalar) {
                ColumnView::Words(words) => words,
                ColumnView::Bytes(_) => unreachable!("validated: the measure side is u64"),
            };
            let mut positions = match view {
                View::Bound(BoundView::All(_)) => {
                    let mut positions = std::mem::take(spare);
                    positions.clear();
                    positions
                        .extend(0..u32::try_from(image.row_count()).expect("positions fit u32"));
                    positions
                }
                View::Bound(BoundView::Survivors { positions, .. }) => positions,
                View::Unbound => unreachable!("apply binds the view it filters"),
            };
            let mut cursor = 0usize;
            for read in 0..positions.len() {
                let p = positions[read] as usize;
                let (start, end) = (starts[p], ends[p]);
                positions[cursor] = positions[read];
                cursor += usize::from(end != u64::MAX && op.compare(&(end - start), &scalars[p]));
            }
            positions.truncate(cursor);
            View::Bound(BoundView::Survivors {
                image: Arc::clone(image),
                positions,
            })
        }
        _ => unreachable!("refine_measure takes the measure kinds"),
    }
}

/// The infallible conjunction — every non-measure predicate kind. Called
/// with the atom's WHOLE borrowed filter list: measure kinds are skipped
/// in place (they refine afterward, [`refine_measure`]), so no partition
/// or clone ever materializes (finding 051).
#[must_use]
fn apply_infallible(
    image: &Arc<RelationImage>,
    predicates: &[FilterPredicate],
    params: &[Const],
    mut buf: Vec<u32>,
) -> View {
    if predicates.is_empty() {
        return View::Bound(BoundView::All(Arc::clone(image)));
    }
    let row_count = image.row_count();
    debug_assert!(u32::try_from(row_count).is_ok(), "positions fit u32");
    buf.clear();

    // Kernel fast path: the *first kernel-compatible* predicate (not
    // blindly `predicates[0]` — a leading FieldsCompare or byte-column
    // `Ne` must not hide the SIMD path; measure kinds never kernel-scan
    // here) produces the initial survivor set; every other predicate
    // refines it below.
    if let Some(pivot) = predicates
        .iter()
        .position(|p| !is_measure(p) && kernel_scan(image, p, params, &mut buf))
    {
        // Refine in place: evaluate the remaining conjunction per survivor
        // with the branchless cursor write. A single-predicate scan
        // refines vacuously (`idx == pivot` skips its only predicate;
        // every survivor keeps) — the loop already handles it, so no
        // early return exists for it.
        let mut cursor = 0usize;
        for read in 0..buf.len() {
            let position = buf[read] as usize;
            let mut keep = true;
            for (idx, predicate) in predicates.iter().enumerate() {
                if idx == pivot || is_measure(predicate) {
                    continue;
                }
                keep &= row_holds(image, std::slice::from_ref(predicate), params, position);
            }
            buf[cursor] = buf[read];
            cursor += usize::from(keep);
        }
        buf.truncate(cursor);
        return View::Bound(BoundView::Survivors {
            image: Arc::clone(image),
            positions: buf,
        });
    }

    buf.resize(row_count, 0);
    let mut cursor = 0usize;
    // The scalar branchless survivor write (D4's compaction pattern):
    // unconditional store, conditional cursor advance — no `if` in this
    // loop body.
    for position in 0..row_count {
        let keep = row_holds(image, predicates, params, position);
        buf[cursor] = u32::try_from(position).expect("checked above");
        cursor += usize::from(keep);
    }
    buf.truncate(cursor);
    View::Bound(BoundView::Survivors {
        image: Arc::clone(image),
        positions: buf,
    })
}

/// An interval field's two word-column slices — the operand shape of
/// every fused two-column composition.
fn interval_columns(image: &RelationImage, field: OperandAddr) -> (&[u64], &[u64]) {
    let span = image.span(field.field());
    debug_assert_eq!(span.width, ColumnWidth::WordPair);
    let first = usize::from(span.first_column);
    match (image.column(first), image.column(first + 1)) {
        (ColumnView::Words(starts), ColumnView::Words(ends)) => (starts, ends),
        _ => unreachable!("an interval span covers two word columns"),
    }
}

/// Attempts the kernel fast path for one predicate. Scalar compares
/// against a resolved `Word`/`Byte` constant lower to the fixed-width
/// predicate scans; the membership kinds (`PointIn`, `AnyPointIn`,
/// `FieldWithin`) lower to compositions of that same shape over the
/// start/end column pair — two compare-and-mask passes `AND`ed, never a
/// new kernel shape (docs/architecture/40-execution.md, § access
/// paths); the Allen kinds take the configuration kernel over the dense
/// stride-1 column pairs (one branchless, flag-free kernel for every
/// mask — `exec/kernel/allen.rs`). A negated occurrence's view rides
/// this same path: its Allen filters classify identically and the probe
/// inverts at the hit, exactly like every other predicate class.
/// Returns whether the scan ran; `false` falls back to the scalar
/// [`row_holds`] loop.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one arm per kernel-shaped predicate kind
fn kernel_scan(
    image: &RelationImage,
    predicate: &FilterPredicate,
    params: &[Const],
    out: &mut Vec<u32>,
) -> bool {
    match predicate {
        FilterPredicate::Compare { .. } => {}
        FilterPredicate::PointIn { field, point } => {
            let (starts, ends) = interval_columns(image, *field);
            crate::exec::kernel::filter_point_in_u64(starts, ends, point_word(point, params), out);
            return true;
        }
        FilterPredicate::AnyPointIn { field, set } => {
            let (starts, ends) = interval_columns(image, *field);
            crate::exec::kernel::filter_any_point_in_u64(starts, ends, word_set(set, params), out);
            return true;
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let (start, end) = const_interval(outer, params);
            let span = image.span(field.field());
            // A scalar field within the constant interval: point
            // membership is the range scan `[start, end - 1]` (the
            // half-open bound; `end >= 1` because `start < end` and
            // word order is value order). Scalar by construction — an
            // interval field under a constant is `FieldAllen`.
            debug_assert_eq!(span.width, ColumnWidth::Word);
            let ColumnView::Words(words) = image.column(usize::from(span.first_column)) else {
                unreachable!("a word span covers a word column")
            };
            crate::exec::kernel::filter_range_u64(words, start, end - 1, out);
            return true;
        }
        // The Allen kinds: dense stride-1 endpoint columns through the
        // configuration kernel — codes via the 8 predicate lanes and the
        // 64-byte `tbl` nibble table, membership via the broadcast mask,
        // survivors via the branchless cursor-write.
        FilterPredicate::FieldsAllen { left, right, mask } => {
            let (l_starts, l_ends) = interval_columns(image, *left);
            let (r_starts, r_ends) = interval_columns(image, *right);
            crate::exec::kernel::allen_filter_columns(
                l_starts,
                l_ends,
                r_starts,
                r_ends,
                mask_of(*mask, params),
                out,
            );
            return true;
        }
        FilterPredicate::FieldAllen { field, other, mask } => {
            let (starts, ends) = interval_columns(image, *field);
            let (start, end) = const_interval(other, params);
            crate::exec::kernel::allen_filter_columns_const(
                starts,
                ends,
                start,
                end,
                mask_of(*mask, params),
                out,
            );
            return true;
        }
        // Same-fact comparisons read two varying columns per position —
        // no constant side, no kernel shape; the scalar loop evaluates
        // them.
        FilterPredicate::FieldsCompare { .. } | FilterPredicate::FieldsPointIn { .. } => {
            return false;
        }
        // The measure kinds never reach the infallible machinery: they
        // evaluate on the fallible refinement pass (`apply`).
        FilterPredicate::DurationCompare { .. } | FilterPredicate::DurationFieldsCompare { .. } => {
            unreachable!("measure filters take the fallible refinement pass")
        }
    }
    let FilterPredicate::Compare { field, op, value } = predicate else {
        unreachable!("every other kind returned above")
    };
    let span = image.span(field.field());
    let value = resolve(value, params);
    if span.width == ColumnWidth::WordPair {
        // Interval value equality (`Eq` on a negated occurrence's view)
        // has no fixed-width scan shape, like scalar `Ne`: scalar loop.
        return false;
    }
    if let ColumnWidth::Words { count } = span.width {
        // A multi-word bytes<N> Eq: the existing fixed-width Eq scan,
        // widened by word count — the first column's kernel pass seeds
        // the survivors, the remaining columns refine them word-wise
        // (no new NEON shapes). `Ne` has no scan shape, like scalar Ne.
        let (Const::Words(words), WordCmp::Eq) = (value, op) else {
            return false;
        };
        debug_assert_eq!(words.len(), usize::from(count), "validated width");
        let first = usize::from(span.first_column);
        let ColumnView::Words(column0) = image.column(first) else {
            unreachable!("a Words span covers word columns")
        };
        crate::exec::kernel::filter_eq_u64(column0, words[0], out);
        for (i, expected) in words.iter().enumerate().skip(1) {
            let ColumnView::Words(column) = image.column(first + i) else {
                unreachable!("a Words span covers word columns")
            };
            let mut cursor = 0usize;
            for read in 0..out.len() {
                let position = out[read] as usize;
                out[cursor] = out[read];
                cursor += usize::from(column[position] == *expected);
            }
            out.truncate(cursor);
        }
        return true;
    }
    match (image.column(usize::from(span.first_column)), value) {
        (ColumnView::Words(words), Const::Word(c)) => {
            let (lo, hi) = match op {
                WordCmp::Eq => {
                    crate::exec::kernel::filter_eq_u64(words, *c, out);
                    return true;
                }
                WordCmp::Lt => {
                    let Some(hi) = c.checked_sub(1) else {
                        out.clear(); // x < 0 over unsigned words: empty
                        return true;
                    };
                    (0, hi)
                }
                WordCmp::Le => (0, *c),
                WordCmp::Gt => {
                    let Some(lo) = c.checked_add(1) else {
                        out.clear(); // x > MAX: empty
                        return true;
                    };
                    (lo, u64::MAX)
                }
                WordCmp::Ge => (*c, u64::MAX),
                WordCmp::Ne => return false,
            };
            crate::exec::kernel::filter_range_u64(words, lo, hi, out);
            true
        }
        (ColumnView::Bytes(bytes), Const::Byte(c)) if *op == WordCmp::Eq => {
            crate::exec::kernel::filter_eq_u8(bytes, *c, out);
            true
        }
        _ => false,
    }
}
