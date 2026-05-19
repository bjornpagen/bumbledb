# Observability, Lints, And Allocation Hardening

This suite is an interstitial hardening pass after `performance_kill_list/04_real_hash_probe_runtime.md` and before `performance_kill_list/05_direct_selective_query_kernels.md`.

The purpose is to make future performance work safer and more measurable. We just changed the execution architecture heavily. Before adding direct selective kernels, the codebase needs strict compiler/linter gates, panic cleanup, phase timing, trace/profiling UX, allocation recording, and the first stack/GAT cleanup of the hottest allocation paths.

**Current Code Evidence**
- `Cargo.toml` has workspace package/dependency configuration but no `[workspace.lints]` policy.
- Each workspace crate manifest lacks `[lints] workspace = true`.
- `fuzz/Cargo.toml` is a separate workspace, so it cannot inherit the root lint table and needs mirrored lint policy or explicit check coverage.
- `rust-toolchain.toml` already pins nightly and includes `rustfmt` and `clippy`.
- `scripts/bench-quick.sh` runs `cargo clippy --workspace --all-targets -- -D warnings`, but not `--all-features`.
- `docs/todos/performance_kill_list/README.md` already treats Clippy as a global gate, but the repo does not yet enforce the requested deny policy in manifests.
- Initial grep found hundreds of `unwrap`, `expect`, `panic!`, `unreachable!`, and related smells across Rust files, mostly in tests but with production examples in `query.rs`, `sorted_trie.rs`, `hash_trie.rs`, and `bumbledb-bench/src/main.rs`.
- `QueryPlan` currently exposes `PlanCounters`, optimizer trace, QueryImage cache diagnostics, planner stats diagnostics, and Free Join plan summaries, but no `QueryTimings` or allocation summaries.
- Current tracing spans are coarse: `bumbledb.query.execute`, `bumbledb.query.plan`, `bumbledb.query_image.build`, `bumbledb.query.project`, and `bumbledb.query.aggregate` exist, but phase/operator timing is not complete.
- Query hot paths allocate through `EncodedValue { bytes: Vec<u8> }`, `EncodedBinding { values: Vec<Option<EncodedValue>> }`, per-depth participant `Vec`s, `BTreeSet<Vec<EncodedValue>>`, string cache keys, hash prefix `Vec`s, and `rows_owned()` row materialization.

**Strict Order**
1. `00_baseline_inventory_and_guardrails.md`
2. `01_workspace_lints_and_clippy_policy.md`
3. `02_panic_unwrap_and_smell_cleanup.md`
4. `03_query_observability_data_model.md`
5. `04_tracing_and_profiling_ux.md`
6. `05_allocation_recording_and_heap_observability.md`
7. `06_stack_gat_and_hot_path_allocation_cleanup.md`
8. `07_verification_and_handoff.md`

**Global Gates**
```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
scripts/check-cutover.sh
scripts/check-prd-map.sh
scripts/check-performance-kill-list.sh
cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 3 --format markdown
```

**Global Stop Conditions**
- Stop if the lint cleanup becomes a broad semantic rewrite instead of mechanical hardening.
- Stop if allocator instrumentation changes default library behavior for normal users.
- Stop if disabled tracing/profiling regresses focused scale-10000 benchmarks by more than 5% without documented cause.
- Stop if stack/GAT refactors change query results versus SQLite/reference tests.
- Stop if a change reintroduces LMDB cursor recursion, candidate-domain `BTreeSet<EncodedValue>` intersections, or a permanent second production executor outside Free Join/QueryImage.
- Stop if benchmark output cannot attribute query time to phases after the observability PRDs land.

**Return Point**
- After `07_verification_and_handoff.md`, resume `docs/todos/performance_kill_list/05_direct_selective_query_kernels.md`.

**Baseline Artifacts**
- `00_baseline_results.md`
- `01_workspace_lints_results.md`
- `02_panic_unwrap_results.md`
