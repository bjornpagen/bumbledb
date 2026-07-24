//! The empty combinations (`And([])` true, `Or([])` false) keep their
//! algebraic readings in the IR, but the notation does not spell them —
//! a tree node takes at least one condition.
//@ error: takes at least one condition

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        (c) | Parent(child: c, parent: p), and();
    })
}
