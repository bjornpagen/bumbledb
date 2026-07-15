//! `interval<E, >` names no width — the trailing comma is neither the
//! general spelling nor a fixed one, so the grammar refuses it at
//! expansion with the field named and both legal spellings offered.
//@ error: `interval<E, >` names no width

bumbledb::schema! {
    pub Widthless;

    relation Slot {
        playlist: u64,
        span:     interval<u64, >,
    }
}
