//! A non-u64, non-signed weight field (`str`) — nothing to measure:
//! a `[field]` weight reads a u64-encoded SOURCE position.
//!
//@ error: weight field `label` on `Device` is not
//@ error: u64-encoded

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, label: str }

    Pool(id) -> Pool;
    Pool(id) <=[label]{0..supply} Device(pool);
}
