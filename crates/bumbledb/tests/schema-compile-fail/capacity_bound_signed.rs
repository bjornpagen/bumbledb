//! A signed dependent bound — a dependent bound reads a u64 field of
//! the TARGET's row: a signed encoding cannot bound a non-negative
//! measure.
//!
//@ error: bound field `margin` on `Pool` is signed

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, margin: i64 }
    relation Device { pool: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[watts]{0..margin} Device(pool);
}
