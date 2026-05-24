use bumbledb_core::schema::{
    ConstraintDescriptor, EnumDescriptor, FieldDescriptor, RelationDescriptor, SchemaDescriptor,
    ValueType,
};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{DeleteOutcome, Environment, Error, Fact, InsertOutcome, Result, StorageSchema, Value};

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn storage_duplicate_insert_is_noop() -> Result<()> {
    let (env, schema) = env_and_schema("duplicate-insert")?;
    let fact = holder(1, "alice");

    assert_eq!(
        env.write(|txn| txn.insert(&schema, &fact))?,
        InsertOutcome::Inserted
    );
    let tx_id = env.read(|txn| txn.storage_tx_id())?;
    assert_eq!(
        env.write(|txn| txn.insert(&schema, &fact))?,
        InsertOutcome::AlreadyPresent
    );

    assert_eq!(env.read(|txn| txn.storage_tx_id())?, tx_id);
    assert_eq!(
        env.read(|txn| txn.relation_fact_count(&schema, "Holder"))?,
        1
    );
    Ok(())
}

#[test]
fn storage_generates_omitted_serial_field() -> Result<()> {
    let (env, schema) = env_and_schema("generated-serial")?;

    env.write(|txn| {
        txn.insert(
            &schema,
            &Fact::new("Holder", [("name", Value::String("alice".to_owned()))]),
        )
    })?;

    let facts = env.read(|txn| txn.debug_relation_facts(&schema, "Holder"))?;
    assert_eq!(facts[0].value("id"), Some(&Value::Serial(1)));
    Ok(())
}

#[test]
fn storage_explicit_serial_advances_high_water() -> Result<()> {
    let (env, schema) = env_and_schema("explicit-serial")?;

    env.write(|txn| txn.insert(&schema, &holder(41, "alice")))?;
    env.write(|txn| {
        txn.insert(
            &schema,
            &Fact::new("Holder", [("name", Value::String("bob".to_owned()))]),
        )
    })?;

    let facts = env.read(|txn| txn.debug_relation_facts(&schema, "Holder"))?;
    assert!(
        facts
            .iter()
            .any(|fact| fact.value("id") == Some(&Value::Serial(42)))
    );
    Ok(())
}

#[test]
fn storage_aborted_generated_serial_does_not_advance_sequence() -> Result<()> {
    let (env, schema) = env_and_schema("aborted-serial")?;

    let aborted: Result<()> = env.write(|txn| {
        txn.insert(
            &schema,
            &Fact::new("Holder", [("name", Value::String("alice".to_owned()))]),
        )?;
        Err(Error::invalid_query("abort"))
    });
    assert!(aborted.is_err());
    env.write(|txn| {
        txn.insert(
            &schema,
            &Fact::new("Holder", [("name", Value::String("bob".to_owned()))]),
        )
    })?;

    let facts = env.read(|txn| txn.debug_relation_facts(&schema, "Holder"))?;
    assert_eq!(facts[0].value("id"), Some(&Value::Serial(1)));
    Ok(())
}

#[test]
fn storage_absent_delete_is_noop() -> Result<()> {
    let (env, schema) = env_and_schema("absent-delete")?;

    assert_eq!(
        env.write(|txn| txn.delete(&schema, &holder(1, "alice")))?,
        DeleteOutcome::Absent
    );

    assert_eq!(env.read(|txn| txn.storage_tx_id())?, 0);
    Ok(())
}

#[test]
fn storage_delete_then_reinsert() -> Result<()> {
    let (env, schema) = env_and_schema("delete-reinsert")?;
    let fact = holder(1, "alice");

    env.write(|txn| txn.insert(&schema, &fact))?;
    assert_eq!(
        env.write(|txn| txn.delete(&schema, &fact))?,
        DeleteOutcome::Deleted
    );
    assert_eq!(
        env.write(|txn| txn.insert(&schema, &fact))?,
        InsertOutcome::Inserted
    );

    assert_eq!(
        env.read(|txn| txn.relation_fact_count(&schema, "Holder"))?,
        1
    );
    Ok(())
}

#[test]
fn storage_unique_violation_rejects_conflict() -> Result<()> {
    let (env, schema) = env_and_schema("unique")?;

    env.write(|txn| txn.insert(&schema, &holder(1, "alice")))?;
    let result = env.write(|txn| txn.insert(&schema, &holder(2, "alice")));

    assert!(matches!(result, Err(Error::UniqueViolation { .. })));
    Ok(())
}

#[test]
fn storage_fk_violation_rejects_missing_target() -> Result<()> {
    let (env, schema) = env_and_schema("fk")?;

    let result = env.write(|txn| txn.insert(&schema, &pet(1, 999, 1)));

    assert!(matches!(result, Err(Error::ForeignKeyViolation { .. })));
    Ok(())
}

#[test]
fn storage_restrict_delete_rejects_referenced_target() -> Result<()> {
    let (env, schema) = env_and_schema("restrict")?;

    env.write(|txn| txn.insert(&schema, &holder(1, "alice")))?;
    env.write(|txn| txn.insert(&schema, &pet(1, 1, 1)))?;
    let result = env.write(|txn| txn.delete(&schema, &holder(1, "alice")));

    assert!(matches!(result, Err(Error::RestrictViolation { .. })));
    Ok(())
}

#[test]
fn storage_bulk_load_rolls_back_on_invalid_row() -> Result<()> {
    let (env, schema) = env_and_schema("bulk-rollback")?;

    let result = env.bulk_load(&schema, [holder(1, "alice"), holder(2, "alice")]);

    assert!(matches!(result, Err(Error::UniqueViolation { .. })));
    assert_eq!(
        env.read(|txn| txn.relation_fact_count(&schema, "Holder"))?,
        0
    );
    Ok(())
}

