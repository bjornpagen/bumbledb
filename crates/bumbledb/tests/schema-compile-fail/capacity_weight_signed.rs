//! A signed weight field — the typed polarity refusal: a negative
//! weight would let an insert lower a sum, breaking the delta
//! scheduler, so the illegal weight is unrepresentable at expansion,
//! not checked at judge time.
//!
//@ error: weight field `drift` on `Device` is signed
//@ error: refused by polarity

bumbledb::schema! {
    pub Grid;

    relation Pool   { id: u64, supply: u64 }
    relation Device { pool: u64, drift: i64 }

    Pool(id) -> Pool;
    Pool(id) <=[drift]{0..supply} Device(pool);
}
