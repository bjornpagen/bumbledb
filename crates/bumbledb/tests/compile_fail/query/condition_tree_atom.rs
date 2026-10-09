//! A condition tree holds comparisons only.
//@ error: a condition tree takes comparisons only
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
              or(Parent(child: p),
                 c == 1);
    })
    .into_query()
}
