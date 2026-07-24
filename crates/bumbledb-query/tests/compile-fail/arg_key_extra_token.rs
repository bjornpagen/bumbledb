//! The Arg argument group is closed after the key — `key := v |
//! Duration(v)` seals the pair (ruled 2026-07-23, R5), a third position
//! has no grammar, and the refusal is the macro's, spanned at the stray
//! token.
//@ error: ArgMax/ArgMin take a carried variable and a key
//@ line: 19

bumbledb::schema! {
    pub Org;

    relation Mandate {
        org: u64,
        active: interval<u64>,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        (ArgMax(org, Duration(active), org)) |
            Mandate(org, active);
    })
}
