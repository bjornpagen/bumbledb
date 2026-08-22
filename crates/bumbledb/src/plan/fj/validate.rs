use super::{
    FjPlan, PlanError, PlanOccurrence, PointProbe, ValidatedPlan,
    check_occurrence_coverage::check_occurrence_coverage, check_selections,
    derive_nodes::derive_nodes, provably_distinct::provably_distinct, split_filters,
};
use crate::ir::VarId;
use crate::ir::normalize::{NormalizedQuery, Occurrence, Role, SlotWidth};
use crate::schema::Schema;
use bumbledb_theory::schema::FieldId;
use std::collections::BTreeSet;

fn point_filters_of(occurrence: &Occurrence) -> Vec<(FieldId, VarId)> {
    occurrence.point_vars.clone()
}

fn build_occurrences(
    plan: &FjPlan,
    normalized: &NormalizedQuery,
    schema: &Schema,
    signatures: &[&crate::ir::validate::Signature],
    slots: &[(VarId, SlotWidth)],
) -> Vec<PlanOccurrence> {
    normalized
        .occurrences
        .iter()
        .map(|occurrence| {
            let trie_schema: Vec<Vec<VarId>> = match &occurrence.role {
                Role::Positive => plan
                    .nodes
                    .iter()
                    .flat_map(|n| n.subatoms.iter())
                    .filter(|s| s.occ == occurrence.occ_id)
                    .map(|s| s.vars.clone())
                    .collect(),
                Role::Negated => {
                    let occ_vars: BTreeSet<VarId> =
                        occurrence.vars.iter().map(|(_, v)| *v).collect();
                    vec![
                        slots
                            .iter()
                            .map(|(v, _)| *v)
                            .filter(|v| occ_vars.contains(v))
                            .collect(),
                    ]
                }
                Role::Eliminated(_) | Role::Folded(_) => Vec::new(),
            };
            let key_widths: Vec<u16> = trie_schema
                .iter()
                .map(|level| {
                    level
                        .iter()
                        .map(|v| {
                            let (_, width) = slots
                                .iter()
                                .find(|(slot_var, _)| slot_var == v)
                                .expect("trie variables are slot-bound");
                            u16::try_from(width.slots()).expect("widths are at most 8 words")
                        })
                        .sum()
                })
                .collect();

            // positional reading `lean/Bumbledb/Query/Denotation.lean:

            let field_types: Vec<bumbledb_theory::schema::ValueType> = match occurrence.source() {
                crate::ir::AtomSource::Edb(relation) => {
                    let layout = schema.relation(relation).layout();
                    (0..layout.field_count())
                        .map(|idx| layout.field_type(idx))
                        .collect()
                }
                crate::ir::AtomSource::Interior(id) => signatures[id.index()]
                    .columns
                    .iter()
                    .map(|column| *column.ty())
                    .collect(),
            };

            // before the subtraction (the filter-order law,

            let view_filters = occurrence.filters.clone();
            let (selections, filters) = match &occurrence.role {
                Role::Positive => split_filters(&view_filters),
                Role::Negated | Role::Folded(_) => (Vec::new(), view_filters),
                Role::Eliminated(_) => (Vec::new(), Vec::new()),
            };
            PlanOccurrence {
                occ_id: occurrence.occ_id,
                role: occurrence.role.clone(),
                bind: occurrence.bind,
                vars: occurrence.vars.clone(),
                selections,
                filters,
                point_filters: point_filters_of(occurrence),
                spans: crate::image::column_spans(&field_types),
                trie_schema,
                key_widths,
            }
        })
        .collect()
}

/// `bound` holds the cumulative bound-variable set after each node; a
/// zero-variable item (an emptiness-gate anti-probe) attaches to the root
/// because the empty set is bound everywhere. The variables are a slice,
/// re-walked in full per node: a single iterator consumed across the `position`
/// steps is exhausted after the first failing node, making every later check
/// vacuously true — the one-node-too- early misattachment the placement
/// regression test pins.
fn earliest_bound_node(bound: &[BTreeSet<VarId>], vars: &[VarId]) -> Option<usize> {
    bound
        .iter()
        .position(|bound_here| vars.iter().all(|v| bound_here.contains(v)))
}

/// Validates a plan against its normalized query, deriving covers,
/// residual/word-residual/anti-probe placement, trie schemas (negated
/// occurrences included), field→column span maps, the two-slot-aware
/// binding-slot layout, and the optional distinct-bindings witness.
/// # Errors
/// [`PlanError`] when the plan does not partition the query's
/// participating occurrences, joins a non-participating occurrence,
/// duplicates an occurrence within a node, lacks a cover, or leaves a
/// residual or anti-probe unplaced.
/// # Panics
/// node — impossible for plans over the planner's occurrence cap — or a
/// normalized query whose slot-width map misses a variable).
/// Test convenience: EDB-only fixtures pass no derived signatures.
/// Production rules route through [`validate_with_signatures`].
/// Only on programmer-invariant violations (more than 256 subatoms in one
#[cfg(test)]
pub fn validate(
    plan: &FjPlan,
    normalized: &NormalizedQuery,
    schema: &Schema,
    sink_vars: &BTreeSet<VarId>,
) -> Result<ValidatedPlan, PlanError> {
    validate_with_signatures(plan, normalized, schema, &[], sink_vars)
}

