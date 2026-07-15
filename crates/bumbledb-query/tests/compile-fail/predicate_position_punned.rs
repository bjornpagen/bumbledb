//! A predicate atom's bindings address head positions — positional,
//! never nominal, never punned: a pun names a field, and predicate
//! columns have none.
//@ error: a predicate position binds explicitly
//@ line: 19

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Program {
    bumbledb_query::query!(Org {
        reach(c, a) | Parent(child: c, parent: a);
        (c) | reach(c);
    })
}
