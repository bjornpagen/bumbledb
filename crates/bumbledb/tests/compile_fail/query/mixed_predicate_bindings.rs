//! Bare variables and position labels never mix in one atom.
//@ error: bare variables and position labels cannot mix
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
        interior reach(c, a) | Parent(child: c, parent: a);
        (c, a) | reach(c, 1: a);
    })
    .into_query()
}
