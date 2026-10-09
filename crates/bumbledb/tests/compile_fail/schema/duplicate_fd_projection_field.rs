//! A determinant is a set of fields.
//@ error: `kind` appears twice in the determinant of `Task(kind, kind) -> Task`
//@ line: 13

bumbledb::schema! {
    pub Board;

    relation Task {
        kind:    u64,
        subject: u64,
    }

    Task(kind, kind) -> Task;
}
