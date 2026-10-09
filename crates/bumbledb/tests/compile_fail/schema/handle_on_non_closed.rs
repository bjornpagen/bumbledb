//! A bare handle needs a field that references a closed relation.
//@ error: `Task.kind` is not a closed-relation reference
//@ line: 11

bumbledb::schema! {
    pub Board;

    relation Task { owner: u64, kind: u64 }
    relation Done { task: u64 }

    Done(task) <= Task(owner | kind == Frozen);
}
