//! The grounding-evaluator: folding stage-zero atoms
//! (docs/architecture/40-execution.md, § the ground: elimination and
//! evaluation).
//!
//! A closed relation's extension is sealed at validate — stage-0 data
//! (`docs/architecture/40-execution.md` § the staging law). A query atom over
//! it whose filters are prepare-resolvable constants is therefore not a
//! join to plan: the evaluator runs the filters against the sealed rows
//! **at prepare**, producing the surviving id-set `S`, and the atom's
//! whole contribution becomes a plan-constant membership on its
//! siblings — `Const::WordSet` riding exactly the param-set selection
//! machinery (`plan/fj/split_filters.rs` routes the Eq into a
//! set-bound selection level, probed once per element with the
//! survivor union — the machinery makes exactly the choices it makes
//! for a bound param set today; **nothing new executes**).
//!
//! # Foldability (positive occurrence `C`, all strict — any failure
//! # leaves the virtual-image join, which is cheap and always correct)
//!
//! 1. Every variable bound by `C` except at most one is dead outside
//!    `C` ([`super::var_is_dead`]); the at-most-one live variable is
//!    bound at `C`'s id position `FieldId(0)` — the join variable `k`,
//!    and some *other* participating occurrence binds `k` (the
//!    membership needs a home). **What does NOT fold, deliberately**: a
//!    closed atom with a live non-id variable — payload escaping to the
//!    head ("return each event's severity rank") keeps its join against
//!    the L1-resident, generation-immortal virtual image. Folding
//!    payload projection would require value substitution into the head
//!    — a rewrite class with real complexity and no measured need.
//!    REFUSED, recorded; trigger: the calendar family showing
//!    vocabulary-join cost above noise.
//! 2. `C` carries only Eq/range/Allen/membership filters over its own
//!    columns with prepare-resolvable constants
//!    ([`crate::image::view::is_prepare_resolvable`]). A param-bearing filter REFUSES
//!    the fold in v0 (a bind-time fold variant is refused, recorded;
//!    trigger: a measured win in the calendar-family profile); measure
//!    filters refuse too (their ray error is raised per execution — a
//!    prepare-time evaluation would move the error to `prepare`, an
//!    observable timing change for zero measured need).
//! 3. `C` is not negated — negated closed atoms fold to the COMPLEMENT
//!    (below).
//!
//! # The fold
//!
//! - `k` live and `|S| ≥ 1`: mark `C` [`Role::Folded`] and attach `S`
//!   to every other participating occurrence binding `k` as an
//!   `Eq`-`WordSet` membership filter.
//! - `|S| == 0`: the rule is statically empty — the fold's rule-death
//!   channel ([`NormalizedQuery::dead`], rendered `folded to ∅: …`);
//!   the pipeline runs fold then ground, so the evaluator writes the
//!   verdict itself rather than routing a set back through the fold.
//! - No live `k` (a pure constant gate, e.g. a nonemptiness check over
//!   a ψ-subset): `|S| ≥ 1` deletes the atom outright; `|S| == 0` kills
//!   the rule. The gate must bind **no variables at all**: a dead-but-
//!   bound variable still multiplies an aggregate's fold domain (the
//!   binding set is over ALL query variables — 40-execution, D2), so a
//!   var-binding gate is REFUSED, recorded; trigger: a measured
//!   projection-sink-only win.
//!
//! The fold mark carries the σ-survivors (n ≤ 256) and polarity as a
//! sum; introspection reads the mark. The rendered picture always uses
//! the retained original filters so diagnostics preserve the user's
//! spelling.
//!
//! # Negated closed atoms — the complement fold, direction pinned
//!
//! `!Kind(id: k, mastered == true)` rejects a binding iff its `k`
//! matches a σ-surviving fact, i.e. iff `k ∈ S` (id is the whole key).
//!
//! - `|S| == 0`: the anti-probe **rejects nothing** — the atom deletes
//!   outright, no membership attached, the rule is NOT empty. (This
//!   direction needs no domain reasoning: `k ∉ ∅` holds for every `k`.)
//! - `0 < |S| < |extension|`: `k ∉ S` rewrites to `k ∈ complement`
//!   (extension ids minus `S`) — attached exactly like a positive fold.
//!   **Sound only under the domain guarantee** ([`domain_within_ids`]):
//!   `k ∉ S ⟺ k ∈ complement` requires `k ∈ extension ids`; a `k`
//!   outside the extension survives the anti-probe but would fail the
//!   complement membership. The guarantee's two witnesses: `k` is bound
//!   at the id position of another participating occurrence of the same
//!   closed relation, or a binder's field carries an accepted
//!   containment into the closed relation's id (with the statement's φ
//!   carried literally by that occurrence — every committed value is
//!   then inside the compiled closed-target member set).
//!   No witness → REFUSED, recorded (the anti-probe stays; trigger: a
//!   profiled anti-probe worth folding under a richer domain analysis).
//! - complement empty (`S` = the whole extension): under the same
//!   guarantee every binding's `k` is rejected — the rule is dead.
//! - A zero-binding negated gate (`!Kind(mastered == true)`): `|S| ≥ 1`
//!   rejects every binding — rule dead; `|S| == 0` deletes (above).

