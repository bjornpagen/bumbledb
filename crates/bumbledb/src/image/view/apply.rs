//! The filter-application mechanism (docs/architecture/40-execution.md):
//! evaluates the per-atom conjunction over a warm image into a
//! survivor-position vector.

use std::sync::Arc;

use crate::image::RelationImage;

use super::eval::{kernel_scan, refine_measure, row_holds};
use super::{BoundView, Const, FilterPredicate, View};

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
    if !predicates.iter().any(FilterPredicate::is_measure) {
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
    let (mut view, mut spare) = if predicates.iter().all(FilterPredicate::is_measure) {
        (View::Bound(BoundView::All(Arc::clone(image))), buf)
    } else {
        (apply_infallible(image, predicates, params, buf), Vec::new())
    };
    for predicate in predicates.iter().filter(|p| p.is_measure()) {
        view = refine_measure(image, predicate, params, view, &mut spare);
    }
    view
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
        .position(|p| !p.is_measure() && kernel_scan(image, p, params, &mut buf))
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
                if idx == pivot || predicate.is_measure() {
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
