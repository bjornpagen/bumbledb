//! `and` and `or` are reserved.
//@ error: `or` is reserved
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
        or(c, a) | Parent(child: c, parent: a);
        (c, a) | or(c, a);
    })
    .into_query()
}
