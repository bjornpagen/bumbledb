//! The filter-application mechanism (docs/architecture/30-execution.md):
//! evaluates the per-atom conjunction over a warm image into a
//! survivor-position vector.

use std::sync::Arc;

use crate::image::{ColumnView, ColumnWidth, RelationImage};
use crate::ir::CmpOp;
use crate::schema::FieldId;

use super::{Const, FilterPredicate, ResolvedWordSource, View};

/// Resolves a filter constant through the bind-time param slice: `Param`
/// and `ParamSet` markers index it; everything else is already column form.
fn resolve<'a>(value: &'a Const, params: &'a [Const]) -> &'a Const {
    match value {
        Const::Param(param) | Const::ParamSet(param) => &params[usize::from(param.0)],
        other => other,
    }
}

/// The single column of a scalar field, through its span (the width
/// dispatch every field→column translation runs).
fn scalar_column(image: &RelationImage, field: FieldId) -> ColumnView<'_> {
    image.column(usize::from(image.span(field).first_column))
}

/// An interval field's two column words at one position: `(start, end)`.
///
/// # Panics
///
/// On a programmer-invariant violation: the field's span is not a word
/// pair (validation types every interval predicate over interval fields).
fn interval_at(image: &RelationImage, field: FieldId, position: usize) -> (u64, u64) {
    let span = image.span(field);
    assert_eq!(
        span.width,
        ColumnWidth::WordPair,
        "validated: interval predicates read interval fields"
    );
    let first = usize::from(span.first_column);
    match (image.column(first), image.column(first + 1)) {
        (ColumnView::Words(starts), ColumnView::Words(ends)) => (starts[position], ends[position]),
        _ => unreachable!("an interval span covers two word columns"),
    }
}

/// A scalar word field's column word at one position (interval point
/// operands: always word-typed by validation — interval elements are the
/// orderable scalars).
fn word_at(image: &RelationImage, field: FieldId, position: usize) -> u64 {
    match scalar_column(image, field) {
        ColumnView::Words(words) => words[position],
        ColumnView::Bytes(_) => unreachable!("validated: interval points are word-typed"),
    }
}

/// The resolved point word of a membership filter. A var-sourced point is
/// the executor's slot binding — the point-membership scan
/// (`docs/architecture/40-execution.md`).
fn point_word(point: &ResolvedWordSource, params: &[Const]) -> u64 {
    match point {
        ResolvedWordSource::Word(word) => *word,
        ResolvedWordSource::Param(param) => match &params[usize::from(param.0)] {
            Const::Word(word) => *word,
            _ => unreachable!("validated: a point param resolves to a word"),
        },
        ResolvedWordSource::Var(_) => todo!("todo-by-PRD-17"),
    }
}

/// The resolved word set of a bound set param (sorted, deduplicated).
fn word_set(set: crate::ir::ParamId, params: &[Const]) -> &[u64] {
    match &params[usize::from(set.0)] {
        Const::WordSet(words) => words,
        _ => unreachable!("validated: a set param resolves to a word set"),
    }
}

/// Point membership under the half-open interval: `start ≤ p AND p < end`
/// — `p == start` survives, `p == end` does not.
const fn contains_point(start: u64, end: u64, p: u64) -> bool {
    start <= p && p < end
}

