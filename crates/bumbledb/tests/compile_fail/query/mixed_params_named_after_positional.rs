//! Params are named or positional, never both.
//@ error: named and positional ?params cannot mix
//@ line: 18

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
              c == ?0,
              p == ?root;
    })
    .into_query()
}
