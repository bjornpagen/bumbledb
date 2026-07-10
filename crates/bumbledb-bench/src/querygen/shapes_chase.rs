//! The chase shapes (`docs/architecture/40-execution.md` § the chase;
//! `60-validation.md` generator contract): deliberately eliminable join
//! geometry — the existence walk (the containment target joined on its
//! full key with nothing else read from it) and the discriminated-union
//! one-sided walk in both `==` directions — plus the near-miss refusals
//! (one extra projected target field; missing φ), so the randomized
//! differential exercises the rewrite *and* its refusals: the naive
//! model and `SQLite` compute the unrewritten query, which is the
//! differential. These shapes are neither dressed nor negated
//! (`construct.rs`): a random predicate landing on the target atom
//! would flip an eliminable shape to a refusal nondeterministically,
//! and the coverage contract asserts each variant appears per run.

use bumbledb::{AggOp, FieldId, FindTerm, RelationId, Term, Value};

use crate::gen::Rng;
use crate::querygen::target::{ids, SOURCE_IMPORT};
use crate::querygen::{Builder, ChaseVariant};

/// The containment walks the existence shape rotates over: (source
/// relation, reference field, source payload field, target relation,
/// target key field, extra target field for the near-miss variant) —
/// one row per scalar containment of the target schema whose source
/// carries a payload to project.
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

/// The existence walk: source atom projecting its own payload, target
/// joined on its full key. A third of the draws are the near-miss
/// (one extra projected target field — must refuse); a third of the
/// rest fold under an aggregate sink (the rewrite is sink-independent,
/// `40-execution.md`).
pub(super) fn existence_walk(b: &mut Builder, rng: &mut Rng) {
    let idx = usize::try_from(rng.range(WALKS.len() as u64)).expect("small");
    let (source_rel, ref_field, payload_field, target_rel, key_field, extra_field) = WALKS[idx];
    let source = b.atom(source_rel);
    let join = b.bind_var(source, ref_field);
    let payload = b.bind_var(source, payload_field);
    let target = b.atom(target_rel);
    b.bind(target, key_field, Term::Var(join));
    match rng.range(9) {
        // Near-miss (a third): the target produces output, so
        // condition 2 fails.
        0..=2 => {
            b.find_var(payload);
            let extra = b.bind_var(target, extra_field);
            b.find_var(extra);
            b.chase = Some(ChaseVariant::WalkExtraField);
        }
        // The aggregate sink over the eliminable walk: fold per join
        // key, the payload bound but unprojected.
        3 | 4 => {
            b.find_var(join);
            b.finds.push(FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            });
            b.chase = Some(ChaseVariant::Walk);
        }
        _ => {
            b.find_var(payload);
            b.chase = Some(ChaseVariant::Walk);
        }
    }
}

/// The discriminated-union one-sided walk over
/// `JournalEntry(id | source == Import) == ImportBatch(entry)`: the
/// header falls (child-to-header direction), the child falls
/// (header-to-child direction), or the missing-φ near-miss (the header
/// occurrence without `source == Import` — its facts are not all in
/// σφ, so the chase must refuse).
pub(super) fn du_walk(b: &mut Builder, rng: &mut Rng) {
    let import = Term::Literal(Value::Enum(SOURCE_IMPORT));
    match rng.range(3) {
        0 => {
            let child = b.atom(ids::IMPORT_BATCH);
            let join = b.bind_var(child, ids::import_batch::ENTRY);
            let payload = b.bind_var(child, ids::import_batch::BATCH);
            b.find_var(payload);
            let header = b.atom(ids::JOURNAL_ENTRY);
            b.bind(header, ids::journal_entry::ID, Term::Var(join));
            b.bind(header, ids::journal_entry::SOURCE, import);
            b.chase = Some(ChaseVariant::DuHeader);
        }
        variant => {
            let header = b.atom(ids::JOURNAL_ENTRY);
            let join = b.bind_var(header, ids::journal_entry::ID);
            let payload = b.bind_var(header, ids::journal_entry::CREATED_AT);
            b.find_var(payload);
            if variant == 1 {
                b.bind(header, ids::journal_entry::SOURCE, import);
                b.chase = Some(ChaseVariant::DuChild);
            } else {
                b.chase = Some(ChaseVariant::DuMissingPhi);
            }
            let child = b.atom(ids::IMPORT_BATCH);
            b.bind(child, ids::import_batch::ENTRY, Term::Var(join));
        }
    }
}