/// Evaluates the conjunction against one image position. `params` is the
/// bind-time resolution slice, indexed by `ParamId`: `Word`/`Byte` for
/// scalar params, `Interval` for interval params, `WordSet` for set
/// params.
fn row_matches(
    image: &RelationImage,
    predicates: &[FilterPredicate],
    params: &[Const],
    position: usize,
) -> bool {
    predicates.iter().all(|predicate| match predicate {
        FilterPredicate::Compare { field, op, value } => {
            match (
                scalar_or_pair(image, *field, position),
                resolve(value, params),
            ) {
                (Operand::Word(word), Const::Word(c)) => op.compare(&word, c),
                (Operand::Byte(byte), Const::Byte(c)) => op.compare(&byte, c),
                // The interval-vs-interval-constant compositions: fixed
                // word comparisons over the (start, end) pair
                // (`docs/architecture/40-execution.md`).
                (Operand::Pair(s, e), Const::Interval { start, end }) => match op {
                    CmpOp::Eq => s == *start && e == *end,
                    CmpOp::Ne => s != *start || e != *end,
                    CmpOp::Overlaps => s < *end && *start < e,
                    CmpOp::Contains => s <= *start && *end <= e,
                    _ => unreachable!("validated: no order comparison over intervals"),
                },
                // A bound set: `Eq` matches any element (validation admits
                // sets under `Eq` only).
                (Operand::Word(word), Const::WordSet(set)) => set.binary_search(&word).is_ok(),
                (Operand::Byte(byte), Const::WordSet(set)) => {
                    set.binary_search(&u64::from(byte)).is_ok()
                }
                // Width mismatches are unrepresentable through validation,
                // and PendingIntern constants are resolved before execution
                // (docs/architecture/30-execution.md) — a miss empties the query without reaching here.
                _ => unreachable!("validated, resolved filter constant"),
            }
        }
        FilterPredicate::FieldsCompare { left, right, op } => {
            match (
                scalar_or_pair(image, *left, position),
                scalar_or_pair(image, *right, position),
            ) {
                (Operand::Word(a), Operand::Word(b)) => op.compare(&a, &b),
                (Operand::Byte(a), Operand::Byte(b)) => op.compare(&a, &b),
                // Interval fields compare pairwise over their two-word
                // spans; validation admits `Eq`/`Ne` only (order operators
                // never apply to intervals).
                (Operand::Pair(a_s, a_e), Operand::Pair(b_s, b_e)) => match op {
                    CmpOp::Eq => a_s == b_s && a_e == b_e,
                    CmpOp::Ne => a_s != b_s || a_e != b_e,
                    _ => unreachable!("validated: no order comparison over intervals"),
                },
                _ => unreachable!("same-fact comparison joins same-typed fields"),
            }
        }
        FilterPredicate::PointIn { field, point } => {
            let (start, end) = interval_at(image, *field, position);
            contains_point(start, end, point_word(point, params))
        }
        FilterPredicate::AnyPointIn { field, set } => {
            let (start, end) = interval_at(image, *field, position);
            // Sorted set: the smallest element ≥ start decides membership.
            let points = word_set(*set, params);
            let idx = points.partition_point(|&p| p < start);
            idx < points.len() && points[idx] < end
        }
        FilterPredicate::FieldsOverlap { left, right } => {
            let (l_start, l_end) = interval_at(image, *left, position);
            let (r_start, r_end) = interval_at(image, *right, position);
            l_start < r_end && r_start < l_end
        }
        FilterPredicate::FieldsContain { outer, inner } => {
            let (o_start, o_end) = interval_at(image, *outer, position);
            let (i_start, i_end) = interval_at(image, *inner, position);
            o_start <= i_start && i_end <= o_end
        }
        FilterPredicate::FieldsContainPoint { interval, point } => {
            let (start, end) = interval_at(image, *interval, position);
            contains_point(start, end, word_at(image, *point, position))
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let Const::Interval { start, end } = resolve(outer, params) else {
                unreachable!("validated: the outer side is an interval constant")
            };
            match scalar_or_pair(image, *field, position) {
                // A scalar field: point membership in the outer interval.
                Operand::Word(word) => contains_point(*start, *end, word),
                // An interval field: point-set containment.
                Operand::Pair(f_start, f_end) => *start <= f_start && f_end <= *end,
                Operand::Byte(_) => unreachable!("validated: within-comparands are word-typed"),
            }
        }
    })
}

/// One field's value at a position, through its span: the scalar word or
/// byte, or an interval field's `(start, end)` word pair.
enum Operand {
    Word(u64),
    Byte(u8),
    Pair(u64, u64),
}

fn scalar_or_pair(image: &RelationImage, field: FieldId, position: usize) -> Operand {
    match image.span(field).width {
        ColumnWidth::WordPair => {
            let (start, end) = interval_at(image, field, position);
            Operand::Pair(start, end)
        }
        ColumnWidth::Word | ColumnWidth::Byte => match scalar_column(image, field) {
            ColumnView::Words(words) => Operand::Word(words[position]),
            ColumnView::Bytes(bytes) => Operand::Byte(bytes[position]),
        },
    }
}

