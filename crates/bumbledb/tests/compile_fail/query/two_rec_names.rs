//! A query has at most one rec.
//@ error: at most one `rec` name
//@ line: 17

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb::query!(Org {
        rec a(c) | Parent(child: c, parent: p);
        rec b(c) | Parent(child: c, parent: p), b(p);
        (c) | a(c);
    })
    .into_query()
}
