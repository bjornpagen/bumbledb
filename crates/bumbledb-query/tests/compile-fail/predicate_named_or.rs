//! `and` and `or` are the condition grammar's reserved words (R9): a
//! body-position `or(…)` is always a tree, so an interior/rec taking either
//! name would be unreadable — refused at its declaration.
//@ error: is the condition grammar's reserved word
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
        or(c, a) | Parent(child: c, parent: a);
        (c, a) | or(c, a);
    })
    .into_query()
}
