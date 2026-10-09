//! A labeled face never pairs with a bare one.
//@ error: pairs `Task.owner` (`PersonId`) with `Person.id` (no newtype)
//@ line: 11

bumbledb::schema! {
    pub Roster;

    relation Person { id: u64 }
    relation Task   { owner: u64 as PersonId }

    Task(owner) <= Person(id);
}
