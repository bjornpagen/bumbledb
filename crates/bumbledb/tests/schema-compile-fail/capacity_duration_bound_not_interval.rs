//! `{0..Duration(field)}` over a scalar — the Duration bound reads a
//! TARGET interval position's measure
//! (`docs/architecture/30-dependencies.md` § dependent bounds).
//@ error: bound field `supply` on `Pool` is not
//@ error: bounds by a TARGET interval's measure

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[watts]{0..Duration(supply)} Device(pool);
}
