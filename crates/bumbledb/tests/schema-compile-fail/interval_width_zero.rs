//! `interval<E, 0>` denotes nothing — a fixed-width interval is nonempty
//! by construction (`w >= 1`), so the zero width dies at expansion with
//! the field named, never at the validator
//! (`docs/architecture/10-data-model.md` § the admission rule).
//@ error: interval<E, 0> denotes nothing — the width must be >= 1

bumbledb::schema! {
    pub Zeroed;

    relation Slot {
        playlist: u64,
        span:     interval<u64, 0>,
    }
}
