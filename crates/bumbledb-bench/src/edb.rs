//! Builder-internal EDB construction. `Atom` is not EDB-only — the IR
//! source is a match (`Edb` / `Interior`). The CQ assembler still
//! constructs stored-relation atoms only; that premise lives here,
//! not as a trait on `Atom`.

use bumbledb::{Atom, RelationId};

/// The stored relation of a CQ-`Builder` atom.
///
/// # Panics
///
/// On `AtomSource::Interior` — the assembler never constructs one.
#[must_use]
pub fn builder_relation(atom: &Atom) -> RelationId {
    atom.source
        .edb()
        .expect("Builder atoms are stored-relation (Edb) by construction")
}
