//! The empty window `<={}` names no bounds — banned (the
//! canonical-utterance law, `docs/architecture/70-api.md`: `{}` does not
//! parse): bounds are always explicit.
//@ error: the window `{}` names no bounds
//@ error: write `{n}`, `{lo..hi}`, or `{lo..*}`

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={} Task(parent);
}
