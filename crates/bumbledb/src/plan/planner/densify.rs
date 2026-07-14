use super::{MAX_DISTINCT_VARS, OccInfo, OccStats};
use crate::ir::VarId;
use crate::ir::normalize::Occurrence;
use crate::schema::Schema;

/// Densifies the participating occurrences into bitset form, resolving stats
/// and translating key (`Functionality` statement) projections to
/// variable sets.
pub(super) fn densify(
    occurrences: &[&Occurrence],
    schema: &Schema,
    stats: &[OccStats],
) -> Vec<OccInfo> {
    let mut var_index: std::collections::BTreeMap<VarId, usize> = std::collections::BTreeMap::new();
    for occurrence in occurrences {
        for (_, var) in &occurrence.vars {
            let next = var_index.len();
            var_index.entry(*var).or_insert(next);
        }
    }
    debug_assert!(
        var_index.len() <= MAX_DISTINCT_VARS,
        "validation rejects over-cap queries at the boundary"
    );
    occurrences
        .iter()
        .map(|occurrence| {
            let stat = stats
                .iter()
                .find(|s| s.occ_id == occurrence.occ_id)
                .expect("stats cover every participating occurrence");
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
            // Translate each key's projection to a var bitset; skip keys
            // with any non-var-bound field. **The pointwise-key determinant**
            // (docs/architecture/40-execution.md): `occurrence.vars`
            // carries value bindings only — a membership-bound interval
            // field lowered to a filter and never appears here — so a
            // pointwise key contributes its set only when the interval
            // field is bound **by value**. A join binding just the
            // scalar prefix then fails full-set coverage in `estimate`:
            // two facts may share the prefix with disjoint intervals,
            // so prefix agreement certifies no fanout bound.
            let relation = schema.relation(occurrence.relation);
            let key_var_sets = relation
                .keys()
                .iter()
                .filter_map(|id| {
                    let mut set = 0u128;
                    for field in &schema.key(*id).projection {
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
                key_var_sets,
            }
        })
        .collect()
}
