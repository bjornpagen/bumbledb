//! The value sum holds exactly two integer types: `u64` and `i64` are
//! the only suffixes (the magnitude is rustc's — radix prefixes and `_`
//! separators per R8 — but a `u32` value is unrepresentable).
//@ error: is not an integer literal
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
              p == 5u32;
    })
}
