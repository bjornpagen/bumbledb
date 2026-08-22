use bumbledb::{FieldId, FindTerm, RelationId, Term, Value};

use crate::corpus_gen::Rng;
use crate::querygen::target::ids;
use crate::querygen::{Builder, ClosedVariant};

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

pub(super) fn closed_join(b: &mut Builder, rng: &mut Rng) {
    let variant = rng.range(4);

    let (source, reference, closed, _, payload) = if variant == 1 { &PAIRS[0] } else { pair(rng) };
    let atom = b.add_atom(*source);
    match variant {
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

        2 => {
            b.bind(atom, *reference, Term::Literal(Value::U64(rng.range(3))));
            let projected = b.bind_var(atom, *payload);
            b.find_var(projected);
            b.closed = Some(ClosedVariant::HandleLiteral);
        }

        _ => {
            let param = b.fresh_param();
            b.bind(atom, *reference, Term::ParamSet(param));
            let projected = b.bind_var(atom, *payload);
            b.find_var(projected);
            b.closed = Some(ClosedVariant::HandleSet);
        }
    }
}

pub(super) fn ground_fold(b: &mut Builder, rng: &mut Rng) {
    let (source, reference, closed, row, _) = pair(rng);
    let atom = b.add_atom(*source);
    let handle = b.bind_var(atom, *reference);

    let _rows = b.bind_var(atom, *row);
    let vocabulary = b.add_atom(*closed);
    b.bind(vocabulary, FieldId(0), Term::Var(handle));
    if *closed == ids::CURRENCY && rng.chance(1, 2) {
        let _dead = b.bind_var(vocabulary, ids::currency::MINOR_UNITS);
    }
    b.find_var(handle);
    b.finds.push(FindTerm::Count);
    b.closed = Some(ClosedVariant::Fold);
}
