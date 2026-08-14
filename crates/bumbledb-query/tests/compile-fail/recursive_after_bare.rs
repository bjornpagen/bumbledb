//! Declaration order is interiors, then rec, then main.
//@ error: `recursive` cannot follow a bare rule
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
        (c) | Parent(child: c, parent: p);
        recursive reach(c) | Parent(child: c, parent: p);
        recursive reach(c) | Parent(child: c, parent: p), reach(p);
    })
}
