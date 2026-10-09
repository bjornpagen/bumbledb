//! `and(…)` and `or(…)` take at least one condition.
//@ error: takes at least one condition
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
        (c) | Parent(child: c, parent: p), and();
    })
    .into_query()
}
