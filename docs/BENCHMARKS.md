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

**Tracing Benchmarks**
```sh
RUST_LOG=bumbledb_lmdb=debug cargo run -p bumbledb-bench --release -- --trace --dataset joinstress --scale 2000 --repeats 10
```

The library never initializes a tracing subscriber. The benchmark binary installs one only when `--trace` is passed.

**Current Interpretation**
Bumbledb currently behaves well for highly selective prefix joins. It is slow for broad joins because the planner/executor is still primitive:

- no cost-based statistics,
- no real worst-case-optimal join implementation yet,
- low-selectivity predicates often start with primary scans,
- symbol/equality fields are not always indexed in a useful order,
- broad joins do a lot of row decoding and nested cursor work.

Do not treat the current benchmark results as the final design limit. They are a planner baseline.
