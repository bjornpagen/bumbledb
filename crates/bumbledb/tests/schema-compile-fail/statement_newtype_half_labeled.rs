//! The coherence check's half-labeled arm: a labeled face never pairs
//! with a bare one — bare pairs only with bare (the TS wall's own law,
//! adopted so the two hosts judge identically;
//! `docs/architecture/30-dependencies.md` § the taxonomy is checked).
//@ error: the containment pairs `Task.owner` (`PersonId`) with `Person.id` (no newtype)
//@ error: the faces of a dependency agree on their newtype, or neither carries one

bumbledb::schema! {
    pub Roster;

    relation Person { id: u64 }
    relation Task   { owner: u64 as PersonId }

    Task(owner) <= Person(id);
}