use std::collections::BTreeSet;

use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::{FoldedMark, NormalizedQuery, Role};
use crate::ir::{VarId, WordCmp};
use crate::plan::fj::OccBind;
use crate::schema::{Relation, Schema};
use bumbledb_theory::schema::{FieldId, RelationId};

pub(crate) use crate::image::view::push_handle;

use super::var_is_dead;

/// One evaluator step of the grounding loop: finds the first foldable
/// occurrence, applies its fold (mark + membership attachment, outright
/// deletion, or the rule-death verdict) and reports whether anything
/// changed. One action per call — the caller's loop re-runs elimination
/// between folds (each rewrite can expose the other).
pub(super) fn fold_step(
    normalized: &mut NormalizedQuery,
    schema: &Schema,
    output_vars: &BTreeSet<VarId>,
) -> bool {
    for c_idx in 0..normalized.occurrences.len() {
        let folded = match &normalized.occurrences[c_idx].role {
            Role::Positive => fold_positive(normalized, schema, output_vars, c_idx),
            Role::Negated => fold_negated(normalized, schema, c_idx),
            Role::Eliminated(_) | Role::Folded(_) => false,
        };
        if folded {
            return true;
        }
    }
    false
}

/// One positive occurrence's fold attempt (module doc, conditions 1–2).
fn fold_positive(
    normalized: &mut NormalizedQuery,
    schema: &Schema,
    output_vars: &BTreeSet<VarId>,
    c_idx: usize,
) -> bool {
    let occurrence = &normalized.occurrences[c_idx];
    // THE GUARD (20-query-ir.md § engine recursion's consumer guards): sealed
    // extensions exist only for closed stored relations, so an `Interior`
    // occurrence has no stage-0 rows and never folds.
    let OccBind::Edb(relation_id) = OccBind::of_occurrence(occurrence) else {
        return false;
    };
    let relation = schema.relation(relation_id);
    if relation.body().closed_rows().is_none() {
        return false; // ordinary relations have no stage-0 rows
    }
    if !occurrence
        .filters
        .iter()
        .all(crate::image::view::is_prepare_resolvable)
    {
        return false; // condition 2 refusal (params, measures)
    }
    if payload_escapes(normalized, c_idx, output_vars) {
        return false; // condition 1 refusal: the payload projection keeps its join
    }
    let binders = if let Some(k) = join_id_var(normalized, c_idx, output_vars) {
        let binders = membership_binders(normalized, c_idx, k);
        if binders.is_empty() {
            // A live join variable with no other participating binder:
            // deleting C would leave `k` unbound (a projected handle
            // enumerating the extension, or a residual/anti-probe
            // read) — the membership has no home. The single-atom
            // closed scan stays; it is one L1-resident image.
            return false;
        }
        binders
    } else {
        // The pure-gate shape: only a var-less atom may delete — a
        // dead-but-bound variable still multiplies an aggregate's fold
        // domain (module doc), and the gate's truth must survive
        // without it.
        if !normalized.occurrences[c_idx].vars.is_empty() {
            return false;
        }
        // Deleting the last participating occurrence would leave the
        // rule bodyless — a plan shape nothing downstream represents.
        // The single-atom gate keeps its scan.
        if !normalized
            .occurrences
            .iter()
            .enumerate()
            .any(|(idx, occ)| idx != c_idx && occ.role.participates())
        {
            return false;
        }
        Vec::new()
    };
    let survivors = surviving_ids(relation, &normalized.occurrences[c_idx].filters);
    if survivors.is_empty() {
        // The rule-death channel (module doc): σ over the sealed rows
        // is empty, so the atom — and with it the conjunction — denotes
        // nothing on ANY store.
        normalized.dead = Some(format!(
            "folded to ∅: {}",
            folded_picture(schema, relation_id, &normalized.occurrences[c_idx].filters,)
        ));
        return true;
    }
    attach_membership(normalized, &binders, &survivors);
    normalized.occurrences[c_idx].role = Role::Folded(folded_positive(relation_id, survivors));
    true
}

