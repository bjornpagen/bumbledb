use super::{
    check_occurrence_coverage::check_occurrence_coverage, check_selections,
    derive_nodes::derive_nodes, provably_distinct::provably_distinct, split_filters, FjPlan,
    PlanError, PlanOccurrence, ValidatedPlan,
};
use crate::ir::normalize::{NormalizedQuery, Polarity, SlotWidth};
use crate::ir::VarId;
use crate::schema::Schema;
use std::collections::BTreeSet;

/// The execution-facing occurrence table. Trie schemas: a positive
/// occurrence's subatom var-lists in node order (§3.3); a negated
/// occurrence's single probe level — all its variables in binding (slot)
/// order, exactly the shape of a fully-hoisted positive lookup. Key
/// widths per level: the sum of the level's variables' slot widths (an
/// interval join variable is one variable with a two-word key). Spans:
/// the relation's field→column map, built once per witness.
fn build_occurrences(
    plan: &FjPlan,
    normalized: &NormalizedQuery,
    schema: &Schema,
    slots: &[(VarId, SlotWidth)],
) -> Vec<PlanOccurrence> {
    normalized
        .occurrences
        .iter()
        .map(|occurrence| {
            let trie_schema: Vec<Vec<VarId>> = match occurrence.polarity {
                Polarity::Positive => plan
                    .nodes
                    .iter()
                    .flat_map(|n| n.subatoms.iter())
                    .filter(|s| s.occ == occurrence.occ_id)
                    .map(|s| s.vars.clone())
                    .collect(),
                Polarity::Negated => {
                    let occ_vars: BTreeSet<VarId> =
                        occurrence.vars.iter().map(|(_, v)| *v).collect();
                    vec![slots
                        .iter()
                        .map(|(v, _)| *v)
                        .filter(|v| occ_vars.contains(v))
                        .collect()]
                }
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
                            u16::try_from(width.slots()).expect("width is 1 or 2")
                        })
                        .sum()
                })
                .collect();
            let layout = schema.relation(occurrence.relation).layout();
            let field_types: Vec<crate::encoding::TypeDesc> = (0..layout.field_count())
                .map(|idx| layout.field_type(idx))
                .collect();
            // A positive occurrence's Eq-constants become selection
            // levels (probes); a negated occurrence keeps its whole
            // filter list — the ordinary filtered view its anti-probe
            // runs against, memoized per (generation, resolved filters)
            // (docs/architecture/40-execution.md, § anti-probe filters).
            // A selection's miss contract — "the whole conjunctive query
            // is empty" — holds for positive occurrences only; an empty
            // negated view just means the anti-probe never rejects.
            let (selections, filters) = match occurrence.polarity {
                Polarity::Positive => split_filters(&occurrence.filters),
                Polarity::Negated => (Vec::new(), occurrence.filters.clone()),
            };
            PlanOccurrence {
                occ_id: occurrence.occ_id,
                relation: occurrence.relation,
                vars: occurrence.vars.clone(),
                selections,
                filters,
                spans: crate::image::column_spans(&field_types),
                trie_schema,
                key_widths,
            }
        })
        .collect()
}

/// The shared attachment rule (docs/architecture/40-execution.md):
/// residual comparisons, decomposed word residuals, and anti-probes all
/// attach to the **earliest node at which every variable of the item is
/// bound**. `bound` holds the cumulative bound-variable set after each
/// node; a zero-variable item (an emptiness-gate anti-probe) attaches to
/// the root because the empty set is bound everywhere. The variables are
/// a slice, re-walked in full per node: a single iterator consumed
/// across the `position` steps is exhausted after the first failing
/// node, making every later check vacuously true — the one-node-too-
/// early misattachment the placement regression test pins.
fn earliest_bound_node(bound: &[BTreeSet<VarId>], vars: &[VarId]) -> Option<usize> {
    bound
        .iter()
        .position(|bound_here| vars.iter().all(|v| bound_here.contains(v)))
}

