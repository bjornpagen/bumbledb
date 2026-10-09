//! A determinant is a set of fields.
//@ error: `Task.kind` is projected twice
//@ line: 13

bumbledb::schema! {
    pub Board;

    relation Task {
        kind:    u64,
        subject: u64,
    }

    Task(kind, kind) -> Task;
}
