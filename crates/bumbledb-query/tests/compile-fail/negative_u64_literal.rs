//! Literals type by their own spelling: negative is `i64`, so a `u64`
//! suffix on a negative literal is a contradiction the macro refuses at
//! the token.
//@ error: a negative literal cannot carry `u64`
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
              p == -5u64;
    })
}
