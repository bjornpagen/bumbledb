//! A dependent bound names a field of the target's row.
//@ error: relation `Pool` has no field `voltage`
//@ line: 12

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[watts]{0..voltage} Device(pool);
}
