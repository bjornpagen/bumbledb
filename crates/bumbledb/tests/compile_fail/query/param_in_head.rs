//! A param cannot be a result column.
//@ error: a ?param cannot appear in a head
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
        (?window) | Busy(person: p);
    })
    .into_query()
}
