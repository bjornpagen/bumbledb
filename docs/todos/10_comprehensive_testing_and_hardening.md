# 10: Comprehensive Testing And Hardening

**Goal**
- Build a comprehensive correctness, concurrency, crash, fuzz, and differential test suite before adding recursive Datalog or other deferred semantic features.

**Why This Stage Exists**
- The engine now has enough behavior to hide subtle bugs: typed schemas, LMDB transactions, dictionaries, indexes, constraints, Datalog parsing/typechecking, query execution, aggregation, ETL, backup, and compact copy.
- Recursive rules and as-of queries will multiply the state space, so the v0 surface needs a serious test harness first.
- This stage turns the Rosetta Stone testing philosophy into concrete, reusable test infrastructure.

**Scope Decision**
- Include property tests, differential tests, thread-based concurrency tests, failpoint-driven atomicity tests, compile-fail lifetime tests, benchmark correctness comparisons, and documentation.
- Include subprocess crash/recovery tests in this stage behind ignored/expensive test flags.
- Add fuzz targets scaffolding in this stage, but do not require long fuzz runs for normal `cargo test` completion.
- Do not add recursive rules, as-of queries, query macros, prepared query caches, or other deferred query semantics while doing this stage.

**Test Support Crate**
- Add a dev/test support crate, tentatively `crates/bumbledb-test-support`.
- Move reusable test-only fixtures out of product modules when practical.
- Keep product crates focused on engine code and small local unit tests.

**Test Support Modules**
- `schemas.rs`: canonical ledger schemas, narrow edge schemas, overflow schemas, invalid schema helpers.
- `rows.rs`: deterministic row constructors and benchmark row generation.
- `reference.rs`: in-memory reference database and Datalog evaluator.
- `sqlite.rs`: SQLite schema loader and equivalent query runner.
- `assertions.rs`: invariant and output comparison helpers.
- `operations.rs`: randomized operation models for inserts, replaces, deletes, bulk loads, and invalid writes.
- `failpoints.rs`: test-only failpoint controls.
- `crash.rs`: subprocess crash/recovery harness helpers.
- `workloads.rs`: reusable query and operation workloads.

**Core Invariants**
- Every live logical row has exactly one current row record.
- Every live logical row has exactly one entry in every expected current covering index.
- Every covering index decodes to the same logical row as the primary index.
- Every unique constraint has exactly one unique guard per live row.
- No unique guard exists without a matching live row.
- Relation row count stats equal actual current row counts.
- Index entry stats equal actual current index entry counts.
- Dictionary forward and reverse entries agree.
- Re-interning a string or bytes value returns the same ID.
- Deletes remove current row records, current index entries, and unique guards.
- Replaces remove old current index entries and insert new current index entries.
- Failed writes leave rows, indexes, stats, history, counters, and dictionaries unchanged.
- Successful write closures advance the storage transaction ID exactly once.
- Failed write closures do not advance the storage transaction ID.
- History entries correspond only to committed inserts, replaces, and deletes.

**Concrete Work: Test Support Foundation**
- Add `bumbledb-test-support` as a workspace crate.
- Move or duplicate the benchmark schema and deterministic row fixtures into test support.
- Promote the in-memory reference evaluator out of local `query.rs` tests into reusable test support.
- Add reusable assertion helpers for sorted row output, row counts, stats counts, dictionary counts, and invariant scans.
- Add integration-test folders for storage, query, concurrency, recovery, and benchmarks.

**Concrete Work: Invariant Scanner**
- Implement an invariant scanner that can inspect the current database through public/internal-safe APIs.
- Verify row records, current indexes, unique guards, relation stats, index stats, dictionary entries, and history counts.
- Run the invariant scanner after representative insert, replace, delete, bulk load, backup, compact-copy, and reopen sequences.
- Ensure invariant failures produce precise messages naming relation, index, primary key, and expected/actual state.

