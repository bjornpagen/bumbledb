use super::{Const, FilterPredicate};
use crate::image::RelationImage;

// A separate replay AFTER execution/counter sampling. Never a timing result.
pub(crate) fn seed(image: &RelationImage, filters: &[FilterPredicate]) -> Option<(usize, usize, usize)> {
    let mut positions = Vec::new();
    let params: &[Const] = &[];
    filters.iter().position(|p| super::eval::kernel_scan(image, p, params, &mut positions))
        .map(|pivot| (pivot, positions.len(), positions.capacity()))
}
