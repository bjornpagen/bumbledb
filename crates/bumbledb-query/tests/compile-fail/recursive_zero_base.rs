//! A `recursive` block needs at least one base arm after classification
//! (a line whose body does not name the pred).
//@ error: has no base arm

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        recursive reach(c) | Parent(child: c, parent: p), reach(p);
        (c) | reach(c);
    })
}
