//! A relation's fields are named, not numbered.
//@ error: a relation's fields are named
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
        (c) | Parent(0: c,
                     parent: p);
    })
    .into_query()
}
