# Rosetta Set Semantics Paper Adaptation Audit - Investigator 4

## Sources Read

- `docs/ROSETTA_STONE.md`, especially product thesis, core commitments, relation semantics, query semantics, query execution, public output, benchmark, golden-example, and validation contracts.
- `docs/free-join-paper/arXiv-2301.10841v2/main.tex` and `macros.tex` for paper structure and included sections.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/00-abstract.tex` through `tex/06-discussion.tex`, including the non-included scratch section `tex/025-tale.tex` for additional SQL, DuckDB-plan, and asymptotic examples.
- `crates/bumbledb-core/src/query_ir.rs`, `crates/bumbledb-core/src/query_builder.rs`, `crates/bumbledb-core/src/schema/descriptors.rs`, `crates/bumbledb-core/src/schema/validation.rs`, and `crates/bumbledb-core/src/schema_tests.rs`.
- `crates/bumbledb-lmdb/src/free_join.rs`, `crates/bumbledb-lmdb/src/query/model.rs`, `crates/bumbledb-lmdb/src/query/api.rs`, `crates/bumbledb-lmdb/src/query/normalize.rs`, `crates/bumbledb-lmdb/src/query/planner.rs`, `crates/bumbledb-lmdb/src/query/lftj_runtime.rs`, `crates/bumbledb-lmdb/src/query/lftj_access.rs`, `crates/bumbledb-lmdb/src/query/sinks.rs`, and `crates/bumbledb-lmdb/src/query/metrics.rs`.
- `crates/bumbledb-lmdb/src/query_image.rs` and query-image submodules, especially snapshot-local image building and scope selection.
- `crates/bumbledb-lmdb/src/storage.rs`, `crates/bumbledb-lmdb/src/storage/write.rs`, `crates/bumbledb-lmdb/src/storage/types.rs`, and storage/query tests under `crates/bumbledb-lmdb/src/*tests*`.
- `crates/bumbledb-test-support/src/reference.rs`, `crates/bumbledb-test-support/src/assertions.rs`, `crates/bumbledb-test-support/src/schemas.rs`, and tests under `crates/bumbledb-test-support/tests/`.
- `crates/bumbledb-bench/src/main.rs`, `crates/bumbledb-bench/src/main/run.rs`, `crates/bumbledb-bench/src/main/sqlite.rs`, `crates/bumbledb-bench/src/main/result.rs`, `crates/bumbledb-bench/src/main/render_markdown.rs`, `crates/bumbledb-bench/src/main/tracing.rs`, benchmark dataset definitions, open-data importers, and benchmark tests.

## Executive Summary

The Free Join paper can be used as algorithmic inspiration, but its formal and experimental assumptions must be filtered through Rosetta before they enter Bumbledb. The paper explicitly starts from bag semantics, SQL queries, pushed-down selections as derived base tables, post-full-join projection and aggregation, DuckDB-produced binary plans, main-memory column vectors, and JOB/LSQB benchmark conventions. Bumbledb must reject or adapt each of those assumptions because it is a strict Codd-style set engine over LMDB with no SQL frontend, no nulls, no floating-point persistence, no generated IDs, no DB-side aggregation contract, and duplicate-free result sets.

The safe adaptation is narrow: Bumbledb should keep the positive join algorithm ideas, but execute typed Rust query IR over set-valued full facts; treat selections as typed atom terms and comparison predicates inside the engine; treat projection as a set-producing sink; use query images only as snapshot-local derived structures over LMDB; and use SQLite only as a `SELECT DISTINCT` exact-value oracle in benchmarks.

The current result path mostly satisfies the set-output contract. `QueryResultSet::new` sorts and deduplicates facts at `crates/bumbledb-lmdb/src/query/model.rs:166-172`, the encoded projection sink inserts projected facts into a `BTreeSet` at `crates/bumbledb-lmdb/src/query/sinks.rs:82-119`, and the benchmark runner compares exact projected values against SQLite before timing interpretation at `crates/bumbledb-bench/src/main/run.rs:101-112`.

The current risk surface is still material. Public comments still mention database-allocated serials and aggregation, one legacy LMDB benchmark test compares only counts, open benchmark importers encode missing source values as zero or empty-string sentinels, open import parsing uses floating-point conversions before storing integral/decimal values, and validation does not yet make repeated atom fields or same-atom repeated variables an explicit set-engine decision.

## Non-Negotiable Rosetta Constraints

- Bumbledb is an embedded, typed, schemaful, set-semantic relational database for highly normalized application data, not SQL, not a server, not OLAP, and not a document store: `docs/ROSETTA_STONE.md:5-11`.
- The storage backend is LMDB only; SQL frontends, nulls, floating-point persistence, async API, server mode, network protocol, runtime DDL, and multiple writers are forbidden: `docs/ROSETTA_STONE.md:13-26`.
- Exact duplicate insert is an idempotent no-op, delete is exact fact deletion, delete of an absent fact is an idempotent no-op, there is no update operation, and there is no DB-side generated ID allocator: `docs/ROSETTA_STONE.md:36-43`.
- Projection output has set semantics, SQL-style multiset behavior is out of scope, and there is no `SELECT DISTINCT` concept in Bumbledb because distinctness is the default: `docs/ROSETTA_STONE.md:44-46`.
- Schemas are Rust descriptors compiled into the binary, with no primary-key descriptor and no generated-ID descriptor; canonical fact membership is implicit for every relation: `docs/ROSETTA_STONE.md:48-66`.
- Serial values are ordinary nominal values supplied by users or ETL; an `AccountId` is not interchangeable with an `InstrumentId`: `docs/ROSETTA_STONE.md:87-99`.
- Persistent value types are restricted to bool, integral, timestamp-micros, decimal, enum, string, bytes, and nominal serial; optional data is represented by absent facts in separate relations, not nulls: `docs/ROSETTA_STONE.md:100-115`.
- Query solutions are sets of variable bindings, projection returns a set of projected facts, and existential variables must not multiply projected output: `docs/ROSETTA_STONE.md:146-150`.
- Query execution retains Free Join/LFTJ plus fact-native projection/storage paths; legacy non-result-set APIs and scalar caches were removed because they encoded witness multiplicity: `docs/ROSETTA_STONE.md:152-158`.
- Public query output is `QueryOutput { result: QueryResultSet, ... }`, and `QueryResultSet` is duplicate-free and canonicalized: `docs/ROSETTA_STONE.md:160-171`.
- Benchmarks must validate exact Bumbledb result values against SQLite before timings matter, and SQLite projection references must use `SELECT DISTINCT`: `docs/ROSETTA_STONE.md:173-178`.

## Paper Assumptions To Reject Or Adapt

### Bag Semantics And Multiplicity

The paper explicitly says relations may contain duplicates and uses bag semantics at `docs/free-join-paper/arXiv-2301.10841v2/tex/02-background.tex:8-10`. It further describes duplicate tuples or stored multiplicity in binary hash tables at `tex/02-background.tex:146-152`, multiplicity multiplication in Generic Join output at `tex/02-background.tex:181-183`, multiplicity stored in trie leaves at `tex/02-background.tex:197-203`, and leaf multiplicity multiplication at `tex/02-background.tex:231-233`.

Bumbledb must reject all multiplicity behavior. Hash/trie/COLT leaves represent fact membership, not duplicate counts. Output is a set of projected facts. Duplicate witnesses are allowed during internal search only as redundant derivations that must collapse before public output.

The current adaptation is mostly correct: `EncodedProjectSink` uses a `BTreeSet` before decoding at `crates/bumbledb-lmdb/src/query/sinks.rs:82-119`, and `QueryResultSet::new` sorts and deduplicates at `crates/bumbledb-lmdb/src/query/model.rs:166-172`. Tests cover duplicate-witness projection in `crates/bumbledb-test-support/tests/golden_examples.rs:135-162` and result-set deduplication in `crates/bumbledb-lmdb/src/query_tests/basic.rs:34-47`.

### SQL Query Surface

The paper presents SQL examples for the triangle query at `tex/02-background.tex:31-44`, the sand-dollar query at `tex/025-tale.tex:10-22`, and a JOB fragment at `tex/025-tale.tex:81-99`. It treats SQL as the user-level query language in its examples.

Bumbledb must reject SQL as a product surface. SQL may appear only in benchmark reference code, and only as an external oracle that uses `SELECT DISTINCT`. The actual query surface is the typed Rust IR and builder API in `crates/bumbledb-core/src/query_ir.rs:33-152` and `crates/bumbledb-core/src/query_builder.rs:55-164`.

SQL aliases in the paper must adapt to typed relation occurrences. Self-joins cannot be reasoned about by relation name alone; the implementation already has normalized atom IDs at `crates/bumbledb-lmdb/src/query/model.rs:82-93`, but any formal Free Join plan must make occurrence identity explicit.

### Selections Pushed To Base Tables

The paper assumes selections are pushed down to base tables, so an atom may denote a selected/projection-derived base table, and all variables in each atom are distinct: `tex/02-background.tex:23-26`. Its SQL example rewrites `S = Pi_uv(sigma_w>30(M))` and `T = Pi_uv(sigma_v=w(M))` before the triangle query at `tex/02-background.tex:46-48`.

Bumbledb must adapt, not copy, this assumption. There is no external SQL optimizer that creates temporary base tables. Literal fields, input fields, omitted fields, wildcard fields, and comparison predicates are part of the typed query IR and must be validated and optimized inside Bumbledb.

Current code stores comparisons separately as `TypedComparison` and `NormPredicate` at `crates/bumbledb-core/src/query_ir.rs:121-143` and `crates/bumbledb-lmdb/src/query/model.rs:121-145`, then evaluates ready predicates during LFTJ execution at `crates/bumbledb-lmdb/src/query/lftj_runtime.rs:135-160` and `crates/bumbledb-lmdb/src/query/lftj_runtime.rs:187-197`. That is acceptable for set semantics, but it is not the paper's pushed-base-table model.

Repeated variables inside one atom need an explicit decision. The paper excludes them after pushdown, but Bumbledb can either reject them at the typed IR boundary or lower them into same-fact equality predicates. Current access planning detects repeated variables only as an access-path limitation at `crates/bumbledb-lmdb/src/query/lftj_access.rs:56-58` and `crates/bumbledb-lmdb/src/query/lftj_access.rs:268-278`, which risks an internal planning failure rather than a clear query error or selection lowering.

### Projection And Aggregation After Full Joins

The paper says projections and aggregates are performed after the full join and are omitted from the formal CQ at `tex/02-background.tex:27-28`. The evaluation section says benchmark queries contain base-table filters, natural joins, and a simple group-by at the end at `tex/05-eval.tex:139-144`, excludes selection and aggregation time from reported performance at `tex/05-eval.tex:161-164`, and discusses output before aggregation at `tex/05-eval.tex:254-259`.

Bumbledb must reject aggregation under the current Rosetta contract. Projection is not SQL `SELECT` with bag semantics and is not `SELECT DISTINCT`; it is the native set-valued output operation. There is no public count, group-by, sum, or factorized aggregate output contract.

Bumbledb may adapt the paper's post-join projection boundary into a result-set sink that consumes completed bindings and inserts projected facts into a set. It does not need to materialize a full join before projection, as long as existential variables do not multiply public output. The current sink behavior at `crates/bumbledb-lmdb/src/query/sinks.rs:96-160` is aligned with that adaptation.

Stale aggregation wording remains in current source comments and report rendering. Examples are `crates/bumbledb-lmdb/src/free_join.rs:8-10`, `crates/bumbledb-lmdb/src/free_join.rs:40-45`, `crates/bumbledb-lmdb/src/query/metrics.rs:177-194`, and `crates/bumbledb-bench/src/main/render_markdown.rs:203-206`.

### DuckDB Plans And Cost Optimizer Assumptions

The paper starts from optimized binary plans produced by DuckDB at `tex/01-intro.tex:138-146`, compares against DuckDB at `tex/01-intro.tex:189-196`, uses DuckDB's optimizer in plan conversion at `tex/04-optimizations.tex:31-40`, and repeatedly invokes DuckDB in evaluation at `tex/05-eval.tex:9-13`, `tex/05-eval.tex:115-117`, `tex/05-eval.tex:157-160`, and `tex/05-eval.tex:169-170`.

Bumbledb must reject DuckDB as a planning dependency. It has no SQL frontend and no plan import contract. The paper's `binary2fj`, factorization, and robustness experiments can inspire an internal optimizer, but the inputs must be typed Bumbledb IR, compiled Rust schema metadata, LMDB/query-image statistics, and declared access paths.

Current benchmark code correctly uses SQLite for reference execution, not DuckDB. The benchmark runner creates Bumbledb typed queries via `BenchQuery.build` and SQLite strings via `BenchQuery.sqlite` at `crates/bumbledb-bench/src/main/tracing.rs:130-136`.

### Main-Memory And COLT Raw Data Assumptions

The paper's COLT section assumes raw data is stored column-wise in main memory at `tex/04-optimizations.tex:163-178`. The limitations section states the current system is main-memory only and warns that disk-resident data may make COLT inefficient due to repeated random access at `tex/06-discussion.tex:6-9`. The experiments configure all systems to run single-threaded in main memory at `tex/05-eval.tex:154-156`.

Bumbledb must reject main-memory data as the storage model. LMDB is the only source of truth. Query images may be in-memory, but they are derived, snapshot-local, scoped execution structures as required by Rosetta.

The current query-image implementation follows the right direction: `QueryImageBuilder` records the LMDB read snapshot transaction ID at `crates/bumbledb-lmdb/src/query_image/builder.rs:28-61`, builds relation images from current access state at `crates/bumbledb-lmdb/src/query_image/builder.rs:103-248`, and keys images by schema fingerprint, storage transaction ID, and scope at `crates/bumbledb-lmdb/src/query_image/scope.rs:6-15`.

Any future COLT-like structure must be an execution image over LMDB facts or access entries. It must not become a second storage backend, a durable column store, or a public API contract.

### Benchmark Setup And Dataset Semantics

The paper's benchmark assumptions include JOB and LSQB, base-table filters, natural joins, simple group-by, no nulls, excluding five empty JOB queries, excluding LSQB anti/outer joins, single-threaded main-memory execution, and omitting selection/aggregation time: `tex/05-eval.tex:139-164`.

Bumbledb must adapt benchmarks to Rosetta. Correctness is exact projected value equality against SQLite with `SELECT DISTINCT`, not counts first and not SQL bag output. Empty query results are valid set outputs and should be tested, not categorically excluded for paper reproducibility reasons. Anti-joins, outer joins, group-by, and SQL null semantics remain out of scope unless explicitly modeled as positive set relations.

The current benchmark suite mostly obeys the Rosetta benchmark contract. Built-in and open benchmark SQL strings use `SELECT DISTINCT`, for example `crates/bumbledb-bench/src/main/datasets.rs:60-64`, `crates/bumbledb-bench/src/open/job_query_list.rs:9-19`, `crates/bumbledb-bench/src/open/imdb.rs:126-130`, and `crates/bumbledb-bench/src/open/ldbc.rs:341-353`. Exact Bumbledb-vs-SQLite projected values are compared before timing interpretation at `crates/bumbledb-bench/src/main/run.rs:101-112`.

The remaining benchmark setup risks are open-data ETL and measurement labeling, not the main exact-value comparison path.

## Current Violations/Risks

### P0: Public Serial Documentation Contradicts Rosetta

`ValueType::Serial` is documented as a "Nominal database-allocated serial value" at `crates/bumbledb-core/src/schema/descriptors.rs:227-231`. Rosetta explicitly says serial values are preserved ordinary values and are not generated by the database at `docs/ROSETTA_STONE.md:87-99`.

This is a public API documentation violation. Even if the implementation does not allocate IDs, the comment teaches the wrong model.

### P0: Legacy Benchmark Comparison Is Count-Only

`BenchmarkComparison` stores only Bumbledb and SQLite fact counts at `crates/bumbledb-lmdb/src/benchmark.rs:29-40`, and `benchmark_schema_loads_and_sqlite_comparison_runs` asserts only count equality at `crates/bumbledb-lmdb/src/benchmark/tests.rs:36-45`. This violates the benchmark contract if used as a benchmark correctness pattern.

The newer `bumbledb-bench` runner is better because it compares exact values first at `crates/bumbledb-bench/src/main/run.rs:101-112`. The older LMDB benchmark fixture should not remain as a count-only exemplar.

### P0: Open Benchmark Importers Encode Missing Values As Sentinels

`parse_optional_i64` and `parse_optional_u64` convert empty strings and `\N` to `0` at `crates/bumbledb-bench/src/open.rs:109-123`. `job_text` converts empty strings and `\N` to `String::new()` at `crates/bumbledb-bench/src/open/csv_readers.rs:42-48`. These helpers are used across IMDB, JOB, and Lahman importers, including `crates/bumbledb-bench/src/open/imdb.rs:28-31`, `crates/bumbledb-bench/src/open/job_stream.rs:218-233`, and `crates/bumbledb-bench/src/open/lahman.rs:88-99`.

That is a Rosetta modeling risk. Optional source data should become absent facts in separate relations, or the dataset adapter must explicitly prove that the sentinel is a real closed-domain value. Otherwise, imported benchmark facts contain encoded null semantics inside non-null fields.

### P1: Floating-Point Parsing Enters Benchmark ETL

`parse_rating_x10` and `parse_decimal_i128` parse strings through `f64` and then round at `crates/bumbledb-bench/src/open.rs:129-139`. The persisted values are integral or decimal, not float values, so this is not floating-point persistence. It is still a benchmark-alignment risk because decimal/rating ETL can be rounded differently from SQLite or the source specification.

TPCH open import uses `parse_decimal_i128` for `extended_price` at `crates/bumbledb-bench/src/open/tpch.rs:83-86`, and IMDB import uses `parse_rating_x10` for ratings at `crates/bumbledb-bench/src/open/imdb.rs:66-71`. Exact decimal string parsing should replace floating conversion.

### P1: Aggregation Terminology Leaks Through Public/Diagnostic Surfaces

`OutputPlan` is commented as "projection/aggregation" at `crates/bumbledb-lmdb/src/free_join.rs:8-10` and `crates/bumbledb-lmdb/src/free_join.rs:40-45`. `PlanCounters` comments mention projection/aggregation at `crates/bumbledb-lmdb/src/query/metrics.rs:177-194`. Benchmark Markdown explains sink finish as possibly "projection, aggregation, sorting, or decode" at `crates/bumbledb-bench/src/main/render_markdown.rs:203-206`. Test-support docs refer to an "aggregation overflow" schema at `crates/bumbledb-test-support/src/schemas.rs:87-103`.

These are not harmless names. They suggest a scalar or grouped output path that Rosetta intentionally removed.

### P1: Timed SQLite Samples Count Rows Instead Of Materializing Values

The benchmark runner validates exact SQLite values once via `sqlite_result_facts` at `crates/bumbledb-bench/src/main/run.rs:101-112`, then measures SQLite cold/warm/sample timings through `sqlite_count` at `crates/bumbledb-bench/src/main/run.rs:113-125` and `crates/bumbledb-bench/src/main/run.rs:133-149`. `sqlite_count` iterates rows without decoding projected values at `crates/bumbledb-bench/src/main/sqlite.rs:3-13`.

This does not violate the minimum Rosetta requirement because exact values are checked before timings matter. It is still a measurement-label risk because `sqlite_materialized_facts` is hard-coded true at `crates/bumbledb-bench/src/main/result.rs:49`, while timed SQLite samples are not materialized value sets.

### P1: Query Validation Does Not Explicitly Decide Duplicate Atom Fields

The typed query builder lets relation atom fields accumulate without duplicate-field checks at `crates/bumbledb-core/src/query_builder.rs:348-363`. Execution-boundary validation checks IDs, names, and types at `crates/bumbledb-lmdb/src/query/normalize.rs:15-99`, but does not reject duplicate field bindings within one atom.

Under a strict set fact model, each relation field in an atom should be bound at most once unless duplicate binding is explicitly lowered into an equality predicate. Leaving this implicit risks paper-style pushed-selection assumptions leaking into malformed IR behavior.

### P1: Same-Atom Repeated Variables Are Not A Product-Level Semantic

The paper assumes atom variables are distinct after pushed-down selections at `tex/02-background.tex:23-26`. Bumbledb can support same-atom equality, but it must say so and implement it consistently. Current LFTJ access planning treats repeated variables as a reason not to use lazy durable access at `crates/bumbledb-lmdb/src/query/lftj_access.rs:56-58`, then may fail if no fallback source exists.

The correct Rosetta adaptation is either a clean invalid-query error or a normalized comparison predicate over two fields of the same fact.

### P1: Self-Joins Rely On Repeated Relation Names Without Public Alias Semantics

The paper says self-joins are handled by renaming relation occurrences at `tex/02-background.tex:20-22`. Bumbledb benchmark builders use repeated `rel("Title")`, `rel("CompanyName")`, and `rel("InfoType")` calls for self-join-like JOB queries at `crates/bumbledb-bench/src/open/job_query_builders.rs:271-320` and `crates/bumbledb-bench/src/open/job_query_builders.rs:322-380`.

This can be semantically fine because normalized atoms have occurrence IDs, but the public typed IR does not expose alias identity. Any formal Free Join or explain output must refer to atom occurrence IDs, not only base relation names.

### P2: Benchmark Query Names Still Suggest Aggregation

`joinstress` has a benchmark named `triangle_count` at `crates/bumbledb-bench/src/main/datasets.rs:325-330`, but its SQLite query projects `eab.a` with `SELECT DISTINCT`, not a count. This is a naming risk because Rosetta forbids resurrecting scalar count APIs as the query output contract.

### P2: SQLite Reference Schemas Use SQL Primary Keys

Benchmark SQLite schemas use `PRIMARY KEY` to mimic relation identity and unique constraints, for example `crates/bumbledb-bench/src/main/datasets.rs:34-46`, `crates/bumbledb-bench/src/main/datasets.rs:155-164`, and `crates/bumbledb-bench/src/open/imdb.rs:107-115`.

This is acceptable only as SQLite reference scaffolding. Documentation should prevent readers from inferring that Bumbledb has primary-key descriptors or generated IDs.

### P2: Tests Mostly Cover Set Output, But Not Paper Rejection Boundaries

Golden and differential tests cover duplicate witnesses, exact projections, duplicate inserts, absent deletes, and exact SQLite comparison, such as `crates/bumbledb-test-support/tests/golden_examples.rs:39-172`, `crates/bumbledb-test-support/tests/sqlite_comparison.rs:45-65`, and `crates/bumbledb-test-support/tests/property_and_differential.rs:35-90`.

Missing tests are concentrated around rejection/adaptation boundaries: bag multiplicity must never surface, same-atom repeated variables must be rejected or lowered, duplicate field bindings must be rejected, open-data nulls must not become silent sentinel facts, and old benchmark helpers must not validate by count only.

## Required Breaking Changes

- Change public serial documentation and any API prose from "database-allocated" to externally supplied nominal serial values.
- Remove aggregation from `OutputPlan`, `PlanCounters`, benchmark reports, test-support comments, and any public docs until a Rosetta-approved aggregation product contract exists.
- Replace `BenchmarkComparison` count-only correctness with exact projected value sets, or delete the legacy LMDB benchmark comparison API in favor of the `bumbledb-bench` result-set contract.
- Make `sqlite_materialized_facts` and timed SQLite measurement labels honest: either time exact value materialization or report that timed SQLite samples are row-iteration counts after a separate exact-value correctness pass.
- Refactor open benchmark ETL so missing source attributes become absent facts in separate optional relations, or reject source rows with missing data when a benchmark model cannot represent optionality cleanly.
- Replace floating-point decimal/rating parsing in benchmark importers with exact string-to-integer or string-to-decimal parsing.
- Add execution-boundary validation for duplicate field bindings in a relation atom.
- Decide whether repeated variables inside one atom are legal equality constraints or invalid IR, then implement that decision at normalization instead of leaving it to access-path failure.
- Make relation occurrence identity explicit in any public explain/debug output that describes self-joins or formal Free Join plan structure.
- Keep SQLite SQL strictly as benchmark oracle text; do not introduce SQL parsing, DuckDB plan ingestion, or SQL-shaped API compatibility aliases.

## Documentation Updates

- Add a Free Join adaptation note that explicitly says the paper's bag semantics are rejected and all multiplicity examples become set-membership examples in Bumbledb.
- Document that paper SQL is illustrative only; Bumbledb's query surface is typed Rust IR and builder API.
- Document selection adaptation: relation atom terms and comparison predicates are Bumbledb's internal selection model; no external pushed-down base tables or SQL views are part of execution.
- Document projection adaptation: projection is a duplicate-free result-set sink, not `SELECT DISTINCT`, not count, and not group-by aggregation.
- Document that DuckDB appears only in the paper; Bumbledb benchmarks use SQLite as an exact `SELECT DISTINCT` oracle and Bumbledb planning uses internal schema/access statistics.
- Document query images as snapshot-local LMDB-derived execution structures, not main-memory storage and not COLT persistence.
- Document benchmark ETL rules for open datasets: no nulls, no float persistence, no silent optional sentinels unless the sentinel is a declared domain value.
- Document SQLite reference schemas as oracle scaffolding only; their `PRIMARY KEY` clauses do not imply Bumbledb primary-key descriptors.
- Rename or explain benchmark queries whose names imply aggregation, especially `triangle_count`.

## New Tests

- Add a benchmark test that fails when Bumbledb and SQLite have equal row counts but different projected values in an end-to-end `run_dataset` path, not only helper-level sorted vectors.
- Add a test that every benchmark `sqlite` string uses `SELECT DISTINCT` and contains no `COUNT(`, `GROUP BY`, `LEFT JOIN`, `OUTER JOIN`, or unmodeled null-sensitive predicate.
- Add a test that the legacy `crates/bumbledb-lmdb/src/benchmark` comparison path validates exact projected values or is removed from correctness claims.
- Add query validation tests for duplicate field bindings in one relation atom.
- Add query validation tests for same-atom repeated variables, matching the chosen policy: explicit invalid-query error or equality-predicate lowering with exact expected results.
- Add self-join tests that assert repeated relation occurrences are independent atom occurrences and produce duplicate-free projected sets.
- Add open-data ETL tests where `\N` or empty optional fields produce absent optional facts or rejected rows, not zero or empty-string sentinel values in required fields.
- Add exact decimal/rating parser tests that cover values like `7.05`, `7.15`, and large TPCH decimal strings without `f64` rounding.
- Add regression tests proving existential variables do not multiply projection output across at least two independent duplicate-witness shapes.
- Add a public API/docs lint test, if feasible, to prevent reintroducing "database-allocated serial", "aggregation", or "multiset" wording in public-facing Rust docs.

## Open Questions

- Are open benchmark adapters allowed to use explicit sentinel values for missing source attributes if the sentinel is documented as part of that benchmark model, or must every optional source attribute become a separate relation?
- Should same-atom repeated variables be a supported equality-selection shorthand, or should users express that as explicit comparison predicates only?
- Should timed SQLite samples materialize exact projected values every time for measurement symmetry, despite the extra overhead, or is row iteration acceptable after one exact-value correctness pass?
- Will Bumbledb v4 intentionally exclude aggregation entirely, or is a future set-semantic aggregate extension expected to get its own Rosetta contract?
- Should public typed IR fields remain directly mutable, or should malformed-IR rejection be supplemented by sealed constructors to reduce invalid query shapes?
- Should empty-result benchmark queries be permanent fixtures, contrary to the paper's JOB exclusion, to prove empty set semantics and reproducibility under Bumbledb?
