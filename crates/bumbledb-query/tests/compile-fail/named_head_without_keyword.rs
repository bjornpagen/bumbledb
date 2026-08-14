//! A named head without `interior` / `recursive` is the former named-head
//! sneak — refused at the name, telling the author to write the keyword.
//@ error: named heads require `interior` or `recursive`
//@ line: 17

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        reach(c, a) | Parent(child: c, parent: a);
        reach(c, a) | Parent(child: c, parent: m), reach(m, a);
        (c, a) | reach(c, a);
    })
}
