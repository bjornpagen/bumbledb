//! A window states its ceiling.
//@ error: write `{lo..*}` for a floor
//@ line: 11

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={2..} Task(parent);
}
