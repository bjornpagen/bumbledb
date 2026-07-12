//! The chase-evaluator: folding stage-zero atoms
//! (docs/architecture/40-execution.md, § the chase: elimination and
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
//!    ([`filters_prepare_resolvable`]). A param-bearing filter REFUSES
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
//!   the pipeline runs fold-then-chase, so the evaluator writes the
//!   verdict itself rather than routing a set back through the fold.
//! - No live `k` (a pure constant gate, e.g. a nonemptiness check over
//!   a ψ-subset): `|S| ≥ 1` deletes the atom outright; `|S| == 0` kills
//!   the rule. The gate must bind **no variables at all**: a dead-but-
//!   bound variable still multiplies an aggregate's fold domain (the
//!   binding set is over ALL query variables — 40-execution, D2), so a
//!   var-binding guard is REFUSED, recorded; trigger: a measured
//!   projection-sink-only win.
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
//!   then inside the compiled member set, `Resolved::ClosedContainment`).
//!   No witness → REFUSED, recorded (the anti-probe stays; trigger: a
//!   profiled anti-probe worth folding under a richer domain analysis).
//! - complement empty (`S` = the whole extension): under the same
//!   guarantee every binding's `k` is rejected — the rule is dead.
//! - A zero-binding negated gate (`!Kind(mastered == true)`): `|S| ≥ 1`
//!   rejects every binding — rule dead; `|S| == 0` deletes (above).

use std::collections::BTreeSet;

use crate::allen::classify_bounds;
use crate::encoding::field_bytes;
use crate::image::view::{Const, FilterPredicate, MaskConst, ResolvedWordSource};
use crate::ir::normalize::{FoldedMark, NormalizedQuery, Role};
use crate::ir::render::{literal, mask_names};
use crate::ir::{CmpOp, VarId};
use crate::schema::{
    FieldId, IntervalElement, Relation, RelationId, Schema, StatementDescriptor, ValueType,
};

use super::var_is_dead;

/// One evaluator step of the chase fixpoint: finds the first foldable
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
        let folded = match normalized.occurrences[c_idx].role {
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
    let relation = schema.relation(occurrence.relation);
    if relation.extension().is_none() {
        return false; // ordinary relations have no stage-0 rows
    }
    if !filters_prepare_resolvable(&occurrence.filters) {
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
        // The pure-guard shape: only a var-less atom may delete — a
        // dead-but-bound variable still multiplies an aggregate's fold
        // domain (module doc), and the guard's truth must survive
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
            folded_picture(
                schema,
                normalized.occurrences[c_idx].relation,
                &normalized.occurrences[c_idx].filters,
            )
        ));
        return true;
    }
    let mark = FoldedMark {
        ids: u16::try_from(survivors.len()).expect("extensions cap at 256 rows"),
        negated: false,
    };
    attach_membership(normalized, &binders, &survivors);
    normalized.occurrences[c_idx].role = Role::Folded(mark);
    true
}

/// One negated occurrence's fold attempt (module doc, the complement
/// fold — direction pinned there and by the tests).
fn fold_negated(normalized: &mut NormalizedQuery, schema: &Schema, c_idx: usize) -> bool {
    let occurrence = &normalized.occurrences[c_idx];
    let relation = schema.relation(occurrence.relation);
    let Some(rows) = relation.extension() else {
        return false;
    };
    if !filters_prepare_resolvable(&occurrence.filters) {
        return false;
    }
    let survivors = surviving_ids(relation, &occurrence.filters);
    if survivors.is_empty() {
        // No fact can ever match the probe's filters: the anti-probe
        // never rejects, whatever the bindings — the atom deletes
        // outright (and the rule is NOT empty). Any binding shape
        // qualifies: emptiness of σ needs no key reasoning.
        remove_anti_probe(normalized, c_idx);
        normalized.occurrences[c_idx].role = Role::Folded(FoldedMark {
            ids: 0,
            negated: true,
        });
        return true;
    }
    if occurrence.vars.is_empty() {
        // The negated gate: some sealed row satisfies the filters on
        // every store, so the probe rejects every binding — rule dead.
        normalized.dead = Some(format!(
            "folded: !{} rejects every binding",
            folded_picture(schema, occurrence.relation, &occurrence.filters)
        ));
        return true;
    }
    // The keyed shape: exactly one variable, at the id position — the
    // probe is then precisely `k ∈ S`. A payload-bound probe key would
    // need multi-column set reasoning; REFUSED v0, recorded (trigger: a
    // profiled multi-key anti-probe on a closed relation).
    let [(field, k)] = occurrence.vars.as_slice() else {
        return false;
    };
    if *field != FieldId(0) {
        return false;
    }
    let k = *k;
    let closed = occurrence.relation;
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
    let mark = FoldedMark {
        ids: u16::try_from(survivors.len()).expect("extensions cap at 256 rows"),
        negated: true,
    };
    attach_membership(normalized, &binders, &complement);
    remove_anti_probe(normalized, c_idx);
    normalized.occurrences[c_idx].role = Role::Folded(mark);
    true
}