**Concrete Work: Property Tests**
- Add `proptest` as a dev dependency.
- Generate deterministic fixed-schema data, not arbitrary schemas yet.
- Generate valid FK graphs for holders, accounts, postings, instruments, journal entries, tags, organizations, and edges.
- Generate invalid operations: duplicate primary keys, duplicate unique keys, missing foreign keys, restricted deletes, missing fields, wrong types.
- Generate operation sequences with inserts, replaces, deletes, composite tuple inserts/deletes, and bulk loads.
- Compare LMDB behavior to the reference model after each operation or batch.
- Run invariant scans after generated sequences.

**Concrete Work: Query Differential Tests**
- Generate supported positive Datalog queries over the fixed ledger schema.
- Include single-relation queries, two-way joins, many-way joins, primary inputs, ref inputs, string equality, timestamp ranges, literal comparisons, projections, and aggregations.
- Execute every generated query against LMDB and the reference model.
- Sort outputs and assert exact equality.
- Verify generated query failures are intentional and diagnostic, not panics.

**Concrete Work: SQLite Differential Tests**
- Expand the SQLite fixture beyond the current smoke query.
- Load the full benchmark schema into SQLite with correct indexes.
- Compare Bumbledb and SQLite for representative hand-written benchmark queries.
- Include postings for holder over time range, balances grouped by instrument, postings for account, journal entries touching account sets, tag lookup, and multiway joins over holder/account/posting/instrument/source.
- Record explain plan text alongside each comparison for debugging.
- Do not fail on performance differences yet; fail only on result mismatches and invalid benchmark setup.

**Concrete Work: Parser And Typechecker Golden Tests**
- Add stable golden tests for valid parser/typechecker outputs and diagnostics.
- Cover valid single-relation queries, joins, comparisons, ranges, aggregate queries, unknown relation, unknown field, variable type conflicts, input type conflicts, literal type mismatches, unbound projections, unbound aggregates, and unsupported deferred features.
- Include spans and clear error messages in the golden output.

**Concrete Work: Planner And Explain Golden Tests**
- Add stable explain golden tests for primary lookup, ref-prefix lookup, range scan, two-relation join, many-relation join, aggregation, fallback primary scan, and filtered query.
- Assert variable order, atom execution order, chosen indexes, prefix fields, multiway join flag, cursor seeks, rows scanned, rows matched, bindings yielded, comparisons evaluated/failed, aggregate groups, and output rows.
- Keep plans deterministic unless a future stats-driven planner intentionally changes them.

**Concrete Work: Concurrency Tests**
- Add thread-based integration tests with barriers and channels.
- Test that many readers can query while one writer commits batches.
- Test that readers see stable snapshots while later writes commit.
- Test that readers never observe partial write transactions.
- Test that failed writers do not affect active readers.
- Test that concurrent write attempts serialize safely.
- Test that backup while readers are active creates a usable copy.
- Test that backup while writes happen either succeeds consistently or returns a safe error.

**Concrete Work: Failpoint Atomicity Tests**
- Add test-only failpoints behind `#[cfg(any(test, feature = "test-failpoints"))]`.
- Failpoints should be inert in normal builds.
- Suggested failpoints: `before_dictionary_put`, `after_dictionary_put`, `after_current_row_put`, `after_current_index_put`, `after_unique_guard_put`, `after_stats_update`, `after_history_append`, `before_commit`.
- For each failpoint, test insert, replace, delete, and bulk load.
- Assert operation failure leaves visible state, dictionary, stats, history, and transaction ID unchanged.
- Reopen after failpoint-triggered abort and rerun invariants.

**Concrete Work: Subprocess Crash/Recovery Tests**
- Add an ignored/expensive subprocess test harness.
- Provide a small test helper binary or test mode that can perform a controlled write phase and abort the process.
- Crash cases: before write, after dictionary insert before commit, after current row put before commit, after current index put before commit, after stats/history before commit, immediately after commit, during bulk load before commit, and after backup copy.
- Reopen after every crash scenario.
- Assert pre-commit crashes leave no partial visible state.
- Assert post-commit crashes leave fully visible committed state.
- Assert reopen never panics and invariants pass.