/// Validates a plan against its normalized query, deriving covers,
/// residual/word-residual/anti-probe placement, trie schemas (negated
/// occurrences included), field→column span maps, the two-slot-aware
/// binding-slot layout, and the distinct-bindings flag.
///
/// # Errors
///
/// [`PlanError`] when the plan does not partition the query's positive
/// occurrences, joins a negated occurrence, duplicates an occurrence
/// within a node, lacks a cover, or leaves a residual or anti-probe
/// unplaced.
///
/// # Panics
///
/// Only on programmer-invariant violations (more than 256 subatoms in one
/// node — impossible for plans over the planner's occurrence cap — or a
/// normalized query whose slot-width map misses a variable).
pub fn validate(
    plan: &FjPlan,
    normalized: &NormalizedQuery,
    schema: &Schema,
    estimates: Vec<u64>,
    sink_vars: &BTreeSet<VarId>,
) -> Result<ValidatedPlan, PlanError> {
    check_occurrence_coverage(plan, normalized)?;
    // Partition property, per positive occurrence: subatom vars are
    // disjoint and union to the occurrence's var set. (Negated
    // occurrences appear in no subatom — enforced above.)
    for occurrence in &normalized.occurrences {
        if occurrence.polarity == Polarity::Negated {
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
        node.sink_relevant = node.new_vars.iter().any(|v| sink_vars.contains(v));
    }

    // Cumulative bound-variable sets, once — the one input every
    // attachment below shares.
    let bound: Vec<BTreeSet<VarId>> = nodes
        .iter()
        .scan(BTreeSet::new(), |acc, node| {
            acc.extend(node.new_vars.iter().copied());
            Some(acc.clone())
        })
        .collect();

    // Residual placement: the earliest node at which both sides are bound.
    for (residual_idx, residual) in normalized.residuals.iter().enumerate() {
        let Some(node) = earliest_bound_node(&bound, &[residual.lhs, residual.rhs]) else {
            return Err(PlanError::UnplacedResidual {
                residual: residual_idx,
            });
        };
        nodes[node].residuals.push(*residual);
    }
    // Decomposed interval word residuals: the same rule over the word
    // operands' variables.
    for (residual_idx, residual) in normalized.word_residuals.iter().enumerate() {
        let Some(node) = earliest_bound_node(&bound, &[residual.lhs.var, residual.rhs.var]) else {
            return Err(PlanError::UnplacedWordResidual {
                residual: residual_idx,
            });
        };
        nodes[node].word_residuals.push(*residual);
    }
    // Anti-probe attachment: the earliest node binding the negated
    // occurrence's whole variable set; a zero-variable emptiness gate
    // attaches to the root (docs/architecture/40-execution.md,
    // § anti-probe filters).
    for (probe_idx, anti_probe) in normalized.anti_probes.iter().enumerate() {
        let vars: Vec<VarId> = anti_probe.probe_bindings.iter().map(|(_, v)| *v).collect();
        let Some(node) = earliest_bound_node(&bound, &vars) else {
            return Err(PlanError::UnplacedAntiProbe {
                anti_probe: probe_idx,
            });
        };
        nodes[node].anti_probes.push(anti_probe.clone());
    }

    // Binding-slot layout: node order, then `VarId` order within a node
    // (`new_vars` comes off a `BTreeSet`) — dense, with an interval
    // variable holding two consecutive slots (the [`SlotWidth`] layout,
    // decided at normalization and carried into the witness).
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

    let occurrences = build_occurrences(plan, normalized, schema, &slots);
    // A tautology at this call site — `split_filters` just constructed
    // these occurrences, so no Eq-constant can sit in `filters`. The real
    // producers `check_selections` guards against are hand-built
    // `PlanOccurrence`s (tests, future callers); the executor-side twin
    // is a debug_assert too. Positive occurrences only (they lead the
    // table): a negated occurrence's Eq-constants legitimately live in
    // its filter list (see `build_occurrences`).
    debug_assert!({
        let positive = normalized
            .occurrences
            .iter()
            .filter(|o| o.polarity == Polarity::Positive)
            .count();
        check_selections(&occurrences[..positive]).is_ok()
    });

    let distinct_bindings = provably_distinct(normalized, schema);
    let skip_free = nodes.iter().all(|n| n.sink_relevant);

    Ok(ValidatedPlan {
        occurrences,
        nodes,
        slots,
        distinct_bindings,
        skip_free,
        estimates,
    })
}
