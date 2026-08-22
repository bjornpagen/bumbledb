use super::{FjPlan, PlanError};
use crate::ir::normalize::NormalizedQuery;

/// The occurrence-coverage half of the boundary: every subatom resolves to a
/// **participating** occurrence of this query (an unknown `OccId` would reach
/// the executor as an out-of-range COLT index; a negated or
/// grounding-eliminated one would join a node it must never join), and every
/// participating occurrence appears in at least one subatom.
pub(super) fn check_occurrence_coverage(
    plan: &FjPlan,
    normalized: &NormalizedQuery,
) -> Result<(), PlanError> {
    for (node_idx, node) in plan.nodes.iter().enumerate() {
        for subatom in &node.subatoms {
            match normalized
                .occurrences
                .iter()
                .find(|o| o.occ_id == subatom.occ)
            {
                None => {
                    return Err(PlanError::UnknownOccurrence {
                        node: node_idx,
                        occ: subatom.occ,
                    });
                }
                Some(occurrence) if !occurrence.role.participates() => {
                    return Err(PlanError::NonParticipatingOccurrenceInNode {
                        node: node_idx,
                        occ: subatom.occ,
                    });
                }
                Some(_) => {}
            }
        }
    }
    for occurrence in &normalized.occurrences {
        if !occurrence.role.participates() {
            continue;
        }
        let appears = plan
            .nodes
            .iter()
            .flat_map(|n| &n.subatoms)
            .any(|s| s.occ == occurrence.occ_id);
        if !appears {
            return Err(PlanError::MissingOccurrence {
                occ: occurrence.occ_id,
            });
        }
    }
    Ok(())
}
