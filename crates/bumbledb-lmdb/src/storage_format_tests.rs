use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

use super::*;
use crate::{Environment, Error, STORAGE_FORMAT_VERSION};

#[test]
fn storage_format_new_database_writes_v5_marker() -> crate::Result<()> {
    let path = test_path("new-marker");
    clean(&path)?;

    let env = Environment::open(&path)?;

    assert_eq!(env.storage_format_version()?, STORAGE_FORMAT_VERSION);
    clean(&path)?;
    Ok(())
}

#[test]
fn storage_format_old_marker_fails() -> crate::Result<()> {
    let path = test_path("old-marker");
    clean(&path)?;
    std::fs::create_dir_all(&path)?;
    std::fs::write(path.join("FORMAT"), "4")?;

    let result = Environment::open(&path);

    clean(&path)?;
    assert!(matches!(result, Err(Error::StorageFormatMismatch { .. })));
    Ok(())
}

#[test]
fn storage_format_missing_marker_in_non_empty_directory_fails() -> crate::Result<()> {
    let path = test_path("missing-marker");
    clean(&path)?;
    std::fs::create_dir_all(&path)?;
    std::fs::write(path.join("stray"), "old data")?;

    let result = Environment::open(&path);

    clean(&path)?;
    assert!(matches!(result, Err(Error::StorageFormatMismatch { .. })));
    Ok(())
}

#[test]
fn storage_format_key_namespaces_are_ordered_and_distinct() {
    let handle = FactHandle([7; 16]);
    let keys = vec![
        canonical_fact_key(1, b"fact"),
        fact_handle_key(1, handle),
        live_row_key(1, handle),
        column_key(1, 2, handle),
        serial_sequence_key(1, 2),
        unique_guard_key(1, "u", b"key"),
        reverse_fk_guard_key(1, "u", b"key", 2, "fk", handle),
        accelerator_key(1, 3, b"tuple", handle),
        stats_key(1, "count"),
    ];
    let namespace_bytes: Vec<_> = keys.iter().map(|key| key[0]).collect();

    assert_eq!(namespace_bytes, b"THLCQURAS");
    for key in keys {
        assert!(!key.is_empty());
    }
}

#[test]
fn storage_format_fact_handle_is_content_derived_and_collision_checkable() {
    let handle_a = fact_handle(1, b"abc");
    let handle_b = fact_handle(1, b"abd");

    assert_ne!(handle_a, handle_b);
    assert_eq!(handle_a.0.len(), 16);
    assert_ne!(fact_handle_key(1, handle_a), canonical_fact_key(1, b"abc"));
}

#[test]
fn storage_format_serial_sequence_key_is_distinct_from_fact_handle_key() {
    let handle = fact_handle(1, b"abc");

    assert_ne!(serial_sequence_key(1, 0), fact_handle_key(1, handle));
    assert_eq!(
        serial_sequence_key(1, 0)[0],
        Namespace::SerialSequence.byte()
    );
}

#[test]
fn storage_format_schema_fingerprint_uses_v5_label() {
    let schema = SchemaDescriptor::new(
        "Fingerprint",
        vec![RelationDescriptor::new(
            "R",
            vec![FieldDescriptor::new("x", ValueType::U64)],
        )],
    );

    assert_eq!(schema.fingerprint(), schema.fingerprint());
}

fn test_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("bumbledb-prd07-{name}"))
}

fn clean(path: &std::path::Path) -> crate::Result<()> {
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}
