//! An inverted literal window is refused.
//@ error: the window `{4..2}` is inverted
//@ line: 11

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={4..2} Task(parent);
}