/// # Errors
/// # Panics
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]

pub fn validate_with_signatures(
    plan: &FjPlan,
    normalized: &NormalizedQuery,
    schema: &Schema,
    signatures: &[&crate::ir::validate::Signature],
    sink_vars: &BTreeSet<VarId>,
) -> Result<ValidatedPlan, PlanError> {
    check_occurrence_coverage(plan, normalized)?;

    for occurrence in &normalized.occurrences {
        if !occurrence.role.participates() {
            continue;
        }
        let mut seen: BTreeSet<VarId> = BTreeSet::new();
        for node in &plan.nodes {
            for subatom in node.subatoms.iter().filter(|s| s.occ == occurrence.occ_id) {
                for var in &subatom.vars {
                    if !seen.insert(*var) {
                        return Err(PlanError::BrokenPartition {
                            occ: occurrence.occ_id,
                        });
                    }
                }
            }
        }
        let expected: BTreeSet<VarId> = occurrence.vars.iter().map(|(_, v)| *v).collect();
        if seen != expected {
            return Err(PlanError::BrokenPartition {
                occ: occurrence.occ_id,
            });
        }
    }

    let mut nodes = derive_nodes(plan)?;
    for node in &mut nodes {
        node.suffix_skip = if node.new_vars.iter().any(|v| sink_vars.contains(v)) {
            super::SuffixSkip::Forbidden
        } else {
            super::SuffixSkip::Licensed
        };
    }

    let bound: Vec<BTreeSet<VarId>> = nodes
        .iter()
        .scan(BTreeSet::new(), |acc, node| {
            acc.extend(node.new_vars.iter().copied());
            Some(acc.clone())
        })
        .collect();

    for (residual_idx, residual) in normalized.residuals.iter().enumerate() {
        let (left, right, _) = residual.compare_sides();
        let Some(node) = earliest_bound_node(&bound, &[left.var(), right.var()]) else {
            return Err(PlanError::UnplacedResidual {
                residual: residual_idx,
            });
        };
        nodes[node].residuals.push(residual.clone());
    }

    for (residual_idx, residual) in normalized.word_residuals.iter().enumerate() {
        let (left, right, _) = residual.compare_sides();
        let Some(node) = earliest_bound_node(&bound, &[left.var(), right.var()]) else {
            return Err(PlanError::UnplacedWordResidual {
                residual: residual_idx,
            });
        };
        nodes[node].word_residuals.push(residual.clone());
    }

    for (residual_idx, residual) in normalized.allen_residuals.iter().enumerate() {
        let (left, right, _) = residual.allen_sides();
        let Some(node) = earliest_bound_node(&bound, &[left.var(), right.var()]) else {
            return Err(PlanError::UnplacedAllenResidual {
                residual: residual_idx,
            });
        };
        nodes[node].allen_residuals.push(residual.clone());
    }

    // probe, so the probe cannot run before that variable is bound); a

    for (probe_idx, anti_probe) in normalized.anti_probes.iter().enumerate() {
        let occurrence = &normalized.occurrences[usize::from(anti_probe.occurrence.0)];
        let vars: Vec<VarId> = anti_probe
            .probe_bindings
            .iter()
            .map(|(_, v)| *v)
            .chain(point_filters_of(occurrence).iter().map(|(_, v)| *v))
            .collect();
        let Some(node) = earliest_bound_node(&bound, &vars) else {
            return Err(PlanError::UnplacedAntiProbe {
                anti_probe: probe_idx,
            });
        };
        nodes[node].anti_probes.push(anti_probe.clone());
    }

    for occurrence in &normalized.occurrences {
        if !occurrence.role.participates() {
            continue;
        }
        let filters = point_filters_of(occurrence);
        if filters.is_empty() {
            continue;
        }
        let vars: Vec<VarId> = filters.iter().map(|(_, v)| *v).collect();
        let Some(var_node) = earliest_bound_node(&bound, &vars) else {
            return Err(PlanError::UnplacedPointProbe {
                occ: occurrence.occ_id,
            });
        };
        let last_subatom_node = nodes
            .iter()
            .rposition(|node| node.subatoms.iter().any(|s| s.occ == occurrence.occ_id))
            .expect("coverage checked: every positive occurrence joins a node");
        nodes[var_node.max(last_subatom_node)]
            .point_probes
            .push(PointProbe {
                occ: occurrence.occ_id,
                filters,
            });
    }

    let width_of = |var: VarId| -> SlotWidth {
        normalized
            .slot_widths
            .get(&var)
            .copied()
            .expect("normalization records every variable's slot width")
    };
    let mut slots: Vec<(VarId, SlotWidth)> = Vec::new();
    for node in &nodes {
        for var in &node.new_vars {
            if !slots.iter().any(|(v, _)| v == var) {
                slots.push((*var, width_of(*var)));
            }
        }
    }

    let occurrences = build_occurrences(plan, normalized, schema, signatures, &slots);

    debug_assert!(check_selections(&occurrences).is_ok());

    let distinctness = match provably_distinct(normalized, schema) {
        Some(witness) => crate::plan::fj::Distinctness::Proven(witness),
        None => crate::plan::fj::Distinctness::Unproven,
    };
    Ok(ValidatedPlan {
        occurrences,
        nodes,
        slots,
        distinctness,
    })
}
