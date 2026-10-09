//! A weight reads its own row: a path is refused at the dot.
//@ error: `model.…` is refused
//@ line: 12

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, model: u64, watts: u64 }

    Pool(id) -> Pool;
    Pool(id) <=[model.watts]{0..supply} Device(pool);
}
