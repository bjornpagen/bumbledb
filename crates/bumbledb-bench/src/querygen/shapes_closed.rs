//! The closed-relation query shapes (PRD 06): closed relations join the
//! drawable atom pool. Two shapes, four pattern classes:
//!
//! - [`closed_join`] — joins against closed relations, with and without
//!   payload-column projections and payload-column **selections** (the
//!   ψ-shaped literal on `Currency.minor_units`), plus the two
//!   handle-binding classes: a handle **literal** and a handle **param
//!   set** on a referencing field.
//! - [`closed_fold`] — the fold-shaped pattern PRD 07 targets, under its
//!   own family knob: a closed atom whose only escaping variable is the
//!   join id (an id-position variable join; any payload variable it
//!   binds is dead — bound, never projected, never compared).
//!
//! Both shapes are their own deliberate dressing (like the chase
//! shapes): random predicates or negated probes landing on the closed
//! atom would blur the class the coverage contract counts.

use bumbledb::{AggOp, FieldId, FindTerm, RelationId, Term, Value};

use crate::corpus_gen::Rng;
use crate::querygen::target::ids;
use crate::querygen::{Builder, ClosedVariant};

/// The referencing pairs: (source relation, referencing field, closed
/// target, the source's row-identity field, a source payload field to
/// project). The row-identity field keeps the fold's count meaningful
/// (one binding per referencing row).
const PAIRS: &[(RelationId, FieldId, RelationId, FieldId, FieldId)] = &[
    (
        ids::ACCOUNT,
        ids::account::CURRENCY,
        ids::CURRENCY,
        ids::account::ID,
        ids::account::HOLDER,
    ),
    (
        ids::JOURNAL_ENTRY,
        ids::journal_entry::SOURCE,
        ids::SOURCE,
        ids::journal_entry::ID,
        ids::journal_entry::CREATED_AT,
    ),
    (
        ids::POSTING_TAG,
        ids::posting_tag::TAG,
        ids::TAG,
        ids::posting_tag::POSTING,
        ids::posting_tag::POSTING,
    ),
];

fn pair(rng: &mut Rng) -> &'static (RelationId, FieldId, RelationId, FieldId, FieldId) {
    &PAIRS[usize::try_from(rng.range(PAIRS.len() as u64)).expect("small")]
}

/// One closed-relation join query, in one of the four class variants.
pub(super) fn closed_join(b: &mut Builder, rng: &mut Rng) {
    let variant = rng.range(4);
    // The payload-selection class needs the payload-bearing vocabulary.
    let (source, reference, closed, _, payload) = if variant == 1 { &PAIRS[0] } else { pair(rng) };
    let atom = b.add_atom(*source);
    match variant {
        // The plain join: handle variable through the closed atom's id
        // position, optionally projecting the vocabulary payload.
        0 => {
            let handle = b.bind_var(atom, *reference);
            b.find_var(handle);
            let vocabulary = b.add_atom(*closed);
            b.bind(vocabulary, FieldId(0), Term::Var(handle));
            if *closed == ids::CURRENCY && rng.chance(1, 2) {
                let units = b.bind_var(vocabulary, ids::currency::MINOR_UNITS);
                b.find_var(units);
            }
            let projected = b.bind_var(atom, *payload);
            b.find_var(projected);
            b.closed = Some(ClosedVariant::Join);
        }
        // The payload-column selection: the join restricted to a
        // ψ-shaped literal on the vocabulary's payload (0 selects the
        // zero-decimal sub-vocabulary; 2 its complement).
        1 => {
            let handle = b.bind_var(atom, *reference);
            b.find_var(handle);
            let vocabulary = b.add_atom(*closed);
            b.bind(vocabulary, FieldId(0), Term::Var(handle));
            let units = if rng.chance(1, 2) { 0 } else { 2 };
            b.bind(
                vocabulary,
                ids::currency::MINOR_UNITS,
                Term::Literal(Value::U64(units)),
            );
            let projected = b.bind_var(atom, *payload);
            b.find_var(projected);
            b.closed = Some(ClosedVariant::JoinSelected);
        }
        // The handle literal on the referencing field — the vocabulary
        // row id used as a plain u64 selection.
        2 => {
            b.bind(atom, *reference, Term::Literal(Value::U64(rng.range(3))));
            let projected = b.bind_var(atom, *payload);
            b.find_var(projected);
            b.closed = Some(ClosedVariant::HandleLiteral);
        }
        // The handle param set on the referencing field.
        _ => {
            let param = b.fresh_param();
            b.bind(atom, *reference, Term::ParamSet(param));
            let projected = b.bind_var(atom, *payload);
            b.find_var(projected);
            b.closed = Some(ClosedVariant::HandleSet);
        }
    }
}

/// The fold shape, adversarially covering the shipped fold: referencing
/// rows counted per handle through an
/// id-position variable join, the closed atom contributing nothing but
/// the join id — its payload, when bound, is a dead variable.
pub(super) fn closed_fold(b: &mut Builder, rng: &mut Rng) {
    let (source, reference, closed, row, _) = pair(rng);
    let atom = b.add_atom(*source);
    let handle = b.bind_var(atom, *reference);
    // The row-identity binding keeps one distinct binding per
    // referencing row, so Count counts references per handle.
    let _rows = b.bind_var(atom, *row);
    let vocabulary = b.add_atom(*closed);
    b.bind(vocabulary, FieldId(0), Term::Var(handle));
    if *closed == ids::CURRENCY && rng.chance(1, 2) {
        // The dead payload variable: bound on the closed atom, escaping
        // nowhere — exactly the shape the fold elides.
        let _dead = b.bind_var(vocabulary, ids::currency::MINOR_UNITS);
    }
    b.find_var(handle);
    b.finds.push(FindTerm::Aggregate {
        op: AggOp::Count,
        over: None,
    });
    b.closed = Some(ClosedVariant::Fold);
}
