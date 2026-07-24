//! Params are one style per query: named or positional, never mixed.
//! The refusal spans the offending positional token (`Param::Index`
//! carries its span — finding 121), not the whole invocation.
//@ error: named and positional ?params cannot mix
//@ line: 20

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
              c == ?root,
              p == ?0;
    })
}
