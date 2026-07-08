use super::{OccInfo, OccStats, MAX_DISTINCT_VARS};
use crate::ir::normalize::NormalizedQuery;
use crate::ir::VarId;
use crate::schema::Schema;

/// Densifies occurrences into bitset form, resolving stats and translating
/// unique-constraint field sets to variable sets.
pub(super) fn densify(
    normalized: &NormalizedQuery,
    schema: &Schema,
    stats: &[OccStats],
) -> Vec<OccInfo> {
    let mut var_index: std::collections::BTreeMap<VarId, usize> = std::collections::BTreeMap::new();
    for occurrence in &normalized.occurrences {
        for (_, var) in &occurrence.vars {
            let next = var_index.len();
            var_index.entry(*var).or_insert(next);
        }
    }
    debug_assert!(
        var_index.len() <= MAX_DISTINCT_VARS,
        "validation rejects over-cap queries at the boundary"
    );
    normalized
        .occurrences
        .iter()
        .map(|occurrence| {
            let stat = stats
                .iter()
                .find(|s| s.occ_id == occurrence.occ_id)
                .expect("stats cover every occurrence");
            let rows = stat.rows;
            let mut vars = 0u128;
            for (_, var) in &occurrence.vars {
                vars |= 1 << var_index[var];
            }
            let var_distincts: Vec<(u128, u64)> = stat
                .var_distincts
                .iter()
                .map(|(var, distinct)| (1u128 << var_index[var], *distinct))
                .collect();
            // Translate each unique constraint's field set to a var bitset;
            // skip constraints with any non-var-bound field.
            let relation = schema.relation(occurrence.relation);
            let unique_var_sets = relation
                .unique_constraints()
                .iter()
                .filter_map(|cid| {
                    let mut set = 0u128;
                    for field in relation.constraint(*cid).fields() {
                        let (_, var) = occurrence.vars.iter().find(|(f, _)| f == field)?;
                        set |= 1 << var_index[var];
                    }
                    Some(set)
                })
                .collect();
            OccInfo {
                rows,
                vars,
                var_distincts,
                unique_var_sets,
            }
        })
        .collect()
}
