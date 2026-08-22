use super::{PlanError, PlanOccurrence};
use crate::image::view::FilterPredicate;
use crate::ir::WordCmp;

/// The selection invariant for **participating** occurrences, asserted at the
/// boundary because [`PlanOccurrence`] is plain data anyone can construct:
/// `filters` may not carry an Eq-constant compare — [`split_filters`] routes
/// every Eq into `selections`.
pub(crate) fn check_selections(occurrences: &[PlanOccurrence]) -> Result<(), PlanError> {
    for occurrence in occurrences {
        if !occurrence.role.participates() {
            continue;
        }
        let leaked = occurrence.filters.iter().any(|f| {
            matches!(
                f,
                FilterPredicate::Compare {
                    op: WordCmp::Eq,
                    ..
                }
            )
        });
        if leaked {
            return Err(PlanError::SelectionOnFilteredField {
                occ: occurrence.occ_id,
            });
        }
    }
    Ok(())
}
