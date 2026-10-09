//! A dependent bound reads a u64 field.
//@ error: bound field `Pool.margin` is not u64
//@ line: 12

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, margin: i64 }
    relation Device { pool: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[watts]{0..margin} Device(pool);
}
