use super::{
    FjPlan, PlanError, PlanOccurrence, PointProbe, ValidatedPlan,
    check_occurrence_coverage::check_occurrence_coverage, check_selections,
    derive_nodes::derive_nodes, provably_distinct::provably_distinct, split_filters,
};
use crate::image::view::{FilterPredicate, ResolvedWordSource};
use crate::ir::VarId;
use crate::ir::normalize::{NormalizedQuery, Occurrence, Role, SlotWidth};
use crate::schema::{FieldId, Schema};
use std::collections::BTreeSet;

/// The var-sourced membership filters of one lowered occurrence — the
/// `PointIn` filters whose point reads a bound variable. A view is built
/// per execution while a variable binds per join row, so these never
/// reach the filtered view: they execute inside the join
/// ([`PointProbe`] for positive occurrences, the anti-probe's point
/// checks for negated ones).
fn point_filters_of(occurrence: &Occurrence) -> Vec<(FieldId, VarId)> {
    occurrence
        .filters
        .iter()
        .filter_map(|filter| match filter {
            FilterPredicate::PointIn {
                field,
                point: ResolvedWordSource::Var(var),
            } => Some((*field, *var)),
            _ => None,
        })
        .collect()
}

/// Whether a filter is a var-sourced membership (the complement of
/// [`point_filters_of`]'s selection).
fn is_point_filter(filter: &FilterPredicate) -> bool {
    matches!(
        filter,
        FilterPredicate::PointIn {
            point: ResolvedWordSource::Var(_),
            ..
        }
    )
}

