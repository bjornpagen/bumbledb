//! A `?param` cannot appear in a head: params are execution inputs, not
//! result columns.
//@ error: a ?param cannot appear in a head

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
        (?window) | Busy(person: p);
    })
}
