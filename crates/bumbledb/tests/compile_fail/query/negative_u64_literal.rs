//! A negative literal cannot be u64.
//@ error: the literal does not fit u64
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
        (c) | Parent(child: c, parent: p),
              p == -5u64;
    })
    .into_query()
}
