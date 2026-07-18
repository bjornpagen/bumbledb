//! The key arrow closes over its own relation — `R(X) -> R` is the
//! functional-dependency spelling (the key projection determines the
//! tuple; `docs/architecture/30-dependencies.md`, owner ruling
//! 2026-07-18: the arrow is canon, never respelled). A right side naming
//! a DIFFERENT relation is not a key statement: a teaching
//! `compile_error!` spanned at the offending relation name, never a bare
//! assertion.
//@ error: the key arrow closes over its own relation: `Task(parent) -> Task`
//@ error: `-> Parent` is not a key statement

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Task(parent) -> Parent;
}
