//! A fixed interval width is at least 1.
//@ error: `Slot.span` has interval width 0: a fixed interval width is at least 1
//@ line: 10

bumbledb::schema! {
    pub Zeroed;

    relation Slot {
        playlist: u64,
        span:     interval<u64, 0>,
    }
}