/// The execution-facing occurrence table. Trie schemas: a positive
/// occurrence's subatom var-lists in node order (§3.3); a negated
/// occurrence's single probe level — all its variables in binding (slot)
/// order, exactly the shape of a fully-hoisted positive lookup; a
/// grounding-eliminated occurrence's empty schema — no level is ever forced
/// or probed, and its selections and filters are likewise empty so the
/// bind and view paths have nothing to resolve (`plan/ground.rs`). Key
/// widths per level: the sum of the level's variables' slot widths (an
/// interval join variable is one variable with a two-word key). Spans:
/// the relation's field→column map, built once per witness.
fn build_occurrences(
    plan: &FjPlan,
    normalized: &NormalizedQuery,
    schema: &Schema,
    signatures: &[&crate::ir::validate::Predicate],
    slots: &[(VarId, SlotWidth)],
) -> Vec<PlanOccurrence> {
    normalized
        .occurrences
        .iter()
        .map(|occurrence| {
            let trie_schema: Vec<Vec<VarId>> = match occurrence.role {
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
            // The field→column shape: a stored relation's layout, or —
            // for an `Idb` occurrence — the target predicate's sealed
            // signature columns (`FieldId(i)` is head position `i`, the
            // positional reading `lean/Bumbledb/Exec/Fixpoint.lean:
            // tupleFact` promises; the transient image is built with
            // exactly these types, so the spans agree by construction).
            let field_types: Vec<crate::encoding::TypeDesc> = match occurrence.source {
                crate::ir::AtomSource::Edb(relation) => {
                    let layout = schema.relation(relation).layout();
                    (0..layout.field_count())
                        .map(|idx| layout.field_type(idx))
                        .collect()
                }
                crate::ir::AtomSource::Idb(pred) => signatures[usize::from(pred.0)]
                    .columns
                    .iter()
                    .map(|column| column.ty.type_desc())
                    .collect(),
            };
            // A positive occurrence's Eq-constants become selection
            // levels (probes); a negated occurrence keeps its whole
            // filter list — the ordinary filtered view its anti-probe
            // runs against, memoized per (generation, resolved filters)
            // (docs/architecture/40-execution.md, § anti-probe filters).
            // A selection's miss contract — "the whole conjunctive query
            // is empty" — holds for positive occurrences only; an empty
            // negated view just means the anti-probe never rejects. A
            // grounding-eliminated occurrence carries nothing: its filters
            // are implied by the containment and the key, so nothing is
            // resolved, probed, or scanned for it (`plan/ground.rs`). A
            // grounding-FOLDED occurrence keeps its filter list but empties
            // its selections: the filters are introspection's fold picture
            // (`plan/ground/evaluate.rs::folded_picture`) — never
            // resolved, probed, or scanned (`Role::discharged`, read by
            // every execution-side loop).
            let view_filters: Vec<FilterPredicate> = occurrence
                .filters
                .iter()
                .filter(|f| !is_point_filter(f))
                .cloned()
                .collect();
            let (selections, filters) = match occurrence.role {
                Role::Positive => split_filters(&view_filters),
                Role::Negated | Role::Folded(_) => (Vec::new(), view_filters),
                Role::Eliminated(_) => (Vec::new(), Vec::new()),
            };
            PlanOccurrence {
                occ_id: occurrence.occ_id,
                source: occurrence.source,
                role: occurrence.role,
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
/// binding-slot layout, and the optional distinct-bindings witness.
///
/// # Errors
///
/// [`PlanError`] when the plan does not partition the query's
/// participating occurrences, joins a non-participating occurrence,
/// duplicates an occurrence within a node, lacks a cover, or leaves a
/// residual or anti-probe unplaced.
///
/// # Panics
///
/// Only on programmer-invariant violations (more than 256 subatoms in one
/// node — impossible for plans over the planner's occurrence cap — or a
/// normalized query whose slot-width map misses a variable).
/// The query-path entry: the empty `Idb` signature surface (a sealed
/// `ValidatedQuery` carries no `Idb` occurrence). Test observability —
/// production rules route through [`validate_with_signatures`].
#[cfg(test)]
pub fn validate(
    plan: &FjPlan,
    normalized: &NormalizedQuery,
    schema: &Schema,
    estimates: Vec<u64>,
    sink_vars: &BTreeSet<VarId>,
) -> Result<ValidatedPlan, PlanError> {
    validate_with_signatures(plan, normalized, schema, &[], estimates, sink_vars)
}

/// [`validate`] with the program's `Idb` signature surface: an `Idb`
/// occurrence's field→column spans derive from the target predicate's
/// sealed columns (in `PredId` order) instead of a stored relation's
/// layout — everything else is the conjunctive validation, verbatim.
/// The query path passes the empty surface through [`validate`]: a
/// sealed `ValidatedQuery` carries no `Idb` occurrence.
///
/// # Errors
///
/// As [`validate`].
///
/// # Panics
///
/// As [`validate`].
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // the placement rules read in order;
// each attaches one residual kind
pub fn validate_with_signatures(
    plan: &FjPlan,
    normalized: &NormalizedQuery,
    schema: &Schema,
    signatures: &[&crate::ir::validate::Predicate],
    estimates: Vec<u64>,
    sink_vars: &BTreeSet<VarId>,
) -> Result<ValidatedPlan, PlanError> {
    check_occurrence_coverage(plan, normalized)?;
    // Partition property, per participating occurrence: subatom vars are
    // disjoint and union to the occurrence's var set. (Negated and
    // eliminated occurrences appear in no subatom — enforced above.)
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
    // Decomposed point-membership word residuals: the same rule over
    // the word operands' variables.
    for (residual_idx, residual) in normalized.word_residuals.iter().enumerate() {
        let Some(node) = earliest_bound_node(&bound, &[residual.lhs.var, residual.rhs.var]) else {
            return Err(PlanError::UnplacedWordResidual {
                residual: residual_idx,
            });
        };
        nodes[node].word_residuals.push(*residual);
    }
    // Allen residuals: the same rule again — the earliest node binding
    // both interval variables.
    for (residual_idx, residual) in normalized.allen_residuals.iter().enumerate() {
        let Some(node) = earliest_bound_node(&bound, &[residual.lhs, residual.rhs]) else {
            return Err(PlanError::UnplacedAllenResidual {
                residual: residual_idx,
            });
        };
        nodes[node].allen_residuals.push(*residual);
    }
    // Measure residuals: the same rule — the earliest node binding the
    // interval variable and its u64 comparison side.
    for (residual_idx, residual) in normalized.duration_residuals.iter().enumerate() {
        let Some(node) = earliest_bound_node(&bound, &[residual.interval, residual.scalar]) else {
            return Err(PlanError::UnplacedDurationResidual {
                residual: residual_idx,
            });
        };
        nodes[node].duration_residuals.push(*residual);
    }
    // Anti-probe attachment: the earliest node binding the negated
    // occurrence's whole variable set — probe keys plus point-filter
    // variables (a membership check reads its point variable inside the
    // probe, so the probe cannot run before that variable is bound); a
    // zero-variable emptiness gate attaches to the root
    // (docs/architecture/40-execution.md, § anti-probe filters).
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

    // Membership-probe attachment (participating occurrences): the
    // earliest node where every point variable is bound AND the
    // occurrence's trie is fully descended — only then are its remaining
    // positions exactly the facts consistent with the binding, and the
    // existential check `∃ fact: every membership holds` is per-binding
    // correct.
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

    let occurrences = build_occurrences(plan, normalized, schema, signatures, &slots);
    // A tautology at this call site — `split_filters` just constructed
    // these occurrences, so no Eq-constant can sit in `filters`. The real
    // producers `check_selections` checks against are hand-built
    // `PlanOccurrence`s (tests, future callers); the executor-side twin
    // is a debug_assert too. `check_selections` judges participating
    // occurrences only: a negated occurrence's Eq-constants legitimately
    // live in its filter list, a folded occurrence retains its
    // pre-split list for introspection, and an eliminated occurrence's lists
    // are empty (see `build_occurrences`).
    debug_assert!(check_selections(&occurrences).is_ok());

    let distinct_witness = provably_distinct(normalized, schema);
    Ok(ValidatedPlan {
        occurrences,
        nodes,
        slots,
        distinct_witness,
        estimates,
    })
}
