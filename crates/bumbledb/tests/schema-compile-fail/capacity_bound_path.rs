//! The path spelling `{0..supply.max}` in a dependent bound — refused:
//! a dependent bound names a field of the TARGET's own row, closed at
//! the row exactly like the weight (ruled 2026-07-24, ruling 6), and
//! the diagnostic names the composition idiom — the SAME verdict the
//! weight bracket, the TS surface, and the spec resolver give a dotted
//! name (one spelling, one refusal, every authoring wall).
//@ error: the bound path `{..supply.…}` is refused
//@ error: the pinned-column idiom

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[watts]{0..supply.max} Device(pool);
}
