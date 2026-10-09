//! Derived-table names begin lowercase.
//@ error: derived-table names begin lowercase
//@ line: 16

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb::query!(Org {
        Reach(c, a) | Parent(child: c, parent: a);
        (c, a) | Reach(c, a);
    })
    .into_query()
}
