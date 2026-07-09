//! The chase: containment-implied occurrence elimination
//! (docs/architecture/30-dependencies.md, docs/architecture/40-execution.md).
//!
//! An accepted containment `A(X | φ) <= B(Y | ψ)` makes a query's inner
//! join of `A` to `B` on X→Y redundant when `B` contributes nothing else
//! — the rewrite Postgres rejected for decades because deferred
//! constraints make FKs untrustworthy at plan time. This engine deleted
//! the objection: no deferral modes exist and every readable snapshot
//! satisfies every statement (30-dependencies, judged on final states),
//! so the rewrite is always sound here when its conditions hold.
//!
//! Runs after normalization and before statistics and the DP
//! (40-execution planner placement), as a fixpoint: removing one
//! occurrence can make another removable (chains `A<=B<=C` are real),
//! and ≤20 occurrences make the loop trivially cheap. Elimination is a
//! [`Role`] mark, never a removal: occurrence ids never move, and the
//! `Eliminated(StatementId)` mark doubles as the record EXPLAIN and the
//! tests read.
//!
//! # Why it is sound
//!
//! For each binding of the surviving query that satisfies the paired
//! `A` occurrence:
//!
//! - **Existence** — the `A` occurrence carries φ (condition 2, literal
//!   set containment), so its fact is in σφ(A); the containment then
//!   guarantees a ψ-satisfying `B` fact matching on Y.
//! - **Uniqueness** — the acceptance gate (30-dependencies) requires Y
//!   to be a permutation of a declared key of `B`, so at most one `B`
//!   fact matches the Y tuple; combined with existence, **exactly one**.
//!   The eliminated occurrence's own selections are a literal subset of
//!   ψ (condition 2), so that one fact satisfies them, and no other
//!   predicate of the query reads it (conditions 2–3).
//! - **Aggregate safety** — the fold domain is the set of distinct full
//!   bindings over all query variables (20-query-ir, aggregation).
//!   Key-ness of Y makes every non-Y field of the match functionally
//!   determined by the join tuple, so a variable bound only on `B`'s
//!   non-key fields takes exactly one value per surviving binding and
//!   cannot multiply the fold domain; dropping it maps the binding set
//!   1:1.
//!
//! Removal is therefore bit-identical under both sinks — projection and
//! aggregate alike.
//!
//! **Chains and support.** An eliminated occurrence may itself serve as
//! the pairing source of a later elimination: its fact still exists
//! (uniquely, per the above) and satisfies its whole filter list (the
//! list is a subset of the ψ that fact is guaranteed to satisfy). What
//! must never happen is *circular* support — `A == B` eliminating both
//! occurrences would leave the pair resting on nothing — so each
//! elimination records its source and a source qualifies only if its
//! support chain does not pass through the candidate. The support
//! relation is then a forest rooted in participating occurrences, and
//! the existence/uniqueness argument composes by induction from the
//! roots. This also keeps every variable that anything live still reads
//! (an output variable, a residual, a negated probe, a membership
//! point) bound by some participating occurrence: such a variable is
//! never dead, so at each elimination it was join-classified, meaning
//! the source binds it too — walk the support chain to its root.

use std::collections::BTreeSet;

use crate::image::view::{Const, FilterPredicate, ResolvedWordSource};
use crate::ir::normalize::{lower_literal, NormalizedQuery, Occurrence, Role};
use crate::ir::{AggOp, CmpOp, FindTerm, VarId};
use crate::schema::{FieldId, Resolved, Schema, Side, StatementDescriptor, StatementId};

