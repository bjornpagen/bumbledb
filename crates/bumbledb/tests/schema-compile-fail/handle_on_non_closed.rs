//! A bare handle resolves through the selected field's newtype to its
//! owning closed relation; on any other field it is an expansion error.
//@ error: `Task.kind` is not a closed-relation reference

bumbledb::schema! {
    pub Board;

    relation Task { owner: u64, kind: u64 }
    relation Done { task: u64 }

    Done(task) <= Task(owner | kind == Frozen);
}
