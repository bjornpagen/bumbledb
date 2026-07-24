//! A binding's `in` is set membership and takes a ?param bound to a
//! set — interval membership is the `==` typing rule (on an interval
//! field) or the item-position `point in interval`.
//@ error: a binding's `in` takes a ?param bound to a set
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
        (c) | Parent(child: c,
                     parent in 5);
    })
}