// The foldability conditions, one named predicate each (the chase
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
/// then a guard candidate — and a var-binding guard refuses, module
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

/// **Condition 2** — every filter is a prepare-evaluable constant
/// shape: Eq/range compares, same-fact compares, membership
/// compositions, and literal-mask `Allen` kinds. Param-bearing shapes
/// (`Param`/`ParamSet`/param masks/param points) are stage-3 values a
/// stage-2 pass must not judge — the bind-time fold variant is REFUSED
/// v0, recorded (trigger: a measured calendar-family win). `str`
/// literals (`PendingIntern`) cannot type against a closed relation
/// (closed relations refuse `str` columns — `schema/validate.rs`) and
/// refuse defensively. Measure kinds refuse: their ray error is a
/// per-execution error, not a prepare error (module doc).
pub(super) fn filters_prepare_resolvable(filters: &[FilterPredicate]) -> bool {
    filters.iter().all(|filter| match filter {
        FilterPredicate::Compare { value, .. } => matches!(
            value,
            Const::Word(_)
                | Const::Byte(_)
                | Const::Words(_)
                | Const::Interval { .. }
                | Const::WordSet(_)
        ),
        FilterPredicate::FieldsCompare { .. } | FilterPredicate::FieldsContainPoint { .. } => true,
        FilterPredicate::PointIn { point, .. } => matches!(point, ResolvedWordSource::Word(_)),
        FilterPredicate::FieldsAllen { mask, .. } => matches!(mask, MaskConst::Mask(_)),
        FilterPredicate::FieldAllen { other, mask, .. } => {
            matches!(other, Const::Interval { .. }) && matches!(mask, MaskConst::Mask(_))
        }
        FilterPredicate::FieldWithin { outer, .. } => matches!(outer, Const::Interval { .. }),
        // `AnyPointIn`'s set is a bind-time `ParamSet` marker (stage-3),
        // and the measure kinds' ray error is per-execution (module
        // doc) — all three refuse.
        FilterPredicate::AnyPointIn { .. }
        | FilterPredicate::DurationCompare { .. }
        | FilterPredicate::DurationFieldsCompare { .. } => false,
    })
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
                    && ((occ.relation == closed && *field == FieldId(0))
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
    schema.statements().iter().any(|statement| {
        let StatementDescriptor::Containment { source, target } = &statement.descriptor else {
            return false;
        };
        source.relation == occurrence.relation
            && source.projection.as_ref() == [field]
            && target.relation == closed
            && target.projection.as_ref() == [FieldId(0)]
            && super::encoded_selection(source).iter().all(|(f, value)| {
                occurrence.filters.iter().any(|filter| {
                    matches!(
                        filter,
                        FilterPredicate::Compare { field: ff, op: CmpOp::Eq, value: v }
                            if ff == f && v == value
                    )
                })
            })
    })
}

/// The prepare-time evaluation: σ(filters) over the sealed extension
/// rows, as the ascending surviving row-id list (row id = declaration
/// index — `schema.rs`, `SealedRow`). n ≤ 256 rows through the scalar
/// comparison paths — encoded-word compares, the scalar `Allen`
/// classify, never a batch kernel. Callers hold
/// [`filters_prepare_resolvable`]; unresolvable shapes are unreachable.
/// Crate-visible for the EXPLAIN surface (`exec/explain/into_stats.rs`),
/// which re-runs the σ to name the surviving handles.
pub(crate) fn surviving_ids(relation: &Relation, filters: &[FilterPredicate]) -> Vec<u64> {
    let layout = relation.layout();
    relation
        .extension()
        .expect("callers checked closedness")
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            filters
                .iter()
                .all(|filter| row_satisfies(layout, &row.fact, filter))
        })
        .map(|(id, _)| id as u64)
        .collect()
}

