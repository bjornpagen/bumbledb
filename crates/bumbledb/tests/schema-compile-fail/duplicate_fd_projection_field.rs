//! A determinant is a field set — duplicate-free
//! (`docs/architecture/30-dependencies.md` § functionality): a repeated
//! projection field dies as a teaching error spanned at the second
//! occurrence, never as rustc's E0124 on the generated key struct's
//! field the author never wrote.
//@ error: `kind` appears twice in the determinant of `Task(kind, kind) -> Task`

bumbledb::schema! {
    pub Board;

    relation Task {
        kind:    u64,
        subject: u64,
    }

    Task(kind, kind) -> Task;
}
