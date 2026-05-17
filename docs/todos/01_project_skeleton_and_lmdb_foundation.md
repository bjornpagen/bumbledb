# 01: Project Skeleton And LMDB Foundation

**Goal**
- Establish the Rust workspace, LMDB wrapper boundary, error model, and database open/create path.

**Why This Stage Exists**
- Everything depends on a safe, boring foundation around LMDB.
- We need to prove the embedded shape early: open a path, initialize metadata, start read and write transactions, and close cleanly.
- This stage should avoid query, schema macro, and planner complexity.

**Concrete Work**
- Create the Rust workspace with the initial crate layout.
- Add a public facade crate and an internal LMDB/storage crate boundary.
- Pick the LMDB Rust binding or decide to wrap `liblmdb` directly.
- Implement `Database::open(path)` or equivalent internal open path.
- Create the fixed DBI set: `_meta`, `_index`, and `_dict`.
- Set internal LMDB defaults for mapsize, max DBs, max readers, durability flags, and read transaction behavior.
- Store and read storage format version metadata.
- Define the top-level error enum with storage, schema, query, constraint, and internal variants.
- Implement read and write closure APIs internally, even if not yet exposed as final generated APIs.
- Add reader cleanup hook using LMDB reader checking if available through the chosen wrapper.
- Add a minimal smoke test that creates, closes, reopens, reads metadata, and performs a no-op write transaction.

**Out Of Scope**
- No schema macro.
- No Datalog parser.
- No relation storage.
- No covering indexes.
- No query execution.
- No bulk loader.
- No benchmarks beyond smoke timing if useful.

**Passing Criteria**
- `cargo test` passes for the workspace.
- A temporary database directory can be created, opened, closed, and reopened.
- The database stores a storage format version in `_meta`.
- Opening an existing database verifies the storage format version.
- The internal write closure commits on success and aborts on error.
- The internal read closure sees a consistent snapshot.
- LMDB named DBIs are opened from a fixed internal list, not dynamically per relation or index.
- Unsafe LMDB interaction is isolated behind a small module boundary.
- No public API exposes LMDB handles, raw cursors, or raw transactions.

**Notes**
- Do not introduce tuning knobs in this stage.
- Do not overdesign crate boundaries if one crate is faster initially, but preserve the intended layering.
- If the chosen LMDB wrapper cannot support the required lifetime or transaction model cleanly, stop and reassess the wrapper before continuing.