/// Applies the filter conjunction over a (warm) image, writing survivors
/// into `buf` (caller-owned, reused across executions — capacity is
/// retained). An empty predicate list yields the unfiltered [`View::All`].
///
/// # Panics
///
/// Only on programmer-invariant violations: an image beyond the u32
/// position space (the 10⁷ scale axiom sits orders of magnitude below).
#[must_use]
pub fn apply(
    image: &Arc<RelationImage>,
    predicates: &[FilterPredicate],
    params: &[Const],
    mut buf: Vec<u32>,
) -> View {
    if predicates.is_empty() {
        return View::All(Arc::clone(image));
    }
    let row_count = image.row_count();
    debug_assert!(u32::try_from(row_count).is_ok(), "positions fit u32");
    buf.clear();

    // Kernel fast path: the *first kernel-compatible* predicate (not
    // blindly `predicates[0]` — a leading FieldsCompare or byte-column
    // `Ne` must not hide the SIMD path) produces the initial survivor
    // set; every other predicate refines it below.
    if let Some(pivot) = predicates
        .iter()
        .position(|p| kernel_scan(image, p, params, &mut buf))
    {
        let survivors_only = predicates.len() == 1;
        if survivors_only {
            return View::Survivors {
                image: Arc::clone(image),
                positions: buf,
            };
        }
        // Refine in place: evaluate the remaining conjunction per survivor
        // with the branchless cursor write.
        let mut cursor = 0usize;
        for read in 0..buf.len() {
            let position = buf[read] as usize;
            let mut keep = true;
            for (idx, predicate) in predicates.iter().enumerate() {
                if idx == pivot {
                    continue;
                }
                keep &= row_matches(image, std::slice::from_ref(predicate), params, position);
            }
            buf[cursor] = buf[read];
            cursor += usize::from(keep);
        }
        buf.truncate(cursor);
        return View::Survivors {
            image: Arc::clone(image),
            positions: buf,
        };
    }

    buf.resize(row_count, 0);
    let mut cursor = 0usize;
    // The scalar branchless survivor write (D4's compaction pattern):
    // unconditional store, conditional cursor advance — no `if` in this
    // loop body.
    for position in 0..row_count {
        let keep = row_matches(image, predicates, params, position);
        buf[cursor] = u32::try_from(position).expect("checked above");
        cursor += usize::from(keep);
    }
    buf.truncate(cursor);
    View::Survivors {
        image: Arc::clone(image),
        positions: buf,
    }
}

/// Attempts the kernel fast path for one predicate: a compare against a
/// resolved `Word`/`Byte` constant on a plain single column lowers to a
/// fixed-width predicate scan. Returns whether the scan ran. (The interval
/// filter kinds' fused two-column kernels are PRD 17's.)
fn kernel_scan(
    image: &RelationImage,
    predicate: &FilterPredicate,
    params: &[Const],
    out: &mut Vec<u32>,
) -> bool {
    let FilterPredicate::Compare { field, op, value } = predicate else {
        return false;
    };
    let span = image.span(*field);
    if span.width == ColumnWidth::WordPair {
        return false;
    }
    let value = resolve(value, params);
    match (image.column(usize::from(span.first_column)), value) {
        (ColumnView::Words(words), Const::Word(c)) => {
            let (lo, hi) = match op {
                CmpOp::Eq => {
                    crate::exec::kernel::filter_eq_u64(words, *c, out);
                    return true;
                }
                CmpOp::Lt => {
                    let Some(hi) = c.checked_sub(1) else {
                        out.clear(); // x < 0 over unsigned words: empty
                        return true;
                    };
                    (0, hi)
                }
                CmpOp::Le => (0, *c),
                CmpOp::Gt => {
                    let Some(lo) = c.checked_add(1) else {
                        out.clear(); // x > MAX: empty
                        return true;
                    };
                    (lo, u64::MAX)
                }
                CmpOp::Ge => (*c, u64::MAX),
                // `Ne` has no fixed-width scan shape; the interval
                // operators never pair with a single-word constant
                // (normalization emits the interval filter kinds), and
                // their kernels are PRD 17's compositions.
                CmpOp::Ne | CmpOp::Overlaps | CmpOp::Contains => return false,
            };
            crate::exec::kernel::filter_range_u64(words, lo, hi, out);
            true
        }
        (ColumnView::Bytes(bytes), Const::Byte(c)) if *op == CmpOp::Eq => {
            crate::exec::kernel::filter_eq_u8(bytes, *c, out);
            true
        }
        _ => false,
    }
}
