//! Interior names are unique; non-consecutive reuse is a compile error.
//@ error: interior `mid` is not consecutive
//@ line: 18

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        interior mid(c) | Parent(child: c, parent: p);
        interior other(c) | Parent(child: c, parent: p);
        interior mid(c) | Parent(child: c, parent: p);
        (c) | mid(c);
    })
    .into_query()
}
