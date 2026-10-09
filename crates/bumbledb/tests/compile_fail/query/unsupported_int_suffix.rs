//! Integer literals are u64 or i64.
//@ error: `5u32` is not a supported literal
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
              p == 5u32;
    })
    .into_query()
}
