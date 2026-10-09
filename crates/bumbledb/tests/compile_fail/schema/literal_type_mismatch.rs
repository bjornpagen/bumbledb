//! A row literal fits its column's type.
//@ error: the literal does not fit `Kind.mastered`'s declared type
//@ line: 11

bumbledb::schema! {
    pub Review;

    closed relation Kind as KindId {
        mastered: bool,
    } = {
        DirectPass { mastered: 3 },
        Failed     { mastered: false },
    };
}
