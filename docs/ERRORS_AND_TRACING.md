# Errors And Tracing

This project uses layered public errors and opt-in `tracing` instrumentation.

**Error Taxonomy**
- `OpenError`: environment setup and storage format metadata.
- `SchemaError`: schema descriptors, key layout validation, and schema fingerprints.
- `StorageError`: LMDB operations, dictionary storage, counters, and ETL target checks.
- `TransactionError`: read/write transaction lifecycle and reader cleanup.
- `ConstraintError`: duplicate tuples, missing tuples, unique constraints, foreign keys, restrict deletes, required fields, and write-time type mismatches.
- `QueryError`: planning, execution, and aggregation failures.
- `BackupError`: backup and compact-copy operations.
- `CorruptionError`: malformed persisted data or invalid decoded storage state.
- `InternalError`: impossible engine states and invariant failures.

All public error enums are `#[non_exhaustive]`.

**User-Correctable Errors**
- Unknown relation or field.
- Missing required field.
- Type mismatch.
- Duplicate tuple.
- Unique violation.
- Foreign-key violation.
- Restrict-delete violation.
- Missing query input.
- Query input type mismatch.
- Parse/typecheck diagnostics.
- Schema mismatch requiring ETL.

**Operational Errors**
- Filesystem IO.
- LMDB operation failures.
- Reader cleanup failures.
- Backup or compact-copy failures.
- Bulk-load target already exists.

**Corruption And Internal Errors**
- `CorruptionError` means persisted state does not match the storage format or schema layout.
- `InternalError` means an engine invariant was violated and should generally be treated as a bug.

**Tracing Policy**
- Library crates use `tracing` spans and events.
- Library crates never initialize a tracing subscriber.
- Applications, tests, and benchmarks choose their own subscriber.
- Normal execution does not require tracing.
- Per-row details are `trace` level only.

**Important Span Names**
- `bumbledb.open`
- `bumbledb.open_with_schema`
- `bumbledb.open_fixed_databases`
- `bumbledb.verify_schema`
- `bumbledb.read_txn`
- `bumbledb.write_txn`
- `bumbledb.commit`
- `bumbledb.bulk_load`
- `bumbledb.storage.bulk_load`
- `bumbledb.storage.segment_publish`
- `bumbledb.insert`
- `bumbledb.replace`
- `bumbledb.delete`
- `bumbledb.dict_intern`
- `bumbledb.query.plan`
- `bumbledb.query.execute`
- `bumbledb.query.project`
- `bumbledb.query.aggregate`
- `bumbledb.query_image.build`
- `bumbledb.query_image.relation`
- `bumbledb.backup`
- `bumbledb.compact_copy`

**Benchmark Tracing**
The benchmark binary can initialize a subscriber with `--trace`:

```sh
RUST_LOG=bumbledb_lmdb=debug cargo run -p bumbledb-bench --release -- --trace --dataset joinstress --scale 2000 --repeats 10
```

Use `debug` for plan and operator summaries. Use `trace` only for focused investigation because it can produce very large output.

**Testing Expectations**
- Tests should match structured error variants rather than parse display strings.
- Query failures should preserve parse/typecheck spans where available.
- Storage/LMDB failures should retain source errors.
- Tracing should be safe to leave disabled during benchmarks.
