//! The empty literal set `{}` selects nothing — banned (the
//! canonical-utterance law, `docs/architecture/70-api.md`: `{}` does not
//! parse): an unselected side is written with no binding at all.
//@ error: the literal set for `state` is empty
//@ error: an empty set selects nothing; write no binding

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64, state: u64 }

    Task(parent | state == {}) <= Parent(id);
}
