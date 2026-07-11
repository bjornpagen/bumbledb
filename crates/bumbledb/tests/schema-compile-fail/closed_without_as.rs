//! `as NewType` is required on a closed relation: the handle needs a host
//! type.
//@ error: closed relation `Status` needs `as NewType`

bumbledb::schema! {
    pub Review;

    closed relation Status = { Open, Frozen, Closed };
}
