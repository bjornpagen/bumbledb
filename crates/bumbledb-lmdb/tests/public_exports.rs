#![allow(clippy::result_large_err)]

use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb_lmdb::{
    BulkLoadReport, DeleteOutcome, Environment, Fact, InputBindings, InsertOutcome, QueryResultSet,
    ResultColumn, STORAGE_FORMAT_VERSION, StorageSchema, Value,
};

#[test]
fn public_exports_cover_current_contract_without_raw_internals()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("bumbledb-public-exports-{}", std::process::id()));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let schema = StorageSchema::new(
        SchemaDescriptor::new(
            "PublicExports",
            vec![RelationDescriptor::new(
                "R",
                vec![FieldDescriptor::new("x", ValueType::U64)],
            )],
        ),
        511,
    )?;
    let env = Environment::open_with_schema(&path, &schema)?;
    let fact = Fact::new("R", [("x", Value::U64(1))]);
    let inputs = InputBindings::new();
    let result = QueryResultSet::new(
        vec![ResultColumn::Variable("x".to_owned())],
        vec![vec![Value::U64(1)]],
    );
    let report = BulkLoadReport::default();

    assert_eq!(STORAGE_FORMAT_VERSION, 6);
    assert!(inputs.is_empty());
    assert_eq!(result.cardinality(), 1);
    assert_eq!(report.facts_inserted, 0);
    assert_eq!(
        env.write(|txn| txn.insert(&schema, &fact))?,
        InsertOutcome::Inserted
    );
    assert_eq!(env.read(|txn| txn.relation_fact_count(&schema, "R"))?, 1);
    assert_eq!(
        env.write(|txn| txn.delete(&schema, &Fact::new("R", [("x", Value::U64(2))])))?,
        DeleteOutcome::Absent
    );
    std::fs::remove_dir_all(path)?;
    Ok(())
}
