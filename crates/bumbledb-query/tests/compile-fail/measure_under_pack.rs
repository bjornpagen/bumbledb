//! The measure folds under Sum/Min/Max only — `Pack(Duration(v))` has
//! no IR shape (the coalescing fold is interval-in, interval-out), and
//! the refusal is the macro's, spanned at the measure.
//@ error: the measure folds under Sum/Min/Max only
//@ line: 18

bumbledb::schema! {
    pub Org;

    relation Mandate {
        org: u64,
        active: interval<u64>,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        (org, Pack(Duration(active))) |
            Mandate(org, active);
    })
}
