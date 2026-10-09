//! Handles are unique within a closed relation.
//@ error: closed relation `Status` declares the handle `Frozen` twice
//@ line: 8

bumbledb::schema! {
    pub Review;

    closed relation Status as StatusId = { Open, Frozen, Frozen };
}
