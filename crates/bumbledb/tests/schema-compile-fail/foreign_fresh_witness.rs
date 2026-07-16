//! The schema-bound witness law (`70-api.md` § ETL): a `FreshField`
//! witness carries its resolving handle's schema typestate, so a witness
//! of one schema cannot reach another schema's transaction — the compile
//! half of the cross-schema lock (the dyn-boundary half is the typed
//! refusal pinned by `a_foreign_witness_is_refused_typed_not_minted`).
//@ error: mismatched types
//@ error: FreshField

bumbledb::schema! {
    pub Home;

    relation Counter {
        id: u64 as CounterId, fresh,
        v: u64,
    }
}

bumbledb::schema! {
    pub Away;

    relation Tally {
        id: u64 as TallyId, fresh,
        v: u64,
    }
}

pub fn cross_schema_mint(
    home: &bumbledb::Db<Home>,
    away: &bumbledb::Db<Away>,
) -> bumbledb::Result<u64> {
    let witness = away
        .fresh_field(bumbledb::RelationId(0), bumbledb::FieldId(0))
        .expect("fresh in its own schema");
    home.write(|tx| tx.alloc_at(witness))
}
