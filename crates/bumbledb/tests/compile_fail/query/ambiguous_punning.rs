//! A field punned twice in one rule is refused at the second pun.
//@ error: `person` is punned twice
//@ line: 21

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
        (person) | Busy(person, during: d),
                   Ooo(person);
    })
    .into_query()
}
