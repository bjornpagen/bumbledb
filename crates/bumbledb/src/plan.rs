//! Statistics, the grounding, the DP planner, and Free Join plan lowering
//! (docs/architecture).

pub mod fj;
pub(crate) mod ground;
pub mod planner;
pub(crate) mod selectivity;

use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::Occurrence;
use bumbledb_theory::schema::FieldId;

/// The fields an occurrence pins **by value**: Eq against one scalar
/// constant (literal word/byte/interval, param, pending intern) — the
/// one pinned-field vocabulary, shared by the distinctness witness
/// (`fj/provably_distinct.rs`) and the DP's key-coverage translation
/// (`planner/densify.rs`) so the two coverage predicates cannot
/// diverge. Sets (`ParamSet`/`WordSet`) pin nothing: an Eq against a
/// set matches any element, so two distinct facts can differ on the
/// field.
pub(crate) fn pinned_fields(occurrence: &Occurrence) -> impl Iterator<Item = FieldId> + '_ {
    occurrence.filters.iter().filter_map(|filter| match filter {
        FilterPredicate::Compare {
            field,
            op: crate::ir::CmpOp::Eq,
            value:
                Const::Word(_)
                | Const::Byte(_)
                | Const::Interval { .. }
                | Const::Param(_)
                | Const::PendingIntern { .. },
        } => Some(*field),
        _ => None,
    })
}
