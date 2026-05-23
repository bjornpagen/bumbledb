use super::*;
use crate::benchmark::{benchmark_facts, benchmark_queries, benchmark_schema};
use crate::{InputBindings, STORAGE_FORMAT_VERSION, Value};
use bumbledb_core::schema::{FieldDescriptor, RelationDescriptor, ValueType};

const MARKER_KEY: &[u8] = b"test_marker";
type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn opens_initializes_and_reopens_metadata() -> TestResult {
    let dir = tempfile::tempdir()?;

    let env = Environment::open(dir.path())?;
    assert_eq!(env.storage_format_version()?, STORAGE_FORMAT_VERSION);
    assert_eq!(env.max_readers(), DEFAULT_MAX_READERS);
    assert!(env.max_key_size() > 0);
    drop(env);

    let env = Environment::open(dir.path())?;
    assert_eq!(env.storage_format_version()?, STORAGE_FORMAT_VERSION);
    Ok(())
}

#[test]
fn rejects_v3_storage_format_after_fact_id_break() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;
    env.write(|txn| txn.put_meta_bytes(STORAGE_FORMAT_VERSION_KEY, &3u32.to_be_bytes()))?;
    drop(env);

    assert!(matches!(
        Environment::open(dir.path()),
        Err(Error::Open(crate::OpenError::StorageFormatMismatch { expected, found }))
            if expected == STORAGE_FORMAT_VERSION && found == 3
    ));
    Ok(())
}

#[test]
fn write_commits_on_success() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;

    env.write(|txn| {
        txn.put_meta_bytes(MARKER_KEY, b"committed")?;
        Ok::<(), Error>(())
    })?;

    let marker = env.read(|txn| txn.get_meta_bytes(MARKER_KEY))?;
    assert_eq!(marker.as_deref(), Some(&b"committed"[..]));
    Ok(())
}

#[test]
fn write_aborts_on_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;

    let result: Result<()> = env.write(|txn| {
        txn.put_meta_bytes(MARKER_KEY, b"aborted")?;
        Err(Error::internal("intentional abort"))
    });

    assert!(result.is_err());
    let marker = env.read(|txn| txn.get_meta_bytes(MARKER_KEY))?;
    assert_eq!(marker, None);
    Ok(())
}

#[test]
fn read_snapshot_is_stable_across_later_commit() -> TestResult {
    let dir = tempfile::tempdir()?;
    let env = Environment::open(dir.path())?;

    env.write(|txn| {
        txn.put_meta_bytes(MARKER_KEY, b"before")?;
        Ok::<(), Error>(())
    })?;

    env.read(|read| {
        assert_eq!(
            read.get_meta_bytes(MARKER_KEY)?.as_deref(),
            Some(&b"before"[..])
        );

        env.write(|write| {
            write.put_meta_bytes(MARKER_KEY, b"after")?;
            Ok::<(), Error>(())
        })?;

        assert_eq!(
            read.get_meta_bytes(MARKER_KEY)?.as_deref(),
            Some(&b"before"[..])
        );
        Ok::<(), Error>(())
    })?;

    let marker = env.read(|txn| txn.get_meta_bytes(MARKER_KEY))?;
    assert_eq!(marker.as_deref(), Some(&b"after"[..]));
    Ok(())
}

#[test]
fn bulk_load_new_matches_fact_by_fact_results() -> TestResult {
    let facts = benchmark_facts(5);
    let schema = StorageSchema::new(benchmark_schema(), 511)?;

    let fact_dir = tempfile::tempdir()?;
    let fact_env = Environment::open_with_schema(fact_dir.path(), &schema)?;
    fact_env.write(|txn| {
        for fact in &facts {
            txn.insert(&schema, fact.clone())?;
        }
        Ok::<(), Error>(())
    })?;

    let bulk_dir = tempfile::tempdir()?;
    let (bulk_env, report) = Environment::bulk_load_new(bulk_dir.path(), &schema, facts)?;
    assert_eq!(report.facts_inserted, benchmark_facts(5).len());
    assert!(report.dictionary_entries > 0);

    let typed = (benchmark_queries()[0].build)(schema.descriptor())?;
    let inputs = InputBindings::from_values([
        ("holder", Value::Serial(1)),
        (
            "start",
            Value::Timestamp(bumbledb_core::encoding::TimestampMicros(0)),
        ),
        (
            "end",
            Value::Timestamp(bumbledb_core::encoding::TimestampMicros(1000)),
        ),
    ]);
    let fact_result = fact_env
        .read(|txn| txn.execute_query(&schema, &typed, &inputs))?
        .result
        .facts;
    let bulk_result = bulk_env
        .read(|txn| txn.execute_query(&schema, &typed, &inputs))?
        .result
        .facts;
    assert_eq!(sorted_facts(fact_result), sorted_facts(bulk_result));
    Ok(())
}