/// One negated occurrence's fold attempt (module doc, the complement
/// fold — direction pinned there and by the tests).
fn fold_negated(normalized: &mut NormalizedQuery, schema: &Schema, c_idx: usize) -> bool {
    let occurrence = &normalized.occurrences[c_idx];
    // The positive fold's Interior guard, verbatim: no sealed extension,
    // no stage-0 rows, no fold (20-query-ir.md § engine recursion's consumer guards).
    let OccBind::Edb(relation_id) = OccBind::of_occurrence(occurrence) else {
        return false;
    };
    let relation = schema.relation(relation_id);
    let Some(rows) = relation.body().closed_rows() else {
        return false;
    };
    if !occurrence
        .filters
        .iter()
        .all(crate::image::view::is_prepare_resolvable)
    {
        return false;
    }
    let survivors = surviving_ids(relation, &normalized.occurrences[c_idx].filters);
    if survivors.is_empty() {
        // No fact can ever match the probe's filters: the anti-probe
        // never rejects, whatever the bindings — the atom deletes
        // outright (and the rule is NOT empty). Any binding shape
        // qualifies: emptiness of σ needs no key reasoning.
        remove_anti_probe(normalized, c_idx);
        normalized.occurrences[c_idx].role = Role::Folded(folded_negated(relation_id, Vec::new()));
        return true;
    }
    if occurrence.vars.is_empty() {
        // The negated gate: some sealed row satisfies the filters on
        // every store, so the probe rejects every binding — rule dead.
        normalized.dead = Some(format!(
            "folded: !{} rejects every binding",
            folded_picture(schema, relation_id, &occurrence.filters)
        ));
        return true;
    }
    // The keyed shape: exactly one variable, at the id position — the
    // probe is then precisely `k ∈ S`. A payload-bound probe key would
    // need multi-column set reasoning; REFUSED v0, recorded (trigger: a
    // profiled multi-key anti-probe on a closed relation).
    let &[(FieldId(0), k)] = occurrence.vars.as_slice() else {
        return false;
    };
    let closed = relation_id;
    let binders = membership_binders(normalized, c_idx, k);
    if binders.is_empty() {
        return false; // the complement membership needs a home
    }
    if !domain_within_ids(normalized, schema, c_idx, k, closed) {
        // Without the domain guarantee, `k ∉ S` and `k ∈ complement`
        // disagree on out-of-extension values (module doc — the
        // direction this refusal pins). The anti-probe stays.
        return false;
    }
    let extension_len = u64::try_from(rows.len()).expect("extensions cap at 256 rows");
    let complement: Vec<u64> = (0..extension_len)
        .filter(|id| survivors.binary_search(id).is_err())
        .collect();
    if complement.is_empty() {
        // S is the whole extension: with `k` domain-guaranteed inside
        // it, the probe rejects every binding — rule dead.
        normalized.dead = Some(format!(
            "folded: !{} rejects every binding",
            folded_picture(schema, closed, &normalized.occurrences[c_idx].filters)
        ));
        return true;
    }
    let mark = folded_negated(closed, survivors);
    attach_membership(normalized, &binders, &complement);
    remove_anti_probe(normalized, c_idx);
    normalized.occurrences[c_idx].role = Role::Folded(mark);
    true
}

fn assert_fold_cap(survivors: &[u64]) {
    assert!(survivors.len() <= 256, "extensions cap at 256 rows");
}

fn folded_positive(relation: RelationId, survivors: Vec<u64>) -> FoldedMark {
    assert_fold_cap(&survivors);
    FoldedMark::Positive {
        relation,
        survivors: survivors.into_boxed_slice(),
    }
}

fn folded_negated(relation: RelationId, survivors: Vec<u64>) -> FoldedMark {
    assert_fold_cap(&survivors);
    FoldedMark::Negated {
        relation,
        survivors: survivors.into_boxed_slice(),
    }
}

// The foldability conditions, one named predicate each (the grounding
// conditions' naming discipline — `join_covers_full_key`,
// `target_otherwise_unused`); each unit-tested in isolation (tests.rs).

/// **Condition 1 (refusal half)** — whether any non-id variable of
/// `c_idx` is live outside it: a payload variable escaping to the head,
/// another occurrence, or a residual/anti-probe/membership-point read.
pub(super) fn payload_escapes(
    normalized: &NormalizedQuery,
    c_idx: usize,
    output_vars: &BTreeSet<VarId>,
) -> bool {
    normalized.occurrences[c_idx]
        .vars
        .iter()
        .any(|(field, var)| {
            *field != FieldId(0) && !var_is_dead(normalized, c_idx, *var, output_vars)
        })
}

