//! The case partition is total in BOTH directions: lowercase names are
//! interiors or the rec, so a relation respelled lowercase is an unknown
//! derived table, never a silent resolution to the UpperCamel constants —
//! `uppercase_predicate_name.rs` is the mirror fixture.
//@ error: unknown derived table `parent`
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
    .into_query()
}