#[test]
fn bulk_load_duplicate_facts_count_inserted_once() -> TestResult {
    let schema = StorageSchema::new(benchmark_schema(), 511)?;
    let dir = tempfile::tempdir()?;
    let env = Environment::open_with_schema(dir.path(), &schema)?;
    let mut facts = benchmark_facts(2);
    let distinct = facts.len();
    facts.push(facts[0].clone());

    let report = env.bulk_load(&schema, facts)?;
    assert_eq!(report.facts_inserted, distinct);

    let diagnostics = env.storage_diagnostics(&schema)?;
    assert_eq!(diagnostics.storage_tx_id, 1);
    assert_eq!(
        diagnostics
            .relations
            .iter()
            .map(|relation| relation.fact_count)
            .sum::<u64>(),
        distinct as u64
    );
    Ok(())
}

#[test]
fn schema_mismatch_fails_without_destroying_data() -> TestResult {
    let schema = StorageSchema::new(benchmark_schema(), 511)?;
    let dir = tempfile::tempdir()?;
    let env = Environment::open_with_schema(dir.path(), &schema)?;
    env.bulk_load(&schema, benchmark_facts(2))?;
    drop(env);

    let changed = StorageSchema::new(changed_schema(), 511)?;
    assert!(matches!(
        Environment::open_with_schema(dir.path(), &changed),
        Err(Error::Schema(crate::SchemaError::SchemaMismatch { .. }))
    ));

    let env = Environment::open_with_schema(dir.path(), &schema)?;
    let diagnostics = env.storage_diagnostics(&schema)?;
    assert!(
        diagnostics
            .relations
            .iter()
            .any(|relation| relation.relation == "Posting" && relation.fact_count > 0)
    );
    Ok(())
}

#[test]
fn bulk_load_target_must_be_new_and_large_fixture_reopens() -> TestResult {
    let schema = StorageSchema::new(benchmark_schema(), 511)?;
    let dir = tempfile::tempdir()?;
    let (env, report) = Environment::bulk_load_new(dir.path(), &schema, benchmark_facts(12))?;
    assert!(report.facts_inserted > 50);
    assert!(report.dictionary_entries >= 12);
    drop(env);

    assert!(matches!(
        Environment::bulk_load_new(dir.path(), &schema, benchmark_facts(1)),
        Err(Error::Storage(
            crate::StorageError::BulkLoadTargetExists { .. }
        ))
    ));

    let env = Environment::open_with_schema(dir.path(), &schema)?;
    let diagnostics = env.storage_diagnostics(&schema)?;
    assert!(diagnostics.lmdb_map_size > 0);
    assert!(diagnostics.storage_tx_id > 0);
    Ok(())
}

fn sorted_facts(mut facts: Vec<Vec<Value>>) -> Vec<Vec<Value>> {
    facts.sort();
    facts
}

fn changed_schema() -> bumbledb_core::schema::SchemaDescriptor {
    let mut schema = benchmark_schema();
    schema.relations.push(
        RelationDescriptor::new(
            "Extra",
            vec![FieldDescriptor::new(
                "id",
                ValueType::Serial {
                    type_name: "ExtraId".to_owned(),
                    owning_relation: "Extra".to_owned(),
                },
            )],
        )
        .with_unique("id", ["id"]),
    );
    schema
}
