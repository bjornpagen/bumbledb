//! A bare handle resolves through the FIELD-named host enum, and a
//! predicate head position has no field name — the qualified spelling
//! is the one writable form at an indexed position.
//@ error: a predicate position has no field name
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
        pred(c, p) | Parent(child: c, parent: p);
        (x) | pred(0: x, 1 == Usd);
    })
}
