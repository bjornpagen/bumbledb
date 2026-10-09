//! Lowercase names are derived tables, never relations.
//@ error: unknown derived table `parent`
//@ line: 16

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb::query!(Org {
        (child) | parent(child, parent: p);
    })
    .into_query()
}
