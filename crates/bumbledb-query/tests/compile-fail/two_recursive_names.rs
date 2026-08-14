//! At most one `recursive` name this cut.
//@ error: at most one `recursive` name
//@ line: 17

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        recursive a(c) | Parent(child: c, parent: p);
        recursive b(c) | Parent(child: c, parent: p), b(p);
        (c) | a(c);
    })
}
