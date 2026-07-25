//! `{n..n}` is the exact measure's second spelling — banned; the
//! canonical exact-measure spelling is `{n}` (the canonical-utterance
//! law, `docs/architecture/70-api.md`).
//@ error: an exact measure is written `{2}`

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={2..2} Task(parent);
}
