//! A row names declared columns only.
//@ error: closed relation `Kind` has no column `weight`
//@ line: 11

bumbledb::schema! {
    pub Review;

    closed relation Kind as KindId {
        mastered: bool,
    } = {
        DirectPass { mastered: true, weight: 3 },
        Failed     { mastered: false },
    };
}