/// One filter over one sealed row's canonical bytes. Encoded words are
/// order-preserving maps of their values (u64 identity, I64 sign-flip,
/// interval endpoints pairwise — `docs/architecture/50-storage.md`), so
/// word comparison IS value comparison; Eq/Ne over any shape is
/// canonical-byte equality.
fn row_satisfies(
    layout: &crate::encoding::FactLayout,
    fact: &[u8],
    filter: &FilterPredicate,
) -> bool {
    let bytes = |field: FieldId| field_bytes(fact, layout, usize::from(field.0));
    let word = |field: FieldId| field_word(layout, fact, field);
    let pair = |field: FieldId| {
        let b = bytes(field);
        (be_word(&b[..8]), be_word(&b[8..16]))
    };
    match filter {
        FilterPredicate::Compare { field, op, value } => match op {
            CmpOp::Eq => match value {
                Const::WordSet(words) => words.binary_search(&word(*field)).is_ok(),
                _ => bytes(*field) == const_bytes(value),
            },
            CmpOp::Ne => bytes(*field) != const_bytes(value),
            CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => {
                let Const::Word(bound) = value else {
                    unreachable!("validated: order operators compare u64/i64 words")
                };
                order_holds(*op, word(*field), *bound)
            }
            CmpOp::Allen { .. } | CmpOp::Contains => {
                unreachable!("interval predicates lower to their fixed shapes")
            }
        },
        FilterPredicate::FieldsCompare { left, right, op } => match op {
            CmpOp::Eq => bytes(*left) == bytes(*right),
            CmpOp::Ne => bytes(*left) != bytes(*right),
            CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => {
                order_holds(*op, word(*left), word(*right))
            }
            CmpOp::Allen { .. } | CmpOp::Contains => {
                unreachable!("same-atom interval predicates lower to their fixed shapes")
            }
        },
        FilterPredicate::PointIn { field, point } => {
            let ResolvedWordSource::Word(point) = point else {
                unreachable!("filters_prepare_resolvable admits literal points only")
            };
            let (start, end) = pair(*field);
            start <= *point && *point < end
        }
        FilterPredicate::FieldsContainPoint { interval, point } => {
            let (start, end) = pair(*interval);
            let p = word(*point);
            start <= p && p < end
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let Const::Interval { start, end } = outer else {
                unreachable!("filters_prepare_resolvable admits interval constants only")
            };
            let f = word(*field);
            *start <= f && f < *end
        }
        FilterPredicate::FieldsAllen { left, right, mask } => {
            let MaskConst::Mask(mask) = mask else {
                unreachable!("filters_prepare_resolvable admits literal masks only")
            };
            let (ls, le) = pair(*left);
            let (rs, re) = pair(*right);
            mask.contains(classify_bounds(&ls, &le, &rs, &re))
        }
        FilterPredicate::FieldAllen { field, other, mask } => {
            let (MaskConst::Mask(mask), Const::Interval { start, end }) = (mask, other) else {
                unreachable!("filters_prepare_resolvable admits literal masks and intervals only")
            };
            let (fs, fe) = pair(*field);
            mask.contains(classify_bounds(&fs, &fe, start, end))
        }
        FilterPredicate::AnyPointIn { .. }
        | FilterPredicate::DurationCompare { .. }
        | FilterPredicate::DurationFieldsCompare { .. } => {
            unreachable!("filters_prepare_resolvable refused these shapes")
        }
    }
}

/// One scalar field's encoded comparison word off canonical bytes: the
/// byte column widened, or the 8-byte column as-is.
fn field_word(layout: &crate::encoding::FactLayout, fact: &[u8], field: FieldId) -> u64 {
    let bytes = field_bytes(fact, layout, usize::from(field.0));
    match bytes.len() {
        1 => u64::from(bytes[0]),
        8 => be_word(bytes),
        _ => unreachable!("word reads address scalar columns"),
    }
}

fn be_word(bytes: &[u8]) -> u64 {
    u64::from_be_bytes(bytes.try_into().expect("8-byte slice"))
}

/// A constant's canonical bytes — exactly what the sealed row stores
/// for a value-equal fact (`Const` docs: column form IS the canonical
/// encoding, word-padded where the fact is).
fn const_bytes(value: &Const) -> Vec<u8> {
    match value {
        Const::Word(word) => word.to_be_bytes().to_vec(),
        Const::Byte(byte) => vec![*byte],
        Const::Words(words) => words.iter().flat_map(|w| w.to_be_bytes()).collect(),
        Const::Interval { start, end } => {
            let mut out = start.to_be_bytes().to_vec();
            out.extend_from_slice(&end.to_be_bytes());
            out
        }
        Const::WordSet(_) | Const::Param(_) | Const::ParamSet(_) | Const::PendingIntern { .. } => {
            unreachable!("callers matched the resolvable scalar shapes")
        }
    }
}

/// Which order comparison holds between two encoded words (both
/// encodings are order-preserving onto u64 — one unsigned domain).
fn order_holds(op: CmpOp, lhs: u64, rhs: u64) -> bool {
    match op {
        CmpOp::Lt => lhs < rhs,
        CmpOp::Le => lhs <= rhs,
        CmpOp::Gt => lhs > rhs,
        CmpOp::Ge => lhs >= rhs,
        _ => unreachable!("callers matched order operators"),
    }
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
                field: *field,
                op: CmpOp::Eq,
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
/// (`folded to ∅: …`) and EXPLAIN's fold line
/// (`exec/explain/into_stats.rs`), off the folded occurrence's retained
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
        render_filter(&mut out, relation, filter);
    }
    out.push('}');
    out
}

