//! A closed relation's rows are named by their handles, never by text.
//@ error: `Kind.label` is str on a closed relation
//@ line: 8

bumbledb::schema! {
    pub Review;

    closed relation Kind as KindId { label: str } = {
        Pass { label: "pass" },
    };
}
