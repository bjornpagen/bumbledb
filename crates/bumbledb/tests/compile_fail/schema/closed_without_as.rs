//! A closed relation needs a handle newtype.
//@ error: closed relation `Status` needs a handle newtype
//@ line: 8

bumbledb::schema! {
    pub Review;

    closed relation Status = { Open, Frozen, Closed };
}
