//! A duplicate handle is an expansion error naming the handle — rows are
//! ground axioms and the handle is the row's identity.
//@ error: closed relation `Status` declares the handle `Frozen` twice

bumbledb::schema! {
    pub Review;

    closed relation Status as StatusId = { Open, Frozen, Frozen };
}
