//! Numeric labels address a predicate atom's head positions —
//! positional, never nominal. A relation's fields are named, so a
//! position label on a relation atom is refused at the label.
//@ error: a relation's fields are named
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
        (c) | Parent(0: c,
                     parent: p);
    })
}
