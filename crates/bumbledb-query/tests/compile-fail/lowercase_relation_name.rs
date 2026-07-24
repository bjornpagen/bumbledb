//! The case partition is total in BOTH directions: lowercase names are
//! predicates, so a relation respelled lowercase is an unknown
//! predicate, never a silent resolution to the UpperCamel constants —
//! `uppercase_predicate_name.rs` is the mirror fixture.
//@ error: unknown predicate `parent`
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
        (child) | parent(child, parent: p);
    })
}
