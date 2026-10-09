//! A dependent bound reads its own row: a path is refused at the dot.
//@ error: `supply.…` is refused
//@ line: 12

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[watts]{0..supply.max} Device(pool);
}
