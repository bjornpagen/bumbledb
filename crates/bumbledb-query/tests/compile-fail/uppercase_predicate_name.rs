//! Derived-table names begin lowercase: relations are UpperCamel, so the
//! case split is what makes an interior/rec spelled like a relation
//! unwritable (the punning law's discipline, applied to names).
//@ error: derived-table names begin lowercase
//@ line: 18

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        Reach(c, a) | Parent(child: c, parent: a);
        (c, a) | Reach(c, a);
    })
    .into_query()
}
