//! The harness's Edb-only convenience over the pure-data IR. The
//! engine retired its panicking `Atom::relation` accessor (the
//! Lean-modeled `Atom.relation → Atom.source` cut; finding 111) — the
//! total form lives here instead, where its premise actually holds:
//! the generators and oracles construct stored-relation atoms only.

use bumbledb::{Atom, RelationId};

pub trait EdbAtom {
    /// Panics on an `Interior` atom —
    /// none exists on the paths that import this trait.
    fn relation(&self) -> RelationId;
}

impl EdbAtom for Atom {
    fn relation(&self) -> RelationId {
        self.source
            .edb()
            .expect("harness atoms are stored-relation (Edb) by construction")
    }
}
