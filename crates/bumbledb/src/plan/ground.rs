//! Grounding: containment-implied occurrence **elimination** and
//! closed-relation **evaluation**
//! .
//! Two rewrites share one loop. Elimination (below) removes atoms
//! that statements prove redundant; evaluation ([`evaluate`]) removes
//! closed-relation atoms whose extension is stage-0-known by *running
//! plan but a three-element id-set computed before the DP ever sees the
use std::collections::BTreeSet;

use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::{NormalizedQuery, Occurrence, Role, lower_literal};
use crate::ir::{FindTerm, VarId, WordCmp};
use crate::plan::fj::OccBind;
use crate::schema::{Enforcement, Schema};
use bumbledb_theory::schema::{FieldId, Side, StatementId};

pub(crate) mod evaluate;

#[cfg(any(test, feature = "ground-off"))]
thread_local! {

    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// tests' off switch. Restores on unwind.
#[cfg(any(test, feature = "ground-off"))]
pub fn with_grounding_disabled<T>(f: impl FnOnce() -> T) -> T {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            DISABLED.with(|d| d.set(false));
        }
    }
    DISABLED.with(|d| d.set(true));
    let _reset = Reset;
    f()
}

pub(crate) fn ground(normalized: &mut NormalizedQuery, schema: &Schema, finds: &[FindTerm]) {
    #[cfg(any(test, feature = "ground-off"))]
    if DISABLED.with(std::cell::Cell::get) {
        return;
    }
    let output_vars = output_vars(finds);

    let mut support: Vec<Option<usize>> = vec![None; normalized.occurrences.len()];
    loop {
        if let Some((b_idx, a_idx, statement)) =
            removable(normalized, schema, &output_vars, &support)
        {
            normalized.occurrences[b_idx].role = Role::Eliminated(statement);
            support[b_idx] = Some(a_idx);
            continue;
        }
        if evaluate::fold_step(normalized, schema, &output_vars) {
            if normalized.dead.is_some() {
                return;
            }
            continue;
        }
        break;
    }
}

fn removable(
    normalized: &NormalizedQuery,
    schema: &Schema,
    output_vars: &BTreeSet<VarId>,
    support: &[Option<usize>],
) -> Option<(usize, usize, StatementId)> {
    for statement in schema.containments() {
        if !matches!(statement.enforcement, Enforcement::ScalarProbe { .. }) {
            continue;
        }
        let source = &statement.source;
        let target = &statement.target;

        for (b_idx, b) in normalized.occurrences.iter().enumerate() {
            if !b.role.participates() || OccBind::of_occurrence(b) != OccBind::Edb(target.relation)
            {
                continue;
            }
            for (a_idx, a) in normalized.occurrences.iter().enumerate() {
                if a_idx == b_idx
                    || a.role == Role::Negated
                    || OccBind::of_occurrence(a) != OccBind::Edb(source.relation)
                {
                    continue;
                }

                if chain_reaches(support, a_idx, b_idx) {
                    continue;
                }
                if join_covers_full_key(a, b, source, target)
                    && target_otherwise_unused(
                        normalized,
                        b_idx,
                        a_idx,
                        source,
                        target,
                        output_vars,
                    )
                    && variables_join_or_dead(normalized, b_idx, a_idx, source, target, output_vars)
                {
                    return Some((b_idx, a_idx, statement.id));
                }
            }
        }
    }
    None
}

fn join_covers_full_key(a: &Occurrence, b: &Occurrence, source: &Side, target: &Side) -> bool {
    let pairs = || source.projection.iter().zip(target.projection.iter());
    let every_pair_join_covered = pairs().all(|(x, y)| {
        a.vars
            .iter()
            .any(|(f, v)| f == x && b.vars.iter().any(|(g, w)| g == y && w == v))
    });
    let shared_vars_pair_positions_only =
        a.vars
            .iter()
            .all(|(f, v)| match b.vars.iter().find(|(_, w)| w == v) {
                None => true,
                Some((g, _)) => pairs().any(|(x, y)| x == f && y == g),
            });
    every_pair_join_covered && shared_vars_pair_positions_only
}

