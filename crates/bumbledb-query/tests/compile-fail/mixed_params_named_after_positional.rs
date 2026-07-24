//! Params are one style per query — the mirror direction: a named param
//! after a positional one, spanned at the offending `?name` token.
//@ error: named and positional ?params cannot mix
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
        (c) | Parent(child: c, parent: p),
              c == ?0,
              p == ?root;
    })
}
