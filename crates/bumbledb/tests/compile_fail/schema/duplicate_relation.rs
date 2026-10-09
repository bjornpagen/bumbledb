//! A relation is declared once.
//@ error: the relation `Task` is declared twice
//@ line: 8

bumbledb::schema! {
    pub Ledger;
    relation Task { id: u64 }
    relation Task { owner: u64 }
}
