//! A typo'd relation is a compile error at the query literal: the
//! expansion emits `Cal::BUZY`, which does not exist — ordinary rustc
//! name resolution is the checker (the id-constants trick).
//@ error: BUZY

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
        (p) | Buzy(person: p);
    })
}
