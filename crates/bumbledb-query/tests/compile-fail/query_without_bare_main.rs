//! A query needs a bare main rule: `interior` / `rec` declare
//! derived tables; the answer is the unnamed rules.
//@ error: a query needs a bare main rule

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        rec reach(c, a) | Parent(child: c, parent: a);
        rec reach(c, a) | Parent(child: c, parent: m), reach(m, a);
    })
}
