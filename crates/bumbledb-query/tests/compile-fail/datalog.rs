//! The refused grammar must not parse: Datalog's `head :- body` was
//! considered and rejected (the refusals ledger) — the notation is the
//! statement grammar's query side, promoted.
//@ error: refused

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
        (p) :- Busy(person: p);
    })
}
