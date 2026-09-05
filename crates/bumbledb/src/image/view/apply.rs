//! The filter-application mechanism:
//! evaluates the per-atom conjunction over a warm image into a
//! survivor-position vector.
use std::sync::Arc;

use crate::error::Result;
use crate::image::{RelationImage, TextEq};

use super::eval::{kernel_scan, row_holds};
use super::{BoundView, Const, FilterPredicate, View};

/// Applies the filter conjunction over a (warm) image, writing survivors
/// into `buf` (caller-owned, reused across executions — capacity is
/// retained). An empty predicate list yields the unfiltered [`View::All`].
/// # Errors
/// Scratch text lookup I/O, work refusal, or corrupt UTF-8 — never
/// rewritten as an empty survivor set.
/// # Panics
/// position space (the 32 GiB map physically bounds live rows roughly an
/// order of magnitude under u32; the validated 10⁷ scale sits far below).
/// Only on programmer-invariant violations: an image beyond the u32
pub fn apply(
    image: &Arc<RelationImage>,
    predicates: &[FilterPredicate],
    params: &[Const],
    buf: Vec<u32>,
    text: TextEq<'_>,
) -> Result<View> {
    apply_resolved(image, predicates, params, buf, text)
}

fn apply_resolved(
    image: &Arc<RelationImage>,
    predicates: &[FilterPredicate],
    params: &[Const],
    mut buf: Vec<u32>,
    text: TextEq<'_>,
) -> Result<View> {
    if predicates.is_empty() {
        return Ok(View::Bound(BoundView::All(Arc::clone(image))));
    }
    let row_count = image.row_count();
    debug_assert!(u32::try_from(row_count).is_ok(), "positions fit u32");
    buf.clear();

    // `Ne` must not hide the SIMD path) produces the initial survivor

    if let Some(pivot) = predicates
        .iter()
        .position(|p| kernel_scan(image, p, params, &mut buf, text))
    {
        let mut cursor = 0usize;
        for read in 0..buf.len() {
            let position = buf[read] as usize;
            let mut keep = true;
            for (idx, predicate) in predicates.iter().enumerate() {
                if idx == pivot {
                    continue;
                }
                keep &= row_holds(
                    image,
                    std::slice::from_ref(predicate),
                    params,
                    position,
                    text,
                )?;
            }
            buf[cursor] = buf[read];
            cursor += usize::from(keep);
        }
        buf.truncate(cursor);
        return Ok(View::Bound(BoundView::Survivors {
            image: Arc::clone(image),
            positions: buf,
        }));
    }

    buf.resize(row_count, 0);
    let mut cursor = 0usize;
    for position in 0..row_count {
        let keep = row_holds(image, predicates, params, position, text)?;
        buf[cursor] = u32::try_from(position).expect("checked above");
        cursor += usize::from(keep);
    }
    buf.truncate(cursor);
    Ok(View::Bound(BoundView::Survivors {
        image: Arc::clone(image),
        positions: buf,
    }))
}
