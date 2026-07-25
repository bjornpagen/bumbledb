//! `{0..*}` is the vacuous window — it provably says nothing
//! (`lean/Bumbledb/Cardinality.lean: cardinality_zero_star`), and a
//! statement that says nothing is deleted, not defaulted.
//@ error: the `{0..*}` window is vacuous
//@ error: cardinality_zero_star

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={0..*} Task(parent);
}
