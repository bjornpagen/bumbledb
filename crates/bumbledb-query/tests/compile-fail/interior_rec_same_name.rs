//! Derived names are unique: an interior and the rec cannot share a name.
//@ error: cannot be both `interior` and `rec`
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
        interior reach(c) | Parent(child: c, parent: p);
        rec reach(c) | Parent(child: c, parent: p);
        rec reach(c) | Parent(child: c, parent: p), reach(p);
        (c) | reach(c);
    })
}