#[cfg(test)]
thread_local! {
    /// The test-only off switch: differential tests run the same query
    /// with and without the rewrite. Crate-internal and
    /// `#[cfg(test)]`-reachable only — no runtime mode ships (no public
    /// API, no env var, no feature flag). Thread-local because the test
    /// harness runs tests concurrently.
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with the chase bypassed on this thread — the differential
/// tests' off switch. Restores on unwind.
#[cfg(test)]
pub(crate) fn with_chase_disabled<T>(f: impl FnOnce() -> T) -> T {
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

/// Marks every provably redundant positive occurrence
/// [`Role::Eliminated`], to a fixpoint. `finds` is the query's find
/// list — the source of the output-variable set condition 2 checks
/// projection against (an aggregate's `over` variable and an Arg key
/// are outputs exactly like a projected variable).
pub(crate) fn chase(normalized: &mut NormalizedQuery, schema: &Schema, finds: &[FindTerm]) {
    #[cfg(test)]
    if DISABLED.with(std::cell::Cell::get) {
        return;
    }
    let output_vars = output_vars(finds);
    // `support[b] = Some(a)`: occurrence `b` was eliminated paired with
    // source occurrence `a`. Edges are only added toward sources whose
    // chain avoids the candidate, so the relation stays a forest rooted
    // in participating occurrences (module doc, chains and support).
    let mut support: Vec<Option<usize>> = vec![None; normalized.occurrences.len()];
    while let Some((b_idx, a_idx, statement)) =
        removable(normalized, schema, &output_vars, &support)
    {
        normalized.occurrences[b_idx].role = Role::Eliminated(statement);
        support[b_idx] = Some(a_idx);
    }
}

/// One fixpoint step: the first `(target, source, statement)` triple
/// satisfying conditions 1–4, in materialized statement order (the
/// deterministic scan the marks' statement ids come from).
fn removable(
    normalized: &NormalizedQuery,
    schema: &Schema,
    output_vars: &BTreeSet<VarId>,
    support: &[Option<usize>],
) -> Option<(usize, usize, StatementId)> {
    for (stmt_idx, statement) in schema.statements().iter().enumerate() {
        let StatementDescriptor::Containment { source, target } = &statement.descriptor else {
            continue;
        };
        if !scalar_positions_only(&statement.resolved) {
            continue; // condition 4
        }
        for b_idx in 0..normalized.occurrences.len() {
            let b = &normalized.occurrences[b_idx];
            if !b.role.participates() || b.relation != target.relation {
                continue;
            }
            for a_idx in 0..normalized.occurrences.len() {
                let a = &normalized.occurrences[a_idx];
                if a_idx == b_idx || a.role == Role::Negated || a.relation != source.relation {
                    continue;
                }
                // Acyclic support: a source resting (transitively) on
                // the candidate would certify the pair with itself.
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
                    return Some((
                        b_idx,
                        a_idx,
                        StatementId(u16::try_from(stmt_idx).expect("statement ids fit u16")),
                    ));
                }
            }
        }
    }
    None
}

/// **Condition 1** — the query joins `A` to `B` exactly on X→Y: every
/// X→Y position pair is join-covered (one variable bound at
/// `source.projection[i]` in `A` and at `target.projection[i]` in `B` —
/// a partial-key join does not qualify, uniqueness needs the whole key,
/// and Y is a whole key by the acceptance gate's `key_permutation`
/// resolution), and every variable shared between the two occurrences
/// pairs a source-projection position with its corresponding target
/// position (a join on any other field pair is a constraint elimination
/// would lose).
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

/// **Condition 2** — `B` is otherwise unused: no `B` field outside Y is
/// projected, filtered, compared in residuals, or referenced by any
/// other occurrence (positive or negated, anti-probe bindings and
/// membership points included — [`var_is_dead`]); `B` carries no
/// selections beyond ψ (a literal subset of ψ is fine); and the `A`
/// occurrence's own filter list contains φ. Both selection checks are
/// (field, encoded literal) set containment — the statement's literals
/// encoded through the very [`lower_literal`] the query's filters came
/// through — never inferred: a param, a range, or any non-`Eq` filter
/// simply fails the containment.
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
    let psi = encoded_selection(target);
    let selections_within_psi = b.filters.iter().all(|filter| match filter {
        FilterPredicate::Compare {
            field,
            op: CmpOp::Eq,
            value,
        } => psi.iter().any(|(f, v)| f == field && v == value),
        _ => false,
    });
    let phi = encoded_selection(source);
    let source_carries_phi = phi.iter().all(|(field, value)| {
        a.filters.iter().any(|filter| {
            matches!(
                filter,
                FilterPredicate::Compare { field: f, op: CmpOp::Eq, value: v }
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

/// **Condition 3** — every variable of `B` is either a join variable
/// (unified with `A`'s at an X→Y position pair) or dead in the sense of
/// condition 2.
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
/// interval-typed. The gate's resolution carries the shared positional
/// index when one exists (`Resolved::Containment::interval_position` —
/// an accepted containment with an interval position always resolves
/// it, 30-dependencies acceptance gate), so `None` *is* the condition.
/// Pointwise coverage is not 1:1 fact-to-fact; the OPEN sub-question
/// rides the doc amendment (trigger: a census query that would
/// benefit).
fn scalar_positions_only(resolved: &Resolved) -> bool {
    matches!(
        resolved,
        Resolved::Containment {
            interval_position: None,
            ..
        }
    )
}

/// Whether `var` is dead outside occurrence `b_idx`: not an output
/// variable, not compared in any residual (whole-value or word), not in
/// any anti-probe's bindings, and neither bound nor read as a
/// membership point by any other non-eliminated occurrence. Eliminated
/// occurrences don't count as references — their reads are already
/// discharged by their own containment proofs, which is what lets
/// chains close in the fixpoint.
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
        .any(|r| r.lhs == var || r.rhs == var)
    {
        return false;
    }
    if normalized
        .word_residuals
        .iter()
        .any(|r| r.lhs.var == var || r.rhs.var == var)
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
            || matches!(occ.role, Role::Eliminated(_))
            || (!occ.vars.iter().any(|(_, v)| *v == var)
                && !occ.filters.iter().any(|filter| {
                    matches!(
                        filter,
                        FilterPredicate::PointIn {
                            point: ResolvedWordSource::Var(v),
                            ..
                        } if *v == var
                    )
                }))
    })
}

/// Whether `from`'s support chain reaches `target`.
fn chain_reaches(support: &[Option<usize>], mut from: usize, target: usize) -> bool {
    while let Some(next) = support[from] {
        if next == target {
            return true;
        }
        from = next;
    }
    false
}

/// A side's selection as the (field, encoded literal) set the query's
/// lowered filters are compared against.
fn encoded_selection(side: &Side) -> Vec<(FieldId, Const)> {
    side.selection
        .iter()
        .map(|(field, value)| (*field, lower_literal(value)))
        .collect()
}

/// The output-variable set: every variable whose value reaches the
/// result — projected finds, aggregate `over` variables, and Arg keys.
/// (Not the D2 `sink_vars` set: under an aggregate that set is every
/// query variable, which gates suffix *skipping*; the chase's
/// aggregate-safety proof is exactly why a dead variable may vanish
/// under a fold — module doc.)
fn output_vars(finds: &[FindTerm]) -> BTreeSet<VarId> {
    let mut vars = BTreeSet::new();
    for term in finds {
        match term {
            FindTerm::Var(var) => {
                vars.insert(*var);
            }
            FindTerm::Aggregate { op, over } => {
                if let Some(var) = over {
                    vars.insert(*var);
                }
                if let AggOp::ArgMax { key } | AggOp::ArgMin { key } = op {
                    vars.insert(*key);
                }
            }
        }
    }
    vars
}

#[cfg(test)]
mod tests;
