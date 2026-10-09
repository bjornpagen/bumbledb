//! A dependent bound is a ceiling, never a floor.
//@ error: a dependent bound in the floor slot
//@ line: 12

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[watts]{supply..9000} Device(pool);
}
