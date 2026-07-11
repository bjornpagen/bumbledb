//! Every declared column is present in each row exactly once — a bare
//! handle on a columned relation panics naming the missing column.
//@ error: row `Failed` of closed relation `Kind` is missing the column `mastered`

bumbledb::schema! {
    pub Review;

    closed relation Kind as KindId {
        mastered: bool,
    } = {
        DirectPass { mastered: true },
        Failed,
    };
}
