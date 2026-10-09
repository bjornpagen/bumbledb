//! The rec precedes the main rules.
//@ error: `rec` cannot follow a main rule
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
        (c) | Parent(child: c, parent: p);
        rec reach(c) | Parent(child: c, parent: p);
        rec reach(c) | Parent(child: c, parent: p), reach(p);
    })
    .into_query()
}
