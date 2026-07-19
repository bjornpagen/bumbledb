use super::{PlanError, PlanOccurrence};
use crate::image::view::FilterPredicate;
use crate::ir::CmpOp;

/// The selection invariant for **participating** occurrences, asserted
/// at the boundary because [`PlanOccurrence`] is plain data anyone can
/// construct: `filters` may carry an Eq-constant compare only when a
/// measure predicate rides the same list — [`split_filters`] routes
/// every other Eq into `selections`, and pins a measured atom's whole
/// list residual so the filter-order law holds (the Eq runs before the
/// subtraction; `docs/architecture/20-query-ir.md` § the measure).
/// Non-participating occurrences are exempt and skipped here: a negated
/// occurrence's Eq-constants stay in its filter list — the ordinary
/// filtered view the anti-probe runs against
/// (docs/architecture/40-execution.md, § anti-probe filters) — and a
/// grounding-folded occurrence retains its pre-split list purely as
/// introspection's fold picture (`plan/ground/evaluate.rs`), never resolved or
/// scanned.
pub(crate) fn check_selections(occurrences: &[PlanOccurrence]) -> Result<(), PlanError> {
    for occurrence in occurrences {
        if !occurrence.role.participates() {
            continue;
        }
        let measured = occurrence.filters.iter().any(|f| {
            matches!(
                f,
                FilterPredicate::DurationCompare { .. }
                    | FilterPredicate::DurationFieldsCompare { .. }
            )
        });
        let leaked = !measured
            && occurrence
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