#[test]
fn storage_failpoints_abort_before_commit_leaves_no_partial_state() -> Result<()> {
    let (env, schema) = env_and_schema("failpoint-abort")?;

    let result: Result<()> = env.write(|txn| {
        txn.insert(&schema, &holder(1, "alice"))?;
        Err(Error::invalid_query("synthetic failpoint before commit"))
    });

    assert!(result.is_err());
    assert_eq!(
        env.read(|txn| txn.relation_fact_count(&schema, "Holder"))?,
        0
    );
    assert_eq!(env.read(|txn| txn.dictionary_entry_count())?, 0);
    Ok(())
}

#[test]
fn storage_reopen_verifies_counts_and_facts() -> Result<()> {
    let (env, schema) = env_and_schema("reopen")?;
    let path = env.path().to_path_buf();

    env.write(|txn| txn.insert(&schema, &holder(1, "alice")))?;
    drop(env);
    let reopened = Environment::open_with_schema(&path, &schema)?;

    assert_eq!(
        reopened.read(|txn| txn.relation_fact_count(&schema, "Holder"))?,
        1
    );
    assert_eq!(
        reopened.read(|txn| txn.debug_relation_facts(&schema, "Holder"))?[0].value("name"),
        Some(&Value::String("alice".to_owned()))
    );
    clean(&path)?;
    Ok(())
}

#[test]
fn storage_concurrency_read_snapshot_survives_concurrent_write() -> Result<()> {
    let (env, schema) = env_and_schema("snapshot")?;

    env.write(|txn| txn.insert(&schema, &holder(1, "alice")))?;
    env.read(|read| {
        assert_eq!(read.relation_fact_count(&schema, "Holder")?, 1);
        env.write(|write| write.insert(&schema, &holder(2, "bob")))?;
        assert_eq!(read.relation_fact_count(&schema, "Holder")?, 1);
        Ok::<(), Error>(())
    })?;
    assert_eq!(
        env.read(|txn| txn.relation_fact_count(&schema, "Holder"))?,
        2
    );
    Ok(())
}

#[test]
fn storage_interns_long_string_and_bytes_values() -> Result<()> {
    let (env, schema) = env_and_schema("long-interned-values")?;
    let long_name = "x".repeat(10_000);
    let long_bytes = vec![7; 10_000];

    env.write(|txn| {
        txn.insert(&schema, &holder(1, &long_name))?;
        txn.insert(&schema, &blob(1, long_bytes.clone()))?;
        Ok::<(), Error>(())
    })?;

    let holder = env.read(|txn| txn.debug_relation_facts(&schema, "Holder"))?;
    let blob = env.read(|txn| txn.debug_relation_facts(&schema, "Blob"))?;

    assert_eq!(holder[0].value("name"), Some(&Value::String(long_name)));
    assert_eq!(blob[0].value("payload"), Some(&Value::Bytes(long_bytes)));
    Ok(())
}

fn env_and_schema(name: &str) -> Result<(Environment, StorageSchema)> {
    let path = test_path(name);
    clean(&path)?;
    let schema = StorageSchema::new(schema(), 511)?;
    let env = Environment::open_with_schema(path, &schema)?;
    Ok((env, schema))
}

fn schema() -> SchemaDescriptor {
    SchemaDescriptor::new(
        "StorageV5",
        vec![
            RelationDescriptor::new(
                "Holder",
                vec![
                    FieldDescriptor::generated_serial("id", "HolderId", "Holder"),
                    FieldDescriptor::new("name", ValueType::String),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::unique("name", ["name"])),
            RelationDescriptor::new(
                "Pet",
                vec![
                    FieldDescriptor::generated_serial("id", "PetId", "Pet"),
                    FieldDescriptor::new("holder", serial("HolderId", "Holder")),
                    FieldDescriptor::new(
                        "kind",
                        ValueType::Enum {
                            name: "Kind".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "holder",
                ["holder"],
                "Holder",
                "id",
            )),
            RelationDescriptor::new(
                "Blob",
                vec![
                    FieldDescriptor::generated_serial("id", "BlobId", "Blob"),
                    FieldDescriptor::new("payload", ValueType::Bytes),
                ],
            )
            .with_unique("id", ["id"]),
        ],
    )
    .with_enum(EnumDescriptor::codes("Kind", [1, 2]))
}

fn holder(id: u64, name: &str) -> Fact {
    Fact::new(
        "Holder",
        [
            ("id", Value::Serial(id)),
            ("name", Value::String(name.to_owned())),
        ],
    )
}

fn pet(id: u64, holder: u64, kind: u8) -> Fact {
    Fact::new(
        "Pet",
        [
            ("id", Value::Serial(id)),
            ("holder", Value::Serial(holder)),
            ("kind", Value::Enum(kind)),
        ],
    )
}

fn blob(id: u64, payload: Vec<u8>) -> Fact {
    Fact::new(
        "Blob",
        [
            ("id", Value::Serial(id)),
            ("payload", Value::Bytes(payload)),
        ],
    )
}

fn serial(type_name: &str, owning_relation: &str) -> ValueType {
    ValueType::Serial {
        type_name: type_name.to_owned(),
        owning_relation: owning_relation.to_owned(),
    }
}

fn test_path(name: &str) -> std::path::PathBuf {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("bumbledb-prd08-{name}-{}-{id}", std::process::id()))
}

fn clean(path: &std::path::Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}
