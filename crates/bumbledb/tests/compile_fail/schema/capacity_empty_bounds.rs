//! A window states its bounds.
//@ error: the window `{}` names no bounds
//@ line: 11

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={} Task(parent);
}
