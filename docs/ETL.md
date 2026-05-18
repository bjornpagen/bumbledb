# ETL And Operational Policy

This database does not support migrations.

Schema changes are handled by full ETL into a new database path. The old database is opened with the old binary/schema, application code transforms rows, and the new database is bulk-loaded with the new binary/schema.

**Rules**
- `Environment::open_with_schema` stores the schema fingerprint on first schema-aware open.
- Opening with a different schema fingerprint fails with `SchemaMismatch`.
- Schema mismatch never rewrites, upgrades, downgrades, deletes, or recreates data.
- `Environment::bulk_load_new` refuses to target a path that already contains `data.mdb`.
- Bulk load runs in one write transaction; any row, dictionary, index, stats, history, or constraint failure aborts the whole load.
- Backup and compact copy are explicit operations into another database directory.
- There is no online migration, in-place vacuum, replication, encryption, or unsafe durability mode in v0.

**Map Size**
- v0 uses an internal fixed LMDB map size and exposes no tuning knobs.
- If the map is exhausted, LMDB surfaces the write failure and the transaction aborts.
- A future stage may add safe automatic map growth, but stage 08 treats map exhaustion as a bounded storage error rather than partial success.
