//! A dependent bound naming no field of TARGET's row — bound idents
//! resolve by NAME against the target's whole field roster (ruled
//! 2026-07-24, C1), and an unknown name is the ordinary unresolvable
//! field, marked at the window.
//@ error: relation `Pool` has no field `voltage`

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[watts]{0..voltage} Device(pool);
}
