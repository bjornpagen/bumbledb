//! A closed relation declares at least one row.
//@ error: the closed relation `Status` declares no rows
//@ line: 7

bumbledb::schema! {
    pub Review;
    closed relation Status as StatusId = {};
}
