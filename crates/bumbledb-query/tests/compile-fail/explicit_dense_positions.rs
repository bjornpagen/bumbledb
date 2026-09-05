//! Dense in-order interior/rec bindings are written bare (`reach(m, a)`)
//! — the ordered form is the one dense spelling, so an explicitly
//! indexed dense in-order variable list is refused.
//@ error: dense in-order interior/rec bindings are written bare
//@ line: 19

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        interior reach(c, a) | Parent(child: c, parent: a);
        (c, a) | reach(0: c, 1: a);
    })
    .into_query()
}
