//! A `[field]` weight reads a u64 field.
//@ error: weight field `Device.label` is not u64
//@ line: 12

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, label: str }

    Pool(id) -> Pool;
    Pool(id) <=[label]{0..supply} Device(pool);
}
