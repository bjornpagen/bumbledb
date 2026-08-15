//! A `rec` block needs at least one rec arm after classification
//! (a line whose body names the rec).
//@ error: has no rec arm

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        rec reach(c) | Parent(child: c, parent: p);
        (c) | reach(c);
    })
}