/// **Condition 1 (join half)** — the occurrence's live join variable:
/// the variable bound at the id position `FieldId(0)`, if it is live
/// outside the occurrence. A dead id variable is no join (the atom is
/// then a gate candidate — and a var-binding gate refuses, module
/// doc).
pub(super) fn join_id_var(
    normalized: &NormalizedQuery,
    c_idx: usize,
    output_vars: &BTreeSet<VarId>,
) -> Option<VarId> {
    normalized.occurrences[c_idx]
        .vars
        .iter()
        .find(|(field, _)| *field == FieldId(0))
        .map(|(_, var)| *var)
        .filter(|var| !var_is_dead(normalized, c_idx, *var, output_vars))
}

/// One sealed extension row as an operand source. Prepare-resolvable
/// filters never intern; a corrupt fixed-interval start is unreachable
/// on a validation-admitted extension.
struct SealedRow<'a> {
    fact: crate::encoding::FactView<'a, 'a>,
}

impl crate::image::view::Operands for SealedRow<'_> {
    type Error = std::convert::Infallible;

    fn word(&self, at: crate::image::view::OperandAddr) -> Result<u64, Self::Error> {
        Ok(match self.loaded(at)? {
            crate::image::view::Loaded::Word(w) => w,
            crate::image::view::Loaded::Byte(b) => u64::from(b),
            crate::image::view::Loaded::Pair(..) | crate::image::view::Loaded::Block { .. } => {
                unreachable!("validated: word operands are scalar")
            }
        })
    }

    fn pair(&self, at: crate::image::view::OperandAddr) -> Result<(u64, u64), Self::Error> {
        Ok(match self.loaded(at)? {
            crate::image::view::Loaded::Pair(s, e) => (s, e),
            crate::image::view::Loaded::Word(_)
            | crate::image::view::Loaded::Byte(_)
            | crate::image::view::Loaded::Block { .. } => {
                unreachable!("validated: interval predicates read interval fields")
            }
        })
    }

    fn block(&self, at: crate::image::view::OperandAddr) -> Result<([u64; 8], u8), Self::Error> {
        Ok(match self.loaded(at)? {
            crate::image::view::Loaded::Block { words, count } => (words, count),
            _ => unreachable!("validated: block operands are bytes<N>"),
        })
    }

    fn loaded(
        &self,
        at: crate::image::view::OperandAddr,
    ) -> Result<crate::image::view::Loaded, Self::Error> {
        use crate::exec::dispatch::{fact_operand, FactOperand};
        Ok(
            match fact_operand(self.fact, at.field()).expect("sealed rows are valid") {
                FactOperand::Word(w) => crate::image::view::Loaded::Word(w),
                FactOperand::Pair(s, e) => crate::image::view::Loaded::Pair(s, e),
                FactOperand::Block { words, count } => {
                    crate::image::view::Loaded::Block { words, count }
                }
            },
        )
    }
}

/// The prepare-time evaluation: σ(filters) over the sealed extension
/// rows, as the ascending surviving row-id list (row id = declaration
/// index — `schema.rs`, `SealedRow`). n ≤ 256 rows through the shared
/// predicate walk. Callers have already proved [`is_prepare_resolvable`].
/// Crate-visible for the introspection surface (`exec/introspection/into_stats.rs`).
pub(crate) fn surviving_ids(relation: &Relation, filters: &[FilterPredicate]) -> Vec<u64> {
    let layout = relation.layout();
    relation
        .body()
        .closed_rows()
        .expect("callers checked closedness")
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            let ops = SealedRow {
                fact: layout.encoded(&row.fact),
            };
            filters.iter().all(|filter| {
                crate::image::view::holds(filter, &ops, &[])
                    .unwrap_or_else(|e| match e {})
                    .unwrap_or(false)
            })
        })
        .map(|(id, _)| id as u64)
        .collect()
}

/// The participating occurrences (other than `c_idx`) binding `var`,
/// with the field each binds it at — the membership set's homes. Never
/// a negated occurrence: attaching a positive membership inside an
/// anti-probe would weaken its rejection.
pub(super) fn membership_binders(
    normalized: &NormalizedQuery,
    c_idx: usize,
    var: VarId,
) -> Vec<(usize, FieldId)> {
    normalized
        .occurrences
        .iter()
        .enumerate()
        .filter(|(idx, occ)| *idx != c_idx && occ.role.participates())
        .filter_map(|(idx, occ)| {
            occ.vars
                .iter()
                .find(|(_, v)| *v == var)
                .map(|(field, _)| (idx, *field))
        })
        .collect()
}

