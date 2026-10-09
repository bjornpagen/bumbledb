//! `[Duration(field)]` weighs by an interval field.
//@ error: `Duration(watts)` reads an interval, and `Device.watts` is not one
//@ line: 12

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[Duration(watts)]{0..supply} Device(pool);
}