**Concrete Work: Compile-Fail API Lifetime Tests**
- Add `trybuild` as a dev dependency.
- Add compile-fail tests proving scan iterators cannot escape read closures.
- Add compile-fail tests proving read/write transaction wrappers cannot escape their closure lifetimes.
- Add compile-fail tests proving raw LMDB handles and raw cursors are not publicly accessible.
- Add compile-pass tests for normal read, write, query, backup, and bulk-load usage.

**Concrete Work: Fuzz Targets**
- Add `cargo-fuzz` scaffolding outside normal workspace tests.
- Add fuzz targets for primitive decoder robustness, Datalog parser robustness, query typechecker robustness, storage operation sequences, and query differential execution.
- Fuzz targets must never require a long run for normal stage completion.
- Document local and long-running fuzz commands.

**Concrete Work: Stress Tests**
- Add ignored stress tests for larger dictionaries, larger relation indexes, repeated reopen cycles, many small writes, large bulk loads, many readers, backup/compact loops, and random operation sequences.
- Keep stress tests opt-in with `#[ignore]`.
- Normal `cargo test --workspace` must remain fast.

**Concrete Work: CI Documentation**
- Add a testing document with fast, expensive, fuzz, and local stress commands.
- Fast checks: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- Expensive checks: `cargo test --workspace --release`, `cargo test --workspace -- --ignored`.
- Miri checks should target `bumbledb-core` first; LMDB FFI is not expected to be Miri-friendly.
- Fuzz checks should be documented separately from normal CI.

**Out Of Scope**
- Recursive rules.
- Stratified negation.
- As-of query execution.
- Query macros.
- Prepared query caching.
- New runtime query semantics.
- Runtime schema changes or migrations.
- Unsafe durability modes.
- Performance gates that fail CI on speed alone.

**Passing Criteria**
- A `bumbledb-test-support` crate exists and centralizes reusable schemas, row fixtures, reference evaluation, SQLite helpers, assertions, and workloads.
- Invariant scanner verifies current rows, indexes, unique guards, stats, dictionaries, and history for representative databases.
- Property tests cover randomized operation sequences over the fixed benchmark schema.
- Query differential tests compare LMDB to the reference evaluator for generated supported queries.
- SQLite comparison tests cover multiple benchmark queries with good SQLite indexes.
- Parser/typechecker golden tests cover valid queries and diagnostics.
- Explain golden tests cover index choice, variable order, counters, and aggregation.
- Concurrency tests prove snapshot stability and no partial write visibility.
- Failpoint tests prove write atomicity across insert, replace, delete, and bulk load.
- Subprocess crash/recovery tests exist and pass when ignored tests are explicitly enabled.
- Compile-fail tests prove transaction-scoped APIs cannot escape safe lifetimes.
- Fuzz targets exist and are documented.
- Stress tests exist behind `#[ignore]`.
- Fast test command `cargo test --workspace` passes.
- No deferred query feature is implemented as part of this stage.

**Suggested Implementation Order**
- Add test-support crate and shared fixtures.
- Move/centralize reference evaluator and benchmark schema helpers.
- Add invariant scanner and use it in existing storage tests.
- Add property tests for operation sequences.
- Add query differential generator.
- Add golden parser/typechecker and explain tests.
- Add concurrency tests.
- Add failpoint infrastructure and atomicity tests.
- Add subprocess crash harness.
- Add trybuild compile-fail tests.
- Add fuzz scaffolding.
- Add ignored stress tests.
- Add testing/CI documentation.

**Notes**
- Correctness gates are mandatory; performance gates are observational for now.
- The reference evaluator should prioritize clarity over speed.
- Keep expensive tests opt-in so normal development remains fast.
- If this stage exposes architecture bugs, fix the engine instead of weakening the tests.
