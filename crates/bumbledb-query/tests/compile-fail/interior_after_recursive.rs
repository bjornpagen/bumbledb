//! Declaration order is interiors, then rec, then main.
//@ error: `interior` cannot follow `recursive`
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
        recursive reach(c) | Parent(child: c, parent: p);
        recursive reach(c) | Parent(child: c, parent: p), reach(p);
        interior mid(c) | Parent(child: c, parent: p);
        (c) | reach(c);
    })
}
