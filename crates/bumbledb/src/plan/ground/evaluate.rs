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
//!    ([`parse_resolvable`]). A param-bearing filter REFUSES
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
//! The fold mark remains `Copy`, so it cannot carry the parsed filter
//! set. introspection reparses the retained original filters on its cold path;
//! a failed reparse maps to an empty handle list after a debug assertion,
//! never to a production panic. The rendered picture always uses those
//! originals so diagnostics preserve the user's spelling.
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

use crate::allen::classify_bounds;
use crate::encoding::field_bytes;
use crate::image::view::{Const, FilterPredicate, MaskConst, ResolvedWordSource};
use crate::ir::normalize::{FoldedMark, NormalizedQuery, Role};
use crate::ir::render::{literal, mask_names};
use crate::ir::{CmpOp, VarId};
use crate::schema::{FieldId, IntervalElement, Relation, RelationId, Schema, ValueType};

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
    // THE GUARD (20-query-ir.md § engine recursion's consumer guards): sealed
    // extensions exist only for closed stored relations, so an `Idb`
    // occurrence has no stage-0 rows and never folds.
    let Some(relation_id) = occurrence.source.edb() else {
        return false;
    };
    let relation = schema.relation(relation_id);
    if relation.extension().is_none() {
        return false; // ordinary relations have no stage-0 rows
    }
    let Some(filters) = parse_resolvable(&occurrence.filters) else {
        return false; // condition 2 refusal (params, measures)
    };
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
    let survivors = surviving_ids(relation, &filters);
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
    // The positive fold's `Idb` guard, verbatim: no sealed extension,
    // no stage-0 rows, no fold (20-query-ir.md § engine recursion's consumer guards).
    let Some(relation_id) = occurrence.source.edb() else {
        return false;
    };
    let relation = schema.relation(relation_id);
    let Some(rows) = relation.extension() else {
        return false;
    };
    let Some(filters) = parse_resolvable(&occurrence.filters) else {
        return false;
    };
    let survivors = surviving_ids(relation, &filters);
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
    let mark = FoldedMark {
        ids: u16::try_from(survivors.len()).expect("extensions cap at 256 rows"),
        negated: true,
    };
    attach_membership(normalized, &binders, &complement);
    remove_anti_probe(normalized, c_idx);
    normalized.occurrences[c_idx].role = Role::Folded(mark);
    true
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

/// A closed atom's filter, proven prepare-resolvable: constants only,
/// over the sealed extension's column words. Minted exclusively by
/// [`parse_resolvable`]; [`surviving_ids`] consumes it totally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolvableFilter {
    /// Eq/Ne/Lt/Le/Gt/Ge against one encoded word (scalar columns).
    WordCompare {
        field: FieldId,
        op: CmpOp,
        word: u64,
    },
    /// Eq/Ne against a canonical multi-word value.
    BytesCompare {
        field: FieldId,
        bytes: Box<[u8]>,
        equal: bool,
    },
    /// Eq against a plan-constant word set (attached memberships).
    WordSetEq { field: FieldId, words: Box<[u64]> },
    /// A same-row comparison between two fields. The parser admits only
    /// the six ordinary comparison operators.
    FieldsCompare {
        left: FieldId,
        right: FieldId,
        op: CmpOp,
    },
    /// A constant point inside the column's interval.
    PointIn { field: FieldId, point: u64 },
    /// A same-row point field inside an interval field.
    FieldsPointIn { interval: FieldId, point: FieldId },
    /// The column's interval within a constant outer interval.
    Within {
        field: FieldId,
        start: u64,
        end: u64,
    },
    /// Literal-mask Allen between two interval fields on the row.
    FieldsAllen {
        left: FieldId,
        right: FieldId,
        mask: crate::allen::AllenMask,
    },
    /// Literal-mask Allen between the column and a constant interval.
    Allen {
        field: FieldId,
        other: (u64, u64),
        mask: crate::allen::AllenMask,
    },
}

/// **Condition 2 as a parser** — returns exactly the prepare-evaluable
/// vocabulary proved for every filter, or `None` without partial output.
///
/// Param-bearing shapes (`Param`/`ParamSet`/param masks/param points) are
/// stage-3 values a stage-2 pass must not judge — the bind-time fold
/// variant is REFUSED v0, recorded (trigger: a measured calendar-family
/// win). `str` literals (`PendingIntern`) cannot type against a closed
/// relation (closed relations refuse `str` columns —
/// `schema/validate.rs`) and refuse defensively. `AnyPointIn`'s set is a
/// bind-time `ParamSet` marker (stage-3). The measure kinds refuse: their
/// ray error is per-execution, not a prepare error (module doc).
///
/// The old boolean gate admitted malformed operator/constant pairings
/// (set inequality and order against non-word constants) that its
/// evaluator could not consume. They now refuse here; valid normalized
/// filters are unchanged, and the parser-totality test pins the boundary.
pub(crate) fn parse_resolvable(filters: &[FilterPredicate]) -> Option<Vec<ResolvableFilter>> {
    filters.iter().map(parse_filter).collect()
}

