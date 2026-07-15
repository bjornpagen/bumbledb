//! `{0..0}` is the exclusion's second spelling — banned; the exclusion
//! is written `{0}` (the canonical-utterance law,
//! `docs/architecture/70-api.md`).
//@ error: the exclusion is written `{0}`

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={0..0} Task(parent);
}
