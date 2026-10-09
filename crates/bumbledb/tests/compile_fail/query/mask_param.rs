//! An Allen mask is a literal.
//@ error: an Allen mask is a literal
//@ line: 18

bumbledb::schema! {
    pub Org;

    relation Mandate {
        org: u64,
        active: interval<u64>,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb::query!(Org {
        (org) |
            Mandate(org, active),
            Allen(active, ?mask, active);
    })
    .into_query()
}
