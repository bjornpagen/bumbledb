//! Body items are comma-separated.
//@ error: expected `,` or `;`
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
        (c, p) | Parent(child: c, parent: p)
                 c < p;
    })
    .into_query()
}
