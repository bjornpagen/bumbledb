//! A one-element set is written as the bare literal.
//@ error: a one-element set is the bare literal
//@ line: 11

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64, state: u64 }

    Task(parent | state == {7}) <= Parent(id);
}
