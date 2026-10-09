//! A misspelled relation is rustc's unresolved-name error at its token.
//@ error: named `Buzy` found for struct `Cal`
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
        (p) | Buzy(person: p);
    })
    .into_query()
}
