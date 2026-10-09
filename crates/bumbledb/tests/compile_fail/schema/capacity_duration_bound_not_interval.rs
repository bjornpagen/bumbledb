//! `Duration(field)` bounds by an interval field.
//@ error: `Duration(supply)` reads an interval, and `Pool.supply` is not one
//@ line: 12

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[watts]{0..Duration(supply)} Device(pool);
}
