# Trace-Backed Performance Kill List

This suite converts the scale-10000 trace RCA into an ordered set of implementation PRDs. The order is strict: each item removes a larger observed time bucket or unblocks the next item.

Trace source:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-trace-scale10000-r1.log
```

The trace shows that current query latency is split roughly as:

| Bucket | Time Across 10 Queries | Share |
|---|---:|---:|
| Setup/pre-plan | `268.3ms` | `54.0%` |
| Plan-to-exec bucket | `228.6ms` | `46.0%` |
| Total | `496.9ms` | `100%` |

The old LMDB recursion bottleneck is gone: all generated benchmark queries report `cursor_seeks=0`, `rows_scanned=0`, and `dictionary_reverse_lookups=0`. The current bottlenecks are repeated construction work and interpreted sorted-trie execution where direct/hash kernels should be used.

**Strict Order**
1. `01_cache_planner_stats.md`
2. `02_cache_query_image_indexes.md`
3. `03_route_queries_through_query_image_cache.md`
4. `04_real_hash_probe_runtime.md`
5. `05_direct_selective_query_kernels.md`
6. `06_optimize_lftj_inner_loop.md`
7. `07_improve_cardinality_estimates.md`
8. `08_add_phase_timing_and_tracing.md`

**Interstitial Hardening Pass**
- After `04_real_hash_probe_runtime.md`, run `../observability_lints_allocation_hardening/README.md` before starting `05_direct_selective_query_kernels.md`.
- This does not change the performance kill-list order. It adds strict linting, panic cleanup, phase timing, profiling UX, allocation recording, and first-pass stack/GAT cleanup so PRD 05 can be implemented with better measurements.
- Status: complete. See `../observability_lints_allocation_hardening/07_verification_handoff_results.md` for the handoff baseline.

**Global Gates**
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- `scripts/check-cutover.sh`
- `scripts/check-prd-map.sh`
- `cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 30 --format markdown`

**Global Stop Conditions**
- Stop if a change reintroduces LMDB prefix cursor construction in query recursion.
- Stop if a change reintroduces candidate-domain `BTreeSet<EncodedValue>` intersections.
- Stop if a change creates a permanent second production executor path outside Free Join/QueryImage.
- Stop if benchmark output cannot attribute the phase being optimized.
- Stop if a stage worsens a focused scale-10000 gate by more than 5% without documented cause.
