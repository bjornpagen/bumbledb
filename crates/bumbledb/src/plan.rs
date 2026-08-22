//! Statistics, the grounding, the DP planner, and Free Join plan lowering
//! .
pub mod fj;
pub(crate) mod ground;
pub mod planner;
pub(crate) mod selectivity;

use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::Occurrence;
use bumbledb_theory::schema::FieldId;

pub(crate) fn pinned_fields(
    occurrence: &Occurrence,
) -> impl Iterator<Item = (FieldId, &Const)> + '_ {
    occurrence.filters.iter().filter_map(|filter| match filter {
        FilterPredicate::Compare {
            field,
            op: crate::ir::WordCmp::Eq,
            value,
        } if matches!(
            value,
            Const::Word(_)
                | Const::Byte(_)
                | Const::Interval { .. }
                | Const::Param(_)
                | Const::PendingIntern { .. }
        ) =>
        {
            Some((field.field(), value))
        }
        _ => None,
    })
}
