//! Row literals ride the same typing machine as statement selections —
//! same machine, same errors.
//@ error: the literal for `Kind.mastered` does not fit the field's declared type

bumbledb::schema! {
    pub Review;

    closed relation Kind as KindId {
        mastered: bool,
    } = {
        DirectPass { mastered: 3 },
        Failed     { mastered: false },
    };
}
