//! A signed weight could lower a sum on insert.
//@ error: the weight field `Device.drift` is not u64; a signed weight
//@ line: 12

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, drift: i64 }

    Pool(id) -> Pool;
    Pool(id) <=[drift]{0..supply} Device(pool);
}
