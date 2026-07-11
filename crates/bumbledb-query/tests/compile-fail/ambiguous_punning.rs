//! The punning law (B): the same punned name in two atoms of one clause
//! is a macro error, spanned at the second occurrence — the line
//! directive pins the span.
//@ error: ambiguous punning — rename explicitly
//@ line: 23

bumbledb::schema! {
    pub Cal;

    relation Busy {
        person: u64,
        during: interval<u64>,
    }
    relation Ooo {
        person: u64,
        during: interval<u64>,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Cal {
        (person) | Busy(person, during: d),
                   Ooo(person);
    })
}
