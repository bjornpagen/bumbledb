use bumbledb::{Atom, RelationId};

/// # Panics
#[must_use]
pub fn builder_relation(atom: &Atom) -> RelationId {
    atom.source
        .edb()
        .expect("Builder atoms are stored-relation (Edb) by construction")
}