fn parse_filter(filter: &FilterPredicate) -> Option<ResolvableFilter> {
    let ordinary = |op: CmpOp| {
        matches!(
            op,
            CmpOp::Eq | CmpOp::Ne | CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge
        )
    };
    match filter {
        FilterPredicate::Compare { field, op, value } => match (op, value) {
            (CmpOp::Eq, Const::WordSet(words)) => Some(ResolvableFilter::WordSetEq {
                field: *field,
                words: words.clone().into_boxed_slice(),
            }),
            (CmpOp::Eq | CmpOp::Ne, Const::Words(words)) => Some(ResolvableFilter::BytesCompare {
                field: *field,
                bytes: words.iter().flat_map(|word| word.to_be_bytes()).collect(),
                equal: matches!(op, CmpOp::Eq),
            }),
            (CmpOp::Eq | CmpOp::Ne, Const::Interval { start, end }) => {
                let mut bytes = Vec::with_capacity(16);
                bytes.extend_from_slice(&start.to_be_bytes());
                bytes.extend_from_slice(&end.to_be_bytes());
                Some(ResolvableFilter::BytesCompare {
                    field: *field,
                    bytes: bytes.into_boxed_slice(),
                    equal: matches!(op, CmpOp::Eq),
                })
            }
            (op, Const::Word(word)) if ordinary(*op) => Some(ResolvableFilter::WordCompare {
                field: *field,
                op: *op,
                word: *word,
            }),
            (CmpOp::Eq | CmpOp::Ne, Const::Byte(byte)) => Some(ResolvableFilter::WordCompare {
                field: *field,
                op: *op,
                word: u64::from(*byte),
            }),
            // Params, pending interns, set inequality, order over
            // multi-word/byte values, and the already-lowered interval
            // operators all refuse.
            _ => None,
        },
        FilterPredicate::FieldsCompare { left, right, op } if ordinary(*op) => {
            Some(ResolvableFilter::FieldsCompare {
                left: *left,
                right: *right,
                op: *op,
            })
        }
        FilterPredicate::PointIn {
            field,
            point: ResolvedWordSource::Word(point),
        } => Some(ResolvableFilter::PointIn {
            field: *field,
            point: *point,
        }),
        FilterPredicate::FieldsPointIn { interval, point } => {
            Some(ResolvableFilter::FieldsPointIn {
                interval: *interval,
                point: *point,
            })
        }
        FilterPredicate::FieldWithin {
            field,
            outer: Const::Interval { start, end },
        } => Some(ResolvableFilter::Within {
            field: *field,
            start: *start,
            end: *end,
        }),
        FilterPredicate::FieldsAllen {
            left,
            right,
            mask: MaskConst::Mask(mask),
        } => Some(ResolvableFilter::FieldsAllen {
            left: *left,
            right: *right,
            mask: *mask,
        }),
        FilterPredicate::FieldAllen {
            field,
            other: Const::Interval { start, end },
            mask: MaskConst::Mask(mask),
        } => Some(ResolvableFilter::Allen {
            field: *field,
            other: (*start, *end),
            mask: *mask,
        }),
        // Param points/masks/intervals, `AnyPointIn`'s stage-3 set, and
        // measure filters refuse for the staging/error-timing reasons
        // above. The unmatched `FieldsCompare` arm is Allen/PointIn,
        // which normalization lowers to fixed filter shapes.
        FilterPredicate::FieldsCompare { .. }
        | FilterPredicate::PointIn { .. }
        | FilterPredicate::AnyPointIn { .. }
        | FilterPredicate::FieldsAllen { .. }
        | FilterPredicate::FieldAllen { .. }
        | FilterPredicate::FieldWithin { .. }
        | FilterPredicate::DurationCompare { .. }
        | FilterPredicate::DurationFieldsCompare { .. } => None,
    }
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
                    && ((occ.source.edb() == Some(closed) && *field == FieldId(0))
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
        occurrence.source.edb() == Some(statement.source.relation)
            && statement.source.projection.as_ref() == [field]
            && statement.target.relation == closed
            && statement.target.projection.as_ref() == [FieldId(0)]
            && super::encoded_selection(&statement.source).is_some_and(|phi| {
                // A disjunctive φ binding answers "unknown" (`None`
                // upstream): no single-literal filter list certifies a
                // set binding, so the domain guarantee is not spent.
                phi.iter().all(|(f, value)| {
                    occurrence.filters.iter().any(|filter| {
                        matches!(
                            filter,
                            FilterPredicate::Compare { field: ff, op: CmpOp::Eq, value: v }
                                if ff == f && v == value
                        )
                    })
                })
            })
    })
}

