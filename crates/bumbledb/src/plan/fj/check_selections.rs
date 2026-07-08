use super::{PlanError, PlanOccurrence};
use crate::image::view::FilterPredicate;
use crate::ir::CmpOp;

/// The selection invariant, asserted at the boundary because
/// [`PlanOccurrence`] is plain data anyone can construct: `filters` may
/// never carry an Eq-constant compare — [`split_filters`] routes every
/// one into `selections`.
pub(crate) fn check_selections(occurrences: &[PlanOccurrence]) -> Result<(), PlanError> {
    for occurrence in occurrences {
        let leaked = occurrence
            .filters
            .iter()
            .any(|f| matches!(f, FilterPredicate::Compare { op: CmpOp::Eq, .. }));
        if leaked {
            return Err(PlanError::SelectionOnFilteredField {
                occ: occurrence.occ_id,
            });
        }
    }
    Ok(())
}
