//! A named head needs `interior` or `rec`.
//@ error: named heads require `interior` or `rec`
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
        reach(c, a) | Parent(child: c, parent: a);
        reach(c, a) | Parent(child: c, parent: m), reach(m, a);
        (c, a) | reach(c, a);
    })
    .into_query()
}
