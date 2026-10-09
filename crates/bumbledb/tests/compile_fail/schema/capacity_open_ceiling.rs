//! A window states its floor.
//@ error: a window states its floor: write `{0..hi}`
//@ line: 11

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={..5} Task(parent);
}
