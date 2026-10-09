//! A rec needs a base arm.
//@ error: has no base arm
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
        rec reach(c) | Parent(child: c, parent: p), reach(p);
        (c) | reach(c);
    })
    .into_query()
}
