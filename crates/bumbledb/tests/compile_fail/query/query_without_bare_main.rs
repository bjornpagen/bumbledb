//! A query needs a main rule.
//@ error: a query needs a main rule
//@ line: 15

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb::query!(Org {
        rec reach(c, a) | Parent(child: c, parent: a);
        rec reach(c, a) | Parent(child: c, parent: m), reach(m, a);
    })
    .into_query()
}
