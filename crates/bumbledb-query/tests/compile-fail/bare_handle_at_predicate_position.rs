//! A bare handle resolves through the FIELD-named host enum, and an
//! interior/rec head position has no field name — the qualified spelling
//! is the one writable form at an indexed position.
//@ error: an interior/rec position has no field name
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
        interior mid(c, p) | Parent(child: c, parent: p);
        (x) | mid(0: x, 1 == Usd);
    })
}
