//! The filter-application mechanism (docs/architecture/30-execution.md):
//! evaluates the per-atom conjunction over a warm image into a
//! survivor-position vector.

use std::sync::Arc;

use crate::image::{ColumnView, RelationImage};
use crate::ir::CmpOp;

use super::{Const, FilterPredicate, View};

/// Evaluates the conjunction against one image position. `params` is the
/// bind-time resolution slice, indexed by `ParamId`, holding `Word`/`Byte`
/// constants only.
fn row_matches(
    image: &RelationImage,
    predicates: &[FilterPredicate],
    params: &[Const],
    position: usize,
) -> bool {
    predicates.iter().all(|predicate| match predicate {
        FilterPredicate::Compare { field, op, value } => {
            let value = match value {
                Const::Param(param) => &params[usize::from(param.0)],
                other => other,
            };
            match (image.column(usize::from(field.0)), value) {
                (ColumnView::Words(words), Const::Word(c)) => op.compare(&words[position], c),
                (ColumnView::Bytes(bytes), Const::Byte(c)) => op.compare(&bytes[position], c),
                // Interval and set constants are evaluable only over the
                // two-word column spans / resolved word sets.
                (_, Const::Interval { .. } | Const::ParamSet(_)) => todo!("todo-by-PRD-14"),
                // Width mismatches are unrepresentable through validation,
                // and PendingIntern constants are resolved before execution
                // (docs/architecture/30-execution.md) — a miss empties the query without reaching here.
                _ => unreachable!("validated, resolved filter constant"),
            }
        }
        FilterPredicate::FieldsCompare { left, right, op } => {
            match (
                image.column(usize::from(left.0)),
                image.column(usize::from(right.0)),
            ) {
                (ColumnView::Words(a), ColumnView::Words(b)) => {
                    op.compare(&a[position], &b[position])
                }
                (ColumnView::Bytes(a), ColumnView::Bytes(b)) => {
                    op.compare(&a[position], &b[position])
                }
                _ => unreachable!("same-fact comparison joins same-typed fields"),
            }
        }
        // The interval filter kinds read two-word column spans (PRD 14's
        // ColumnSpan map) — evaluator arms land there.
        FilterPredicate::PointIn { .. }
        | FilterPredicate::AnyPointIn { .. }
        | FilterPredicate::FieldsOverlap { .. }
        | FilterPredicate::FieldsContain { .. }
        | FilterPredicate::FieldsContainPoint { .. }
        | FilterPredicate::FieldWithin { .. } => todo!("todo-by-PRD-14"),
    })
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
/// resolved `Word`/`Byte` constant on a plain column lowers to a
/// fixed-width predicate scan. Returns whether the scan ran.
fn kernel_scan(
    image: &RelationImage,
    predicate: &FilterPredicate,
    params: &[Const],
    out: &mut Vec<u32>,
) -> bool {
    let FilterPredicate::Compare { field, op, value } = predicate else {
        return false;
    };
    let value = match value {
        Const::Param(param) => &params[usize::from(param.0)],
        other => other,
    };
    match (image.column(usize::from(field.0)), value) {
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
                CmpOp::Ne => return false, // no fixed-width scan shape
                // Interval operators never pair with a single-word
                // constant (normalization emits the interval filter
                // kinds); their kernels are PRD 17's compositions.
                CmpOp::Overlaps | CmpOp::Contains => return false,
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
