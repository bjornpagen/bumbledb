//! The filter-application mechanism:
//! evaluates the per-atom conjunction over a warm image into a
//! survivor-position vector.

use std::sync::Arc;

use crate::image::RelationImage;

use super::eval::{kernel_scan, row_holds};
use super::{BoundView, Const, FilterPredicate, View};

/// Applies the filter conjunction over a (warm) image, writing survivors
/// into `buf` (caller-owned, reused across executions — capacity is
/// retained). An empty predicate list yields the unfiltered [`View::All`].
/// # Panics
/// position space (the 32 GiB map physically bounds live rows roughly an
/// order of magnitude under u32; the validated 10⁷ scale sits far below).
/// Only on programmer-invariant violations: an image beyond the u32
#[must_use]
pub fn apply(
    image: &Arc<RelationImage>,
    predicates: &[FilterPredicate],
    params: &[Const],
    buf: Vec<u32>,
) -> View {
    apply_infallible(image, predicates, params, buf)
}

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

    // `Ne` must not hide the SIMD path) produces the initial survivor

    if let Some(pivot) = predicates
        .iter()
        .position(|p| kernel_scan(image, p, params, &mut buf))
    {
        let mut cursor = 0usize;
        for read in 0..buf.len() {
            let position = buf[read] as usize;
            let mut keep = true;
            for (idx, predicate) in predicates.iter().enumerate() {
                if idx == pivot {
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
