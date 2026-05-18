# 10: Benchmark Gates And Testing

**Goal**
- Create correctness, performance, and counter gates for the rearchitecture so every stage proves progress.

**Benchmark Suites**
- Generated scale 2000 quick gate.
- Generated scale 10000 extreme gate.
- Focused triangle stress gate.
- Focused tag/static-predicate lookup gate.
- Focused TPC-H range/equality gate.
- Open IMDb/JOB-like future gate.

**Required Commands**
```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check --manifest-path fuzz/Cargo.toml
cargo run -p bumbledb-bench --release -- --scale 2000 --repeats 10
cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 30
```

**Performance Gate Structure**
```rust
pub struct BenchmarkGate {
    pub dataset: &'static str,
    pub query: &'static str,
    pub max_bumbledb_avg_micros: u64,
    pub max_sqlite_ratio: f64,
    pub max_iterator_ops: Option<u64>,
    pub max_materialized_values: Option<u64>,
}
```

**Initial Gates After Full Rearchitecture**
- `triangle_count`: under `25ms` at scale 10000.
- `tag_lookup_join`: under `5ms` at scale 10000.
- `red_boat_sailors`: under `5ms` at scale 10000.
- `supplier_nation_orders`: under `5ms` at scale 10000.
- Small selective queries: under `10µs` at scale 10000.

**Counter Gates**
- No LMDB prefix scan openings inside query variable recursion.
- No candidate `BTreeSet` allocations.
- `dictionary_reverse_lookups == 0` unless projected output includes string/bytes.
- `materialized_output_values` equals final output values for projection queries.
- Aggregate count queries do not materialize complete bindings when factorization applies.

**Testing Layers**
- Unit tests for iterator primitives.
- Unit tests for trie and hash indexes.
- Plan IR validation tests.
- Executor differential tests against reference evaluator.
- SQLite row-count and row-set comparison tests.
- Property tests for generated valid schemas/rows.
- Crash tests for durable segments.
- Fuzz tests for parser, encoding, and eventually plan executor.

**Benchmark Output Requirements**
- QueryImage cache/build stats.
- Chosen Free Join plan.
- Node-level estimated and actual rows.
- Iterator operation counts.
- Hash build/probe counts.
- Materialized values.
- Dictionary reverse lookups.
- SQLite comparison timing.

**Passing Criteria**
- Every PRD implementation updates or preserves benchmark gates.
- CI-ready scripts exist for quick and extreme benchmark runs.
- Benchmark result parser emits markdown tables.
- Regressions are explained in docs before moving to the next stage.

**Non-Goals**
- Do not make full extreme benchmarks mandatory for every local edit.
- Do not benchmark with tracing as the primary timing source.
