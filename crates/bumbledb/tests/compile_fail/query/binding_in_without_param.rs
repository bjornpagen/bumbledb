//! A binding's `in` takes a set param.
//@ error: a binding's `in` takes a ?param bound to a set
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
        (c) | Parent(child: c,
                     parent in 5);
    })
    .into_query()
}
