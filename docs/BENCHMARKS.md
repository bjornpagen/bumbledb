# Benchmarks

The benchmark harness lives in `crates/bumbledb-bench`.

It has two tiers:

- Generated local datasets for quick sanity and scaling checks.
- Importers for true open datasets, supplied by local dataset directories.

The harness compares Bumbledb against SQLite with explicit indexes and prints Bumbledb explain counters for every query.

**Fast Local Run**
```sh
cargo run -p bumbledb-bench --release -- --scale 500 --repeats 5
```

**CI Gates**
```sh
scripts/bench-quick.sh
scripts/bench-extreme.sh
scripts/bench-focused.sh
scripts/check-cutover.sh
```

`bench-quick.sh` runs tests, clippy, fuzz check, and the generated scale-2000 benchmark gate.
`bench-extreme.sh` runs the generated scale-10000 benchmark gate.
`bench-focused.sh` runs the focused ledger/sailors/joinstress/tpch gates at scale 10000 by default.
`check-cutover.sh` runs the PRD 11 code-deletion search gates for removed query hot paths.

Set `BUMBLED_BENCH_SCALE` or `BUMBLED_BENCH_REPEATS` for `bench-focused.sh` when a smaller local smoke is needed.

**Markdown Output**
```sh
cargo run -p bumbledb-bench --release -- --scale 2000 --repeats 10 --format markdown
```

Markdown output includes result and counter-gate tables with QueryImage segment/build stats, chosen Free Join plan, iterator estimates, hash build/probe estimates, materialized values, dictionary reverse lookups, and gate status.

**Run One Generated Dataset**
```sh
cargo run -p bumbledb-bench --release -- --dataset ledger --scale 2000 --repeats 10
cargo run -p bumbledb-bench --release -- --dataset sailors --scale 2000 --repeats 10
cargo run -p bumbledb-bench --release -- --dataset joinstress --scale 2000 --repeats 10
cargo run -p bumbledb-bench --release -- --dataset tpch --scale 2000 --repeats 10
```

**Generated Datasets**
- `ledger`: normalized accounting-style workload matching the product thesis.
- `sailors`: scaled Sailors/Boats/Reserves relational teaching dataset.
- `joinstress`: chain joins plus cyclic triangle queries.
- `tpch`: TPC-H-like synthetic subset with customer/orders/lineitem/supplier/part.

**Open Dataset: IMDb Public TSV**
Source:
- https://datasets.imdbws.com/

Expected decompressed files:
- `title.basics.tsv`
- `name.basics.tsv`
- `title.ratings.tsv`
- `title.principals.tsv`

Run:
```sh
cargo run -p bumbledb-bench --release -- --imdb-dir /path/to/imdb --dataset imdb --scale 100000 --repeats 10
```

Notes:
- `--scale` limits imported titles and names, then imports matching ratings/principals.
- IDs are remapped from IMDb string IDs to internal numeric refs.
- This is the most practical real open dataset for join-heavy public benchmarking.

**Open Dataset: TPC-H `.tbl` Files**
Source:
- TPC-H dbgen or compatible generated `.tbl` files.

Expected files:
- `customer.tbl`
- `supplier.tbl`
- `part.tbl`
- `orders.tbl`
- `lineitem.tbl`

Run:
```sh
cargo run -p bumbledb-bench --release -- --tpch-dir /path/to/tpch --dataset tpch-open --scale 100000 --repeats 10
```

Notes:
- This is a standard analytical benchmark shape, not our primary product target.
- The importer samples rows and drops rows with references outside the sample window.

**Open Dataset: Lahman Baseball CSV**
Source:
- https://github.com/chadwickbureau/baseballdatabank

Expected files:
- `People.csv`
- `Teams.csv`
- `Batting.csv`
- `Salaries.csv`

Run:
```sh
cargo run -p bumbledb-bench --release -- --lahman-dir /path/to/lahman/core --dataset lahman --scale 100000 --repeats 10
```

Notes:
- This gives a real normalized CSV workload with player/team/salary/batting joins.
- It is useful as a mid-size open relational dataset, though less join-stressful than IMDb/JOB.

**Open Dataset: LDBC SNB CSV**
Source:
- https://ldbcouncil.org/benchmarks/snb/

Expected CSV files, names may include partition suffixes:
- `person*.csv`
- `post*.csv`
- `person_knows_person*.csv`
- `person_likes_post*.csv`

Run:
```sh
cargo run -p bumbledb-bench --release -- --ldbc-dir /path/to/ldbc/social_network --dataset ldbc --scale 100000 --repeats 10
```

Notes:
- This is the right open dataset family for future recursive/path-query benchmarking.
- v0 uses only non-recursive joins over person/post/knows/likes subsets.

**Open Dataset Still Needed: JOB/IMDb**
The best external benchmark for our actual join-planner thesis is the Join Order Benchmark over IMDb.

Current status:
- The harness imports public IMDb TSVs, not the exact JOB relational dump/query set.
- A future benchmark stage should add the full JOB schema and query translations.

Why:
- JOB has many difficult many-way joins over normalized IMDb data.
- It is more relevant than TPC-H for proving join-planner quality.

**Reading Results**
Each query prints:
- `rows`: result row count.
- `bumbledb_total` and `bumbledb_avg`.
- `sqlite_total` and `sqlite_avg`.
- chosen Bumbledb relation/index plan lines.
- `cursor_seeks`.
- `rows_scanned`.
- `output_rows`.

The markdown table additionally prints:

- QueryImage build time and durable segment usage.
- chosen Free Join candidate.
- estimated iterator operations.
- estimated hash build/probe rows.
- counter-gate notes.

**Current Gate Interpretation**

The active gates are intentionally CI-safe current-stage gates. They enforce structural regressions that should not return during the rearchitecture:

- no LMDB cursor/prefix scans inside query variable recursion,
- no dictionary reverse lookups unless string/bytes are present in final output,
- projection materialization equals final output values,
- count aggregation does not materialize more values than final output.

The long-term post-rearchitecture target gates remain:

- `joinstress/triangle_count`: under 25ms at scale 10000,
- `ledger/tag_lookup_join`: under 5ms at scale 10000,
- `sailors/red_boat_sailors`: under 5ms at scale 10000,
- `tpch/supplier_nation_orders`: under 5ms at scale 10000,
- small selective queries: under 10us at scale 10000.

Known current-stage regressions are expected while PRDs 10 and earlier still route all physical implementations through the LFTJ executor substrate. The benchmark gates document those regressions in markdown output instead of making the full extreme target mandatory for every local edit.

**Tracing Benchmarks**
```sh
RUST_LOG=bumbledb_lmdb=debug cargo run -p bumbledb-bench --release -- --trace --dataset joinstress --scale 2000 --repeats 10
```

The library never initializes a tracing subscriber. The benchmark binary installs one only when `--trace` is passed.

**Current Interpretation**
Bumbledb currently behaves well for highly selective prefix joins. Broad joins are still slower than the final target gates because the v2 executor is cut over to QueryImage/Free Join/sorted-trie execution, while dedicated hash/hybrid runtime kernels remain future optimization work:

- selected hash/hybrid node implementations are explained but not yet separate runtime kernels,
- broad joins still perform many sorted-trie operations,
- full post-rearchitecture latency gates are tracked but not mandatory for local edits.

Do not treat the current benchmark results as the final design limit. They are a planner baseline.
