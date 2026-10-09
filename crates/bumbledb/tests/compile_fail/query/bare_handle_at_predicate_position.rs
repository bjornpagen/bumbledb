//! A derived-table position has no field, so its handle is qualified.
//@ error: a derived-table position has no field
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
        interior mid(c, p) | Parent(child: c, parent: p);
        (x) | mid(0: x, 1 == Usd);
    })
    .into_query()
}
