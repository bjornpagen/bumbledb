//! A key closes over its own relation.
//@ error: a key closes over its own relation: `Task(parent) -> Task`
//@ line: 11

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Task(parent) -> Parent;
}
