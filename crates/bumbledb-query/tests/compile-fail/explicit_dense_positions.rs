//! Dense in-order predicate bindings are written bare (`reach(m, a)`)
//! — the ordered form is the one dense spelling, so an explicitly
//! indexed dense in-order variable list is refused.
//@ error: dense in-order predicate bindings are written bare
//@ line: 19

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Program {
    bumbledb_query::query!(Org {
        reach(c, a) | Parent(child: c, parent: a);
        (c, a) | reach(0: c, 1: a);
    })
}
