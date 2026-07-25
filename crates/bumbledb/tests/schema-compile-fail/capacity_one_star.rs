//! Unit `{1..*}` says only what the bare containment says — the ban
//! table's first line (the canonical-utterance law,
//! `docs/architecture/70-api.md`): drop the annotation, write `X <= Y`.
//! The ban is UNIT-ONLY (the per-aggregate law): the weighted
//! `<=[w]{1..*}` — "positive total" — is legal, pinned as a positive
//! probe in `tests/schema_macro.rs` and the valid suite's
//! `a_weighted_floor_of_one_validates`.
//@ error: `{1..*}` says only what the bare containment says
//@ error: Parent(…) <= Task(…)

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={1..*} Task(parent);
}