fn target_otherwise_unused(
    normalized: &NormalizedQuery,
    b_idx: usize,
    a_idx: usize,
    source: &Side,
    target: &Side,
    output_vars: &BTreeSet<VarId>,
) -> bool {
    let b = &normalized.occurrences[b_idx];
    let a = &normalized.occurrences[a_idx];

    let (Some(psi), Some(phi)) = (encoded_selection(target), encoded_selection(source)) else {
        return false;
    };
    let selections_within_psi = b.filters.iter().all(|filter| match filter {
        FilterPredicate::Compare {
            field,
            op: WordCmp::Eq,
            value,
        } => psi.iter().any(|(f, v)| f == field && v == value),
        _ => false,
    });
    let source_carries_phi = phi.iter().all(|(field, value)| {
        a.filters.iter().any(|filter| {
            matches!(
                filter,
                FilterPredicate::Compare { field: f, op: WordCmp::Eq, value: v }
                    if f == field && v == value
            )
        })
    });
    let non_y_fields_unused = b
        .vars
        .iter()
        .filter(|(field, _)| !target.projection.contains(field))
        .all(|(_, var)| var_is_dead(normalized, b_idx, *var, output_vars));
    selections_within_psi && source_carries_phi && non_y_fields_unused
}

fn variables_join_or_dead(
    normalized: &NormalizedQuery,
    b_idx: usize,
    a_idx: usize,
    source: &Side,
    target: &Side,
    output_vars: &BTreeSet<VarId>,
) -> bool {
    let b = &normalized.occurrences[b_idx];
    let a = &normalized.occurrences[a_idx];
    b.vars.iter().all(|(field, var)| {
        let joins = source
            .projection
            .iter()
            .zip(target.projection.iter())
            .any(|(x, y)| y == field && a.vars.iter().any(|(f, v)| f == x && v == var));
        joins || var_is_dead(normalized, b_idx, *var, output_vars)
    })
}

/// **Condition 4** — interval refusal (v0): no paired position is
/// interval-typed.
fn var_is_dead(
    normalized: &NormalizedQuery,
    b_idx: usize,
    var: VarId,
    output_vars: &BTreeSet<VarId>,
) -> bool {
    if output_vars.contains(&var) {
        return false;
    }
    if normalized
        .residuals
        .iter()
        .any(|r| residual_mentions(r, var))
    {
        return false;
    }
    if normalized
        .word_residuals
        .iter()
        .any(|r| residual_mentions(r, var))
    {
        return false;
    }
    if normalized
        .allen_residuals
        .iter()
        .any(|r| residual_mentions(r, var))
    {
        return false;
    }
    if normalized
        .anti_probes
        .iter()
        .any(|p| p.probe_bindings.iter().any(|(_, v)| *v == var))
    {
        return false;
    }
    normalized.occurrences.iter().enumerate().all(|(idx, occ)| {
        idx == b_idx
            || occ.role.discharged()
            || (!occ.vars.iter().any(|(_, v)| *v == var)
                && !occ.point_vars.iter().any(|(_, v)| *v == var))
    })
}

