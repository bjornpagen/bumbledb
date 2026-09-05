//! Bare idents and indexed labels never mix in one interior/rec atom:
//! ordered dense bindings are all bare (`reach(m, a)`); sparse and
//! selection bindings are all indexed (`2: x`, `0 == …`).
//@ error: bare idents and indexed labels cannot mix
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
        (c, a) | reach(c, 1: a);
    })
    .into_query()
}
