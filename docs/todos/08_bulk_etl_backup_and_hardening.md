# 08: Bulk ETL, Backup, And Hardening

**Goal**
- Turn the working v0 engine into something practical for embedded use: bulk load, ETL migration path, backup, compaction copy, and durability hardening.

**Why This Stage Exists**
- The project explicitly rejects migrations, so ETL must be a first-class workflow.
- Embedded users still need backup, rebuild, and confidence around abort/crash behavior.

**Concrete Work**
- Implement a bulk loader that can build current indexes efficiently from sorted input.
- Use LMDB append modes where safe and justified by sorted keys.
- Validate bulk-loaded constraints before committing final data.
- Provide an ETL-oriented API for creating a new database from application-provided rows.
- Implement backup using LMDB environment copy capabilities where available.
- Implement compact-copy-to-new-path if practical.
- Add schema mismatch tests and document the ETL-only migration policy in user-facing docs.
- Add abort and reopen tests around partially failed bulk loads.
- Add stress tests for larger dictionaries, larger relation indexes, and map growth.
- Harden error messages for storage format mismatch, schema mismatch, key size rejection, and LMDB map errors.

**Out Of Scope**
- Online migrations.
- Automatic schema upgrade.
- In-place vacuum.
- Unsafe durability modes.
- Replication.
- Encryption.

**Passing Criteria**
- A new database can be bulk-loaded from a realistic ledger data set.
- Bulk-loaded data produces the same query results as row-by-row inserted data.
- Bulk load preserves constraints or fails atomically.
- Schema mismatch fails safely and never destroys data.
- Backup creates a usable copy.
- Compact copy creates a usable database when implemented.
- Map growth behavior is tested or clearly bounded by documented errors.
- Dictionary interning remains correct under bulk load.
- ETL is documented as the official migration path.
- The engine can survive repeated open, write, abort, backup, and reopen cycles in tests.

**Notes**
- This stage is about operational confidence, not adding query features.
- Keep backup and compaction explicit.
- Do not introduce online migration machinery by accident.
