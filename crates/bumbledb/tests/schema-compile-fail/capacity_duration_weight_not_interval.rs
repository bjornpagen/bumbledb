//! `[Duration(field)]` over a scalar — the interval-measure weight
//! reads an interval position of the SOURCE row
//! (`docs/architecture/30-dependencies.md` § weight typing).
//@ error: weight field `watts` on `Device` is not
//@ error: interval-typed

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[Duration(watts)]{0..supply} Device(pool);
}
