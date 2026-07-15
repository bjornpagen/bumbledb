//! The `in lo..hi per` window spelling is deleted vocabulary — the
//! window is B-family, target-left (`Parent(key) <={lo..hi}
//! Child(field)`), and the grammar itself rejects the old spelling at
//! expansion, naming the canonical form (the canonical-utterance law,
//! `docs/architecture/70-api.md`).
//@ error: the `in lo..hi per` window form is deleted

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Task(parent) in 1..3 per Parent(id);
}
