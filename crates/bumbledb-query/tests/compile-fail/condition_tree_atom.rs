//! A condition tree takes comparisons only (R9): atoms, negation, and
//! the binding membership stay body items — atoms disjoin by writing
//! rules. The line directive pins the span at the offending atom.
//@ error: a condition tree takes comparisons only
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
        (c) | Parent(child: c, parent: p),
              or(Parent(child: p),
                 c == 1);
    })
}
