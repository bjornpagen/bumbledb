//! A field is declared once per relation.
//@ error: `Task` declares the field `owner` twice
//@ line: 7

bumbledb::schema! {
    pub Ledger;
    relation Task { owner: u64, owner: i64 }
}
