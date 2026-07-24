use super::{AllenKeep, MAX_DISTINCT_VARS, OccInfo, OccStats};
use crate::ir::VarId;
use crate::ir::normalize::{NormalizedQuery, Occurrence};
use crate::schema::Schema;

/// Densifies the participating occurrences into bitset form, resolving stats
/// and translating key (`Functionality` statement) projections to
/// variable sets — plus the query's cross-atom `Allen` residuals as
/// [`AllenKeep`] fractions (the one residual class the DP prices, R19).
pub(super) fn densify(
    normalized: &NormalizedQuery,
    occurrences: &[&Occurrence],
    schema: &Schema,
    stats: &[OccStats],
) -> (Vec<OccInfo>, Vec<AllenKeep>) {
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
    // A literal mask's measure is popcount/13 (the JEPD partition); a
    // param mask is unmeasurable at prepare and takes the range class —
    // the ladder's exact constants (`plan/selectivity.rs::allen_keep`).
    let allen: Vec<AllenKeep> = normalized
        .allen_residuals
        .iter()
        .filter_map(|residual| {
            let vars = (1u128 << *var_index.get(&residual.lhs)?)
                | (1u128 << *var_index.get(&residual.rhs)?);
            let (keep_num, keep_den) = match residual.mask {
                crate::ir::MaskTerm::Literal(mask) => (u64::from(mask.popcount()), 13),
                crate::ir::MaskTerm::Param(_) => (1, crate::plan::selectivity::RANGE_KEEP_DEN),
            };
            Some(AllenKeep {
                vars,
                keep_num,
                keep_den,
            })
        })
        .collect();
    let occs = occurrences
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
            // Translate each key's projection to a var bitset. A
            // projection field Eq-pinned to one scalar constant is
            // covered with no variable bit — the shared pinned-field
            // vocabulary ([`crate::plan::pinned_fields`], the
            // distinctness witness's roster): sets pin nothing, and a
            // pointwise key's interval field counts only under a
            // value-typed Eq. Keys with an unbound, un-pinned field are
            // skipped. **The pointwise-key determinant**
            // (docs/architecture/40-execution.md): `occurrence.vars`
            // carries value bindings only — a membership-bound interval
            // field lowered to a filter and never appears here — so a
            // pointwise key contributes its set only when the interval
            // field is bound **by value**. A join binding just the
            // scalar prefix then fails full-set coverage in `estimate`:
            // two facts may share the prefix with disjoint intervals,
            // so prefix agreement certifies no fanout bound.
            // An `Idb` occurrence has no keyed store — no fanout bound
            // flows from key coverage; its rows already sit on the
            // ladder's delta/accumulated floors (`plan/selectivity.rs`).
            let pinned: std::collections::BTreeSet<bumbledb_theory::schema::FieldId> =
                crate::plan::pinned_fields(occurrence).collect();
            let key_var_sets = match occurrence.source.edb() {
                None => Vec::new(),
                Some(stored) => schema
                    .relation(stored)
                    .keys()
                    .iter()
                    .filter_map(|id| {
                        let mut set = 0u128;
                        for field in &schema.key(*id).projection {
                            if pinned.contains(field) {
                                continue;
                            }
                            let (_, var) = occurrence.vars.iter().find(|(f, _)| f == field)?;
                            set |= 1 << var_index[var];
                        }
                        Some(set)
                    })
                    .collect(),
            };
            OccInfo {
                rows,
                vars,
                var_distincts,
                key_var_sets,
            }
        })
        .collect();
    (occs, allen)
}
