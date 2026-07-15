//! A one-element literal set is the bare literal's second spelling —
//! banned (the canonical-utterance law, `docs/architecture/70-api.md`):
//! write `field == L`, no braces. (`DegenerateSelectionSet` is the same
//! law's descriptor face.)
//@ error: a one-element set is the bare literal
//@ error: write `state == L`, no braces

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64, state: u64 }

    Task(parent | state == {7}) <= Parent(id);
}
