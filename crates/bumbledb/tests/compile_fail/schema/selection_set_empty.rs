//! An empty literal set selects nothing.
//@ error: the literal set for `state` is empty
//@ line: 11

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64, state: u64 }

    Task(parent | state == {}) <= Parent(id);
}
