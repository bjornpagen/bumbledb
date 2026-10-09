//! Dense in-order derived-table bindings are written bare.
//@ error: dense in-order derived-table bindings are written bare
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
        (c, a) | reach(0: c, 1: a);
    })
    .into_query()
}
