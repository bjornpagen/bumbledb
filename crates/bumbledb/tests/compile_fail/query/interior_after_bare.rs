//! Interiors precede the main rules.
//@ error: `interior` cannot follow a main rule
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
        interior mid(c) | Parent(child: c, parent: p);
    })
    .into_query()
}
