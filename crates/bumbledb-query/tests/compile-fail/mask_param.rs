//! Allen masks are literals — a `?param` in the mask position has no
//! grammar, and the refusal is the macro's, spanned at the `?`.
//@ error: Allen masks are literals
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
        (org) |
            Mandate(org, active),
            Allen(active, ?mask, active);
    })
}