/// The prepare-time evaluation: σ(filters) over the sealed extension
/// rows, as the ascending surviving row-id list (row id = declaration
/// index — `schema.rs`, `SealedRow`). n ≤ 256 rows through the scalar
/// comparison paths — encoded-word compares, the scalar `Allen`
/// classify, never a batch kernel. Its narrowed input was minted by
/// [`parse_resolvable`], so evaluation is total over the vocabulary.
/// Crate-visible for the introspection surface (`exec/introspection/into_stats.rs`),
/// which re-runs the σ to name the surviving handles.
pub(crate) fn surviving_ids(relation: &Relation, filters: &[ResolvableFilter]) -> Vec<u64> {
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
/// canonical-byte equality. Every match arm is over a parsed shape; no
/// symbolic or measure form reaches this function.
fn row_satisfies(
    layout: &crate::encoding::FactLayout,
    fact: &[u8],
    filter: &ResolvableFilter,
) -> bool {
    let bytes = |field: FieldId| field_bytes(fact, layout, usize::from(field.0));
    let word = |field: FieldId| field_word(layout, fact, field);
    let pair = |field: FieldId| {
        // A validated interval field is exactly two words; `as_chunks`
        // carries the half width in its type.
        let (halves, _) = bytes(field).as_chunks::<8>();
        (u64::from_be_bytes(halves[0]), u64::from_be_bytes(halves[1]))
    };
    match filter {
        ResolvableFilter::WordCompare {
            field,
            op,
            word: bound,
        } => op.compare(&word(*field), bound),
        ResolvableFilter::BytesCompare {
            field,
            bytes: bound,
            equal,
        } => (bytes(*field) == bound.as_ref()) == *equal,
        ResolvableFilter::WordSetEq { field, words } => words.binary_search(&word(*field)).is_ok(),
        ResolvableFilter::FieldsCompare { left, right, op } => match op {
            CmpOp::Eq => bytes(*left) == bytes(*right),
            CmpOp::Ne => bytes(*left) != bytes(*right),
            CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => {
                op.compare(&word(*left), &word(*right))
            }
            // The parser never constructs these; returning false keeps
            // the consumer total even if this crate-visible enum gains
            // another constructor in the future.
            CmpOp::Allen { .. } | CmpOp::PointIn => false,
        },
        ResolvableFilter::PointIn { field, point } => {
            let (start, end) = pair(*field);
            start <= *point && *point < end
        }
        ResolvableFilter::FieldsPointIn { interval, point } => {
            let (start, end) = pair(*interval);
            let p = word(*point);
            start <= p && p < end
        }
        ResolvableFilter::Within { field, start, end } => {
            let f = word(*field);
            *start <= f && f < *end
        }
        ResolvableFilter::FieldsAllen { left, right, mask } => {
            let (ls, le) = pair(*left);
            let (rs, re) = pair(*right);
            mask.contains(classify_bounds(&ls, &le, &rs, &re))
        }
        ResolvableFilter::Allen {
            field,
            other: (start, end),
            mask,
        } => {
            let (fs, fe) = pair(*field);
            mask.contains(classify_bounds(&fs, &fe, start, end))
        }
    }
}

/// One scalar field's encoded comparison word off canonical bytes: the
/// byte column widened, or the 8-byte column as-is.
fn field_word(layout: &crate::encoding::FactLayout, fact: &[u8], field: FieldId) -> u64 {
    let bytes = field_bytes(fact, layout, usize::from(field.0));
    match bytes {
        &[byte] => u64::from(byte),
        _ => match <[u8; 8]>::try_from(bytes) {
            Ok(word) => u64::from_be_bytes(word),
            Err(_) => unreachable!("parsed word filters address validated scalar columns"),
        },
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
                render_unparsed_filter(out, filter);
                return;
            };
            literal(
                out,
                &decoded_scalar(&element_type(&relation.field(*field).value_type), *point),
            );
            out.push_str(" in ");
            out.push_str(name(field));
        }
        FilterPredicate::FieldsPointIn { interval, point } => {
            out.push_str(name(point));
            out.push_str(" in ");
            out.push_str(name(interval));
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let Const::Interval { start, end } = outer else {
                render_unparsed_filter(out, filter);
                return;
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
                render_unparsed_filter(out, filter);
                return;
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
                render_unparsed_filter(out, filter);
                return;
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
            render_unparsed_filter(out, filter);
        }
    }
}

/// Defensive diagnostic fallback for an original filter that no longer
/// parses. Folded marks prove this unreachable in ordinary construction,
/// but diagnostic rendering stays total over the public filter sum.
fn render_unparsed_filter(out: &mut String, filter: &FilterPredicate) {
    use std::fmt::Write as _;
    let _ = write!(out, "{filter:?}");
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
        CmpOp::Allen { .. } => " Allen ",
        CmpOp::PointIn => " PointIn ",
    }
}

#[cfg(test)]
mod tests;
