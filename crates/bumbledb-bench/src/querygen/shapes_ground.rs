use bumbledb::{FieldId, FindTerm, RelationId, Term, Value};

use crate::corpus_gen::Rng;
use crate::querygen::target::{SOURCE_IMPORT, ids};
use crate::querygen::{Builder, GroundVariant};

const WALKS: &[(RelationId, FieldId, FieldId, RelationId, FieldId, FieldId)] = &[
    (
        ids::POSTING,
        ids::posting::ACCOUNT,
        ids::posting::AMOUNT,
        ids::ACCOUNT,
        ids::account::ID,
        ids::account::CURRENCY,
    ),
    (
        ids::POSTING,
        ids::posting::ENTRY,
        ids::posting::AMOUNT,
        ids::JOURNAL_ENTRY,
        ids::journal_entry::ID,
        ids::journal_entry::CREATED_AT,
    ),
    (
        ids::POSTING,
        ids::posting::INSTRUMENT,
        ids::posting::AT,
        ids::INSTRUMENT,
        ids::instrument::ID,
        ids::instrument::SYMBOL,
    ),
    (
        ids::ACCOUNT,
        ids::account::HOLDER,
        ids::account::CURRENCY,
        ids::HOLDER,
        ids::holder::ID,
        ids::holder::NAME,
    ),
    (
        ids::POSTING_TAG,
        ids::posting_tag::POSTING,
        ids::posting_tag::TAG,
        ids::POSTING,
        ids::posting::ID,
        ids::posting::AMOUNT,
    ),
    (
        ids::MANDATE,
        ids::mandate::ACCOUNT,
        ids::mandate::ORG,
        ids::ACCOUNT,
        ids::account::ID,
        ids::account::HOLDER,
    ),
    (
        ids::ORG_PARENT,
        ids::org_parent::CHILD,
        ids::org_parent::PARENT,
        ids::ORG,
        ids::org::ID,
        ids::org::NAME,
    ),
];

pub(super) fn existence_walk(b: &mut Builder, rng: &mut Rng) {
    let idx = usize::try_from(rng.range(WALKS.len() as u64)).expect("small");
    let (source_rel, ref_field, payload_field, target_rel, key_field, extra_field) = WALKS[idx];
    let source = b.add_atom(source_rel);
    let join = b.bind_var(source, ref_field);
    let payload = b.bind_var(source, payload_field);
    let target = b.add_atom(target_rel);
    b.bind(target, key_field, Term::Var(join));
    match rng.range(9) {
        0..=2 => {
            b.find_var(payload);
            let extra = b.bind_var(target, extra_field);
            b.find_var(extra);
            b.ground = Some(GroundVariant::WalkExtraField);
        }

        3 | 4 => {
            b.find_var(join);
            b.finds.push(FindTerm::Count);
            b.ground = Some(GroundVariant::Walk);
        }
        _ => {
            b.find_var(payload);
            b.ground = Some(GroundVariant::Walk);
        }
    }
}

pub(super) fn du_walk(b: &mut Builder, rng: &mut Rng) {
    let import = Term::Literal(Value::U64(SOURCE_IMPORT));
    match rng.range(3) {
        0 => {
            let child = b.add_atom(ids::IMPORT_BATCH);
            let join = b.bind_var(child, ids::import_batch::ENTRY);
            let payload = b.bind_var(child, ids::import_batch::BATCH);
            b.find_var(payload);
            let header = b.add_atom(ids::JOURNAL_ENTRY);
            b.bind(header, ids::journal_entry::ID, Term::Var(join));
            b.bind(header, ids::journal_entry::SOURCE, import);
            b.ground = Some(GroundVariant::DuHeader);
        }
        variant => {
            let header = b.add_atom(ids::JOURNAL_ENTRY);
            let join = b.bind_var(header, ids::journal_entry::ID);
            let payload = b.bind_var(header, ids::journal_entry::CREATED_AT);
            b.find_var(payload);
            if variant == 1 {
                b.bind(header, ids::journal_entry::SOURCE, import);
                b.ground = Some(GroundVariant::DuChild);
            } else {
                b.ground = Some(GroundVariant::DuMissingPhi);
            }
            let child = b.add_atom(ids::IMPORT_BATCH);
            b.bind(child, ids::import_batch::ENTRY, Term::Var(join));
        }
    }
}
