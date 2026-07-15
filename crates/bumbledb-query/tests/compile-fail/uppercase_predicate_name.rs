//! Predicate names begin lowercase: relations are UpperCamel, so the
//! case split is what makes a predicate spelled like a relation
//! unwritable (the punning law's discipline, applied to names).
//@ error: predicate names begin lowercase
//@ line: 18

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Program {
    bumbledb_query::query!(Org {
        Reach(c, a) | Parent(child: c, parent: a);
        (c, a) | Reach(0: c, 1: a);
    })
}
