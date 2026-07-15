//! The `order` statement form is deleted vocabulary — order is a
//! derivation, not a dependency (fractional indexing over a keyed
//! position, or the exact-partition interval recipe) — and the grammar
//! itself rejects the old spelling at expansion, never the validator
//! (`docs/architecture/30-dependencies.md` § refused: order marks).
//@ error: `order` statements no longer exist

bumbledb::schema! {
    pub Ledger;

    relation Task {
        parent: u64,
        pos:    u64,
    }

    order Task(pos) per Task(parent);
}
