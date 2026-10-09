//! A side does not both select and project one field.
//@ error: `Task.kind` is both selected and projected
//@ line: 11

bumbledb::schema! {
    pub Ledger;
    relation Kind { id: u64 }
    relation Task { kind: u64 }

    Kind(id) -> Kind;
    Task(kind | kind == 1) <= Kind(id);
}
