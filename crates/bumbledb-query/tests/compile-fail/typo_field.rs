//! A typo'd field is a compile error at the query literal: the expansion
//! emits `Cal::BUSY_PERSN`, which does not exist.
//@ error: BUSY_PERSN

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
        (p) | Busy(persn: p);
    })
}
