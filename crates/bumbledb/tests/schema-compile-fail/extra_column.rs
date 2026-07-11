//! A row naming an undeclared column is an expansion error naming it.
//@ error: row `DirectPass` of closed relation `Kind` names an extra column `weight`

bumbledb::schema! {
    pub Review;

    closed relation Kind as KindId {
        mastered: bool,
    } = {
        DirectPass { mastered: true, weight: 3 },
        Failed     { mastered: false },
    };
}
