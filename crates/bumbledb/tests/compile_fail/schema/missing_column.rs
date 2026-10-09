//! Every row supplies every declared column.
//@ error: row `Failed` is missing the column `mastered`
//@ line: 12

bumbledb::schema! {
    pub Review;

    closed relation Kind as KindId {
        mastered: bool,
    } = {
        DirectPass { mastered: true },
        Failed,
    };
}