/// One prepare-resolved filter's picture (unresolvable shapes never
/// reach a folded occurrence's list).
fn render_filter(out: &mut String, relation: &Relation, filter: &FilterPredicate) {
    use crate::ir::normalize::{decoded_interval, decoded_scalar, render_const};
    let name = |field: &FieldId| relation.field(*field).name.as_ref();
    match filter {
        FilterPredicate::Compare { field, op, value } => {
            out.push_str(name(field));
            out.push_str(if matches!(value, Const::WordSet(_)) {
                " ∈ "
            } else {
                op_symbol(*op)
            });
            // The relation's own id position holds row ids — print the
            // handles (a membership set as a handle set), never numbers.
            match value {
                Const::Word(word) if *field == FieldId(0) && relation.is_closed() => {
                    push_handle(out, relation, *word);
                }
                Const::WordSet(words) if *field == FieldId(0) && relation.is_closed() => {
                    out.push('{');
                    for (index, word) in words.iter().enumerate() {
                        if index > 0 {
                            out.push_str(", ");
                        }
                        push_handle(out, relation, *word);
                    }
                    out.push('}');
                }
                _ => render_const(out, &relation.field(*field).value_type, value),
            }
        }
        FilterPredicate::FieldsCompare { left, right, op } => {
            out.push_str(name(left));
            out.push_str(op_symbol(*op));
            out.push_str(name(right));
        }
        FilterPredicate::PointIn { field, point } => {
            let ResolvedWordSource::Word(point) = point else {
                unreachable!("folded filters are prepare-resolved")
            };
            literal(
                out,
                &decoded_scalar(&element_type(&relation.field(*field).value_type), *point),
            );
            out.push_str(" in ");
            out.push_str(name(field));
        }
        FilterPredicate::FieldsContainPoint { interval, point } => {
            out.push_str(name(point));
            out.push_str(" in ");
            out.push_str(name(interval));
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let Const::Interval { start, end } = outer else {
                unreachable!("folded filters are prepare-resolved")
            };
            out.push_str(name(field));
            out.push_str(" in ");
            let outer_type = ValueType::Interval {
                element: match relation.field(*field).value_type {
                    ValueType::I64 => IntervalElement::I64,
                    _ => IntervalElement::U64,
                },
            };
            literal(out, &decoded_interval(&outer_type, (*start, *end)));
        }
        FilterPredicate::FieldsAllen { left, right, mask } => {
            let MaskConst::Mask(mask) = mask else {
                unreachable!("folded filters are prepare-resolved")
            };
            out.push_str("Allen(");
            out.push_str(name(left));
            out.push_str(", ");
            mask_names(out, *mask);
            out.push_str(", ");
            out.push_str(name(right));
            out.push(')');
        }
        FilterPredicate::FieldAllen { field, other, mask } => {
            let (MaskConst::Mask(mask), Const::Interval { start, end }) = (mask, other) else {
                unreachable!("folded filters are prepare-resolved")
            };
            out.push_str("Allen(");
            out.push_str(name(field));
            out.push_str(", ");
            mask_names(out, *mask);
            out.push_str(", ");
            literal(
                out,
                &decoded_interval(&relation.field(*field).value_type, (*start, *end)),
            );
            out.push(')');
        }
        FilterPredicate::AnyPointIn { .. }
        | FilterPredicate::DurationCompare { .. }
        | FilterPredicate::DurationFieldsCompare { .. } => {
            unreachable!("folded filters are prepare-resolved")
        }
    }
}

/// One row id at a closed relation's own id position, as its handle —
/// `DirectPass`; an out-of-range id prints visibly wrong as `Kind(7?)`
/// (the `ir/render` fallback convention: the relation's name, since the
/// engine never learns host newtype names).
pub(crate) fn push_handle(out: &mut String, relation: &Relation, id: u64) {
    let row = relation
        .extension()
        .and_then(|rows| usize::try_from(id).ok().and_then(|index| rows.get(index)));
    if let Some(row) = row {
        out.push_str(&row.handle);
    } else {
        use std::fmt::Write as _;
        let _ = write!(out, "{}({id}?)", relation.name());
    }
}

/// An interval field's element type (the point's rendering type).
fn element_type(value_type: &ValueType) -> ValueType {
    match value_type {
        ValueType::Interval {
            element: IntervalElement::I64,
        } => ValueType::I64,
        _ => ValueType::U64,
    }
}

fn op_symbol(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => " == ",
        CmpOp::Ne => " != ",
        CmpOp::Lt => " < ",
        CmpOp::Le => " <= ",
        CmpOp::Gt => " > ",
        CmpOp::Ge => " >= ",
        CmpOp::Allen { .. } | CmpOp::Contains => {
            unreachable!("interval predicates lower to their fixed shapes")
        }
    }
}

#[cfg(test)]
mod tests;
