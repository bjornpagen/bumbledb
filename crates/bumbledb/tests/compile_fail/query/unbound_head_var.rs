//! A head variable is bound in the body.
//@ error: head variable `q` is not bound in the rule body
//@ line: 16

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb::query!(Org {
        (c, q) |
            Parent(child: c, parent: p);
    })
    .into_query()
}
