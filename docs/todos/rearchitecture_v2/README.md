# Rearchitecture V2 PRD Suite

This suite replaces the current query execution architecture with a durable-LMDB plus in-memory QueryImage runtime. The design target is a Free Join/LFTJ engine backed by specialized sorted and hash trie structures.

This is not an incremental tuning project. It is the roadmap for a full query-system rebuild.

**Primary Direction**
- Keep LMDB as the durable embedded storage substrate.
- Build immutable snapshot-local query images for fast execution.
- Execute Datalog through a Free Join plan IR.
- Support both sorted trie LFTJ and hash/probe execution under one planner.
- Keep values encoded until output, aggregate arithmetic, or unsupported semantic comparisons require decoding.
- Delete old hot paths once replacements pass correctness and benchmark gates.

**PRD Order**
- `00_architecture_and_rca.md`
- `01_query_image.md`
- `02_columnar_relation_image.md`
- `03_sorted_trie_index.md`
- `04_leapfrog_triejoin_executor.md`
- `05_free_join_plan_ir.md`
- `06_hash_trie_and_hybrid_nodes.md`
- `07_factorized_projection_and_aggregation.md`
- `08_optimizer_and_statistics.md`
- `09_durable_segments_and_snapshots.md`
- `10_benchmark_gates_and_testing.md`
- `11_cutover_and_code_deletion.md`
- `12_query_normalization_and_runtime_specialization.md`
- `13_dependency_graph_and_migration_plan.md`

**Global Non-Negotiables**
- No query hot path may construct LMDB prefix iterators inside variable recursion.
- No query hot path may materialize candidate domains into `BTreeSet<EncodedValue>`.
- No query hot path may decode full logical rows.
- No field-name lookup in inner loops.
- No `Vec<u8>` clone per candidate.
- No dictionary reverse lookup unless output or a semantic decoded comparison requires it.
- No dual executor story at the end of the migration.

**Global Completion Bar**
- Every PRD has tests and benchmark gates.
- `cargo test --workspace` passes at each merge point.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo check --manifest-path fuzz/Cargo.toml` passes.
- Benchmark output clearly separates planning, index-image construction, iterator execution, projection, and aggregation.
- Scale-10000 generated suite moves materially closer to SQLite, especially on triangle, tag lookup, sailors joins, and TPC-H joins.