fn residual_mentions(residual: &FilterPredicate, var: VarId) -> bool {
    let (left, right) = match residual {
        FilterPredicate::FieldsCompare { left, right, .. }
        | FilterPredicate::FieldsAllen { left, right, .. } => (*left, *right),
        _ => unreachable!("kind-grouped residual list"),
    };
    left.var() == var || right.var() == var
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Subsumption {
    pub rule: usize,
    pub by: usize,
}

/// Rule subsumption over the grounded query — classical UCQ minimization
/// restricted to the cheap witness the DNF path actually produces: rule K
/// subsumes rule D when, after elimination, K's normalized body equals D's
/// *modulo the filters elimination removed* — identical participating atom
/// multisets with K's conditions ⊆ D's, K's negated atoms within D's, identical
/// head projection. **Refused, the general form:** full CQ-homomorphism
/// minimization is NP-hard — the witness is normalized-form containment and
/// never searches variable mappings (nothing here recurses); `VarId`s must
/// already agree, which is exactly what DNF-cloned rules provide.
pub(crate) fn subsume(rules: &[NormalizedQuery], finds: &[&[FindTerm]]) -> Vec<Subsumption> {
    #[cfg(any(test, feature = "ground-off"))]
    if DISABLED.with(std::cell::Cell::get) {
        return Vec::new();
    }
    let mut deleted = vec![false; rules.len()];
    let mut record = Vec::new();
    for later in 1..rules.len() {
        for earlier in 0..later {
            if deleted[earlier] || deleted[later] {
                continue;
            }

            if rules[earlier].dead.is_some() || rules[later].dead.is_some() {
                continue;
            }
            if subsumes(&rules[earlier], finds[earlier], &rules[later], finds[later]) {
                deleted[later] = true;
                record.push(Subsumption {
                    rule: later,
                    by: earlier,
                });
            } else if subsumes(&rules[later], finds[later], &rules[earlier], finds[earlier]) {
                deleted[earlier] = true;
                record.push(Subsumption {
                    rule: earlier,
                    by: later,
                });
            }
        }
    }
    record.sort_unstable_by_key(|subsumption| subsumption.rule);
    record
}

fn subsumes(
    keeper: &NormalizedQuery,
    keeper_finds: &[FindTerm],
    candidate: &NormalizedQuery,
    candidate_finds: &[FindTerm],
) -> bool {
    keeper_finds == candidate_finds
        && atoms_match(keeper, candidate)
        && subset(&keeper.residuals, &candidate.residuals)
        && subset(&keeper.word_residuals, &candidate.word_residuals)
        && subset(&keeper.allen_residuals, &candidate.allen_residuals)
        && negated_within(keeper, candidate)
}

/// First-fit — a refusal on an ambiguous pairing is only ever conservative (the
/// rule is kept), and the DNF-cloned bodies the witness targets pair
/// index-aligned anyway.
fn atoms_match(keeper: &NormalizedQuery, candidate: &NormalizedQuery) -> bool {
    pairs_off(
        &participating(keeper),
        &participating(candidate),
        |atom, other| {
            atom.source() == other.source()
                && atom.vars == other.vars
                && subset(&atom.filters, &other.filters)
        },
        Matching::Multiset,
    )
}

fn negated_within(keeper: &NormalizedQuery, candidate: &NormalizedQuery) -> bool {
    pairs_off(
        &negated(keeper),
        &negated(candidate),
        |atom, other| {
            atom.source() == other.source()
                && atom.vars == other.vars
                && atom.filters == other.filters
        },
        Matching::Containment,
    )
}

#[derive(Clone, Copy)]
enum Matching {
    Multiset,

    Containment,
}

fn pairs_off(
    from: &[&Occurrence],
    into: &[&Occurrence],
    matches: impl Fn(&Occurrence, &Occurrence) -> bool,
    matching: Matching,
) -> bool {
    if matches!(matching, Matching::Multiset) && from.len() != into.len() {
        return false;
    }
    let mut paired = vec![false; into.len()];
    from.iter().all(|atom| {
        match (0..into.len()).find(|&idx| !paired[idx] && matches(atom, into[idx])) {
            Some(idx) => {
                paired[idx] = true;
                true
            }
            None => false,
        }
    })
}

fn participating(rule: &NormalizedQuery) -> Vec<&Occurrence> {
    rule.occurrences
        .iter()
        .filter(|occurrence| occurrence.role.participates())
        .collect()
}

fn negated(rule: &NormalizedQuery) -> Vec<&Occurrence> {
    rule.occurrences
        .iter()
        .filter(|occurrence| occurrence.role == Role::Negated)
        .collect()
}

fn subset<T: PartialEq>(within: &[T], of: &[T]) -> bool {
    within.iter().all(|item| of.contains(item))
}

fn chain_reaches(support: &[Option<usize>], mut from: usize, target: usize) -> bool {
    while let Some(next) = support[from] {
        if next == target {
            return true;
        }
        from = next;
    }
    false
}

fn encoded_selection(side: &Side) -> Option<Vec<(FieldId, Const)>> {
    side.selection
        .iter()
        .map(|(field, literals)| {
            literals
                .as_equality()
                .map(|value| (*field, lower_literal(value)))
        })
        .collect()
}

fn output_vars(finds: &[FindTerm]) -> BTreeSet<VarId> {
    let mut vars = BTreeSet::new();
    for term in finds {
        match term {
            FindTerm::Var(var) => {
                vars.insert(*var);
            }
            FindTerm::Aggregate { over, .. } | FindTerm::Pack { over } => {
                vars.insert(*over);
            }
            FindTerm::Count => {}
        }
    }
    vars
}

#[cfg(test)]
mod tests;
