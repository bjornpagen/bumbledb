use super::{AllenKeep, MAX_DISTINCT_VARS, OccInfo, OccStats};
use crate::ir::VarId;
use crate::ir::normalize::{NormalizedQuery, Occurrence};
use crate::plan::fj::OccBind;
use crate::schema::{DistinctnessWitness, Schema};
use bumbledb_theory::schema::{FieldId, RelationId};
use std::collections::BTreeSet;

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

    let allen: Vec<AllenKeep> = normalized
        .allen_residuals
        .iter()
        .filter_map(|residual| {
            let (left, right, mask) = residual.allen_sides();
            let vars = (1u128 << *var_index.get(&left.var())?)
                | (1u128 << *var_index.get(&right.var())?);
            let (keep_num, keep_den) = (u64::from(mask.popcount()), 13);
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

            let pinned: std::collections::BTreeSet<bumbledb_theory::schema::FieldId> =
                crate::plan::pinned_fields(occurrence)
                    .map(|(field, _)| field)
                    .collect();
            let key_var_sets = match OccBind::of_occurrence(occurrence) {
                OccBind::Finished(_) | OccBind::RecDelta(_) | OccBind::RecAcc(_) => Vec::new(),
                OccBind::Edb(stored) => compiled_scalar_key_var_sets(
                    schema,
                    stored,
                    occurrence,
                    &pinned,
                    &var_index,
                ),
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

/// Join-step key sets come from interned [`DistinctnessWitness::ScalarKeyUnique`]
/// projections only. Pointwise keys are full-row equality, not a scalar
/// uniqueness premise (L01).
fn compiled_scalar_key_var_sets(
    schema: &Schema,
    stored: RelationId,
    occurrence: &Occurrence,
    pinned: &BTreeSet<FieldId>,
    var_index: &std::collections::BTreeMap<VarId, usize>,
) -> Vec<u128> {
    let Ok(theory) = schema.compiled_theory() else {
        return Vec::new();
    };
    theory
        .key_projections_of(stored)
        .iter()
        .filter_map(|id| {
            match theory.distinctness_witness(*id)? {
                DistinctnessWitness::ScalarKeyUnique { .. } => {}
                DistinctnessWitness::FullRowEquality
                | DistinctnessWitness::ExistenceOnly { .. } => return None,
            }
            let projection = theory.projection(*id)?;
            let mut set = 0u128;
            for field in projection.projection.iter() {
                if pinned.contains(field) {
                    continue;
                }
                let (_, var) = occurrence.vars.iter().find(|(f, _)| f == field)?;
                set |= 1 << var_index[var];
            }
            Some(set)
        })
        .collect()
}
