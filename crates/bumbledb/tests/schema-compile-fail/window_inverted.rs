//! `{hi..lo}` with hi > lo is inverted — no count satisfies it; the
//! grammar rejects the unsatisfiable spelling at expansion, naming the
//! canonical bounds.
//@ error: the window `{4..2}` is inverted
//@ error: bounds are `{lo..hi}` with lo < hi

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={4..2} Task(parent);
}
