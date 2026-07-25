//! The path spelling `[model.watts]` in the weight bracket — refused:
//! the weight vocabulary is closed at the row (ruled 2026-07-24,
//! ruling 6), and the diagnostic names the composition idiom — the
//! two-column containment IS the join, stated as a law, with the
//! capacity reading the pinned local column.
//@ error: the weight path `[model.…]` is refused
//@ error: the pinned-column idiom
//@ error: Device(model, watts) <= Model(id, watts)

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, model: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[model.watts]{0..supply} Device(pool);
}