/// **The complement fold's domain guarantee** — whether `k`'s values
/// are provably within the closed relation's extension ids. Two
/// witnesses (module doc): a participating occurrence binding `k` at
/// the id position of the same closed relation, or one binding `k` at a
/// field whose accepted containment targets the closed relation's id —
/// with the statement's source selection φ carried **literally** by
/// that occurrence (the elimination pass's condition-2 discipline: set
/// containment over (field, encoded literal), never inference).
pub(super) fn domain_within_ids(
    normalized: &NormalizedQuery,
    schema: &Schema,
    c_idx: usize,
    k: VarId,
    closed: RelationId,
) -> bool {
    normalized
        .occurrences
        .iter()
        .enumerate()
        .filter(|(idx, occ)| *idx != c_idx && occ.role.participates())
        .any(|(_, occ)| {
            occ.vars.iter().any(|(field, var)| {
                *var == k
                    && ((OccBind::of_occurrence(occ) == OccBind::Edb(closed)
                        && *field == FieldId(0))
                        || containment_into_id(schema, occ, *field, closed))
            })
        })
}

/// Whether some accepted containment maps `(occurrence.relation, field)`
/// into `closed`'s id position, with its φ carried literally by the
/// occurrence. Any ψ only shrinks the member set — still inside the
/// extension ids, which is all the domain guarantee needs.
fn containment_into_id(
    schema: &Schema,
    occurrence: &crate::ir::normalize::Occurrence,
    field: FieldId,
    closed: RelationId,
) -> bool {
    schema.containments().iter().any(|statement| {
        OccBind::of_occurrence(occurrence) == OccBind::Edb(statement.source.relation)
            && statement.source.projection.as_ref() == [field]
            && statement.target.relation == closed
            && statement.target.projection.as_ref() == [FieldId(0)]
            && super::encoded_selection(&statement.source).is_some_and(|phi| {
                phi.iter().all(|(f, value)| {
                    occurrence.filters.iter().any(|filter| {
                        matches!(
                            filter,
                            FilterPredicate::Compare { field: ff, op: WordCmp::Eq, value: v }
                                if ff == f && v == value
                        )
                    })
                })
            })
    })
}

/// Attaches the plan-constant membership to every binder: one
/// `Eq`-`WordSet` compare per (occurrence, field) — the exact shape
/// `split_filters` routes into a set-bound selection level, so the set
/// rides the param-set machinery verbatim (probed once per element
/// with the survivor union — the machinery's own choices, nothing new
/// executes). `ids` is sorted ascending (construction order), the
/// `WordSet` invariant.
fn attach_membership(normalized: &mut NormalizedQuery, binders: &[(usize, FieldId)], ids: &[u64]) {
    debug_assert!(!ids.is_empty(), "empty sets take the rule-death path");
    debug_assert!(ids.windows(2).all(|w| w[0] < w[1]), "sorted, deduplicated");
    for (idx, field) in binders {
        normalized.occurrences[*idx]
            .filters
            .push(FilterPredicate::Compare {
                field: (*field).into(),
                op: WordCmp::Eq,
                value: Const::WordSet(ids.to_vec()),
            });
    }
}

/// Deletes a folded negated occurrence's anti-probe descriptor: the
/// rejection it encoded is now the attached complement membership (or
/// provably never fired).
fn remove_anti_probe(normalized: &mut NormalizedQuery, c_idx: usize) {
    let occ_id = normalized.occurrences[c_idx].occ_id;
    normalized
        .anti_probes
        .retain(|probe| probe.occurrence != occ_id);
}

/// The fold's rendered picture — `Kind{mastered == true}` — in the rule
/// notation's value formats (`ir/render`, one notation on every
/// diagnostic surface). Two readers: the rule-death verdict
/// (`folded to ∅: …`) and introspection's fold line
/// (`exec/introspect/into_stats.rs`), off the folded occurrence's retained
/// filter list. A word at the relation's own id position prints its
/// handle (a handle set for an attached membership) — the vocabulary's
/// names on every surface a row id reaches.
pub(crate) fn folded_picture(
    schema: &Schema,
    relation: RelationId,
    filters: &[FilterPredicate],
) -> String {
    let relation = schema.relation(relation);
    let mut out = String::from(relation.name());
    out.push('{');
    for (index, filter) in filters.iter().enumerate() {
        if index > 0 {
            out.push_str(" ∧ ");
        }
        crate::image::view::render_filter(&mut out, relation, filter);
    }
    out.push('}');
    out
}

#[cfg(test)]
mod tests;
