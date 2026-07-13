use super::{FjPlan, PlanError};
use crate::ir::normalize::NormalizedQuery;

/// The occurrence-coverage half of the boundary: every subatom resolves
/// to a **participating** occurrence of this query (an unknown `OccId`
/// would reach the executor as an out-of-range COLT index; a negated or
/// chase-eliminated one would join a node it must never join), and every
/// participating occurrence appears in at least one subatom. The
/// partition check is vacuous for a zero-variable (gate) occurrence —
/// empty seen == empty expected — so the appearance check is what keeps
/// a dropped gate from silently skipping its nonemptiness test (wrong
/// results on a validated plan). Gates are legal only as an empty-vars
/// subatom in some node, exactly what `binary2fj` emits; the
/// all-gates/empty-plan degenerate fails here too. Negated occurrences
/// are covered by anti-probe attachment, never by subatoms; eliminated
/// occurrences are covered by their containment proof (`plan/chase.rs`).
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
