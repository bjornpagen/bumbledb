//! A fixed interval width is at least 1.
//@ error: an interval width is at least 1
//@ line: 10

bumbledb::schema! {
    pub Zeroed;

    relation Slot {
        playlist: u64,
        span:     interval<u64, 0>,
    }
}
