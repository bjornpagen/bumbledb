//! A misspelled field is rustc's unknown-field error at its token.
//@ error: no field `persn`
//@ line: 20

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
    bumbledb::query!(Cal {
        (p) | Busy(persn: p);
    })
    .into_query()
}
