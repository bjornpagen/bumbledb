//! `interval<E, >` names no width.
//@ error: expected the interval width, found `>`
//@ line: 10

bumbledb::schema! {
    pub Widthless;

    relation Slot {
        playlist: u64,
        span:     interval<u64, >,
    }
}
