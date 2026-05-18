# 00 Baseline Results

**Baseline Commit**
- HEAD before this PRD implementation: `76546e3 Add real hash probe runtime`.
- Worktree note: the hardening PRD suite docs were already uncommitted from the previous planning step and are part of this implementation commit.
- Baseline date: 2026-05-18.

**Scope**
- This PRD intentionally does not add lint denies, allocation instrumentation, tracing output formats, or query runtime behavior changes.
- One behavior-neutral formatting change was required: `cargo fmt --all --check` initially failed on `crates/bumbledb-lmdb/src/hash_trie.rs`, so `cargo fmt --all` was run before rerunning gates.

**Gate Results**
| Command | Result | Notes |
|---|---|---|
| `cargo fmt --all --check` | pass | Initial run failed on formatting in `hash_trie.rs`; passed after `cargo fmt --all`. |
| `cargo check --workspace --all-targets --all-features` | pass | All workspace targets checked. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | pass | Current Clippy passes before the stricter PRD 01 lint policy. |
| `cargo test --workspace --all-features` | pass | 96 total Rust tests/doc-tests executed or reported, with 3 ignored tests. |
| `cargo check --manifest-path fuzz/Cargo.toml` | pass | Fuzz targets checked. |
| `scripts/check-cutover.sh` | pass | No legacy hot-path regressions reported. |
| `scripts/check-prd-map.sh` | pass | Re-architecture PRD map remains consistent. |
| `scripts/check-performance-kill-list.sh` | pass | Performance kill-list docs remain consistent. |
| `cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 3 --format markdown` | pass | Structural counter gates passed. Release build emitted an unused/dead-code warning for `Failpoint::name`. |
| `BUMBLED_BENCH_REPEATS=3 scripts/bench-focused.sh` | pass | Focused generated baseline for `ledger`, `sailors`, `joinstress`, and `tpch`; structural counter gates passed. |

**Release Warning To Fix In PRD 01**
- `crates/bumbledb-lmdb/src/failpoints.rs:32`: method `Failpoint::name` is never used when building without `test-failpoints` active in the release benchmark command.
- This is part of the future `unused = "deny"` cleanup surface.

**Focused Benchmark Command**
```sh
BUMBLED_BENCH_REPEATS=3 scripts/bench-focused.sh
```

**Focused Benchmark Baseline**
| Dataset | Query | Rows | Bumbledb Avg Us | SQLite Avg Us | Ratio | Chosen Plan | Cursor Seeks | Rows Scanned | Dict Lookups | Gate |
|---|---|---:|---:|---:|---:|---|---:|---:|---:|---|
| ledger | postings_for_holder_range | 3 | 175 | 7 | 22.78 | hash_probe | 0 | 0 | 0 | pass |
| ledger | balances_by_instrument | 3 | 237 | 9 | 25.01 | hash_probe | 0 | 0 | 0 | pass |
| ledger | tag_lookup_join | 10000 | 19098 | 1277 | 14.95 | hash_probe | 0 | 0 | 0 | pass |
| sailors | red_boat_sailors | 10000 | 50119 | 5152 | 9.73 | hash_probe | 0 | 0 | 0 | pass |
| sailors | sailor_range_reserves | 5 | 64 | 8 | 7.70 | hash_probe | 0 | 0 | 0 | pass |
| sailors | high_rating_red_boats | 6660 | 11777 | 3823 | 3.08 | hash_probe | 0 | 0 | 0 | pass |
| joinstress | chain4_from_a | 1 | 106 | 8 | 12.96 | hash_probe | 0 | 0 | 0 | pass |
| joinstress | triangle_count | 1 | 16538 | 14943 | 1.11 | aggregate_pushdown | 0 | 0 | 0 | pass |
| tpch | revenue_by_customer_range | 2000 | 68372 | 3907 | 17.50 | hash_probe | 0 | 0 | 0 | pass |
| tpch | supplier_nation_orders | 5716 | 54689 | 1591 | 34.37 | hash_probe | 0 | 0 | 0 | pass |

**Focused Benchmark Counter Notes**
- Every generated query reported `cursor_seeks=0` and `rows_scanned=0`.
- Every generated query reported `dictionary_reverse_lookups=0`.
- All configured benchmark counter gates passed.
- Broad hash-probe joins remain much slower than the final target and are intentionally deferred until after this hardening suite returns to `performance_kill_list/05_direct_selective_query_kernels.md`.

**Smell Inventory Command**
```sh
rg -n 'unwrap\(|expect\(|panic!\(|todo!\(|unimplemented!\(|unreachable!\(|dbg!\(' crates fuzz --glob '*.rs'
```

**Smell Inventory Summary**
| Category | Count | Notes |
|---|---:|---|
| Production library code | 5 | Manual classification of matches before `#[cfg(test)]` modules in non-benchmark crates. |
| Benchmark binary code | 28 | CLI parsing and benchmark row conversion helpers in `crates/bumbledb-bench/src/main.rs`. |
| Test code | 430 | Unit tests inside source files plus integration/UI tests. |
| Fuzz code | 0 | No matches found under `fuzz/`. |
| Total under `crates/` and `fuzz/` | 463 | Grep inventory before PRD 01 cleanup. |

**Production Library Smell Sites**
| File | Line | Smell | Classification For PRD 02 |
|---|---:|---|---|
| `crates/bumbledb-core/src/datalog.rs` | 628 | `unwrap()` | Replace lexer string parsing with safe `Option` handling. |
| `crates/bumbledb-lmdb/src/hash_trie.rs` | 233 | `unreachable!()` | Rewrite `insert_row` control flow explicitly. |
| `crates/bumbledb-lmdb/src/query.rs` | 3491 | `unwrap()` | Replace aggregate key iterator unwrap with typed internal error. |
| `crates/bumbledb-lmdb/src/query.rs` | 3497 | `unwrap()` | Replace aggregate state iterator unwrap with typed internal error. |
| `crates/bumbledb-lmdb/src/sorted_trie.rs` | 276 | `expect()` | Change trie key access to return `Option` or `Result`. |

**Benchmark Binary Smell Sites**
- `crates/bumbledb-bench/src/main.rs:124-146`: CLI missing values and numeric parse failures use `expect`; invalid format uses `panic!`.
- `crates/bumbledb-bench/src/main.rs:157`: unknown arg uses `panic!`.
- `crates/bumbledb-bench/src/main.rs:1596-1640`: row conversion helpers use `unwrap()` and `panic!` on unexpected field types.
- PRD 02 should convert CLI parsing to typed errors and row helper failures to typed benchmark errors.

**Test Smell Scope**
- Most test matches are straightforward fallible setup and assertion unwraps.
- PRD 01 should not globally allow unwraps in tests.
- PRD 02 should convert tests to `Result`-returning tests and use `?`, `ok_or_else`, or direct `Option` assertions.

**Existing Allow Sites**
| File | Line | Current Allow | Classification |
|---|---:|---|---|
| `crates/bumbledb-lmdb/src/query.rs` | 1955 | `clippy::too_many_arguments` | Replace with `#[expect(..., reason = "planner cost helper has explicit inputs")]` or refactor after PRD 03. |
| `crates/bumbledb-lmdb/src/query.rs` | 2444 | `clippy::too_many_arguments` | Replace with `#[expect(..., reason = "candidate builder mirrors optimizer inputs")]` or refactor after PRD 03. |
| `crates/bumbledb-lmdb/src/query.rs` | 3296 | `dead_code` | Remove compiled-plan scaffold unless PRD 03 needs it immediately. |
| `crates/bumbledb-lmdb/src/query.rs` | 3308 | `dead_code` | Remove compiled-plan scaffold unless PRD 03 needs it immediately. |
| `crates/bumbledb-lmdb/src/query.rs` | 3315 | `dead_code` | Remove compiled-plan scaffold unless PRD 03 needs it immediately. |
| `crates/bumbledb-lmdb/src/query.rs` | 5079 | `clippy::too_many_arguments` | Replace with `#[expect(..., reason = "test reference evaluator recursion carries explicit state")]` or refactor test helper. |
| `crates/bumbledb-lmdb/src/lib.rs` | 77 | `dead_code` | Replace with `#[expect(...)]`; field keeps fixed LMDB database handle open. |
| `crates/bumbledb-lmdb/src/lib.rs` | 79 | `dead_code` | Replace with `#[expect(...)]`; field keeps fixed LMDB database handle open. |
| `crates/bumbledb-lmdb/src/lib.rs` | 88 | `dead_code` | Remove unless path diagnostics need it now. |
| `crates/bumbledb-lmdb/src/lib.rs` | 446 | `dead_code` | Replace with `#[expect(...)]` or cfg-test the field if only test metadata helpers need it. |
| `crates/bumbledb-lmdb/src/failpoints.rs` | 49 | `dead_code` | Replace with feature/cfg gating or `#[expect(...)]`; used by failpoint tests. |
| `crates/bumbledb-lmdb/src/failpoints.rs` | 55 | `dead_code` | Replace with feature/cfg gating or `#[expect(...)]`; used by failpoint tests. |
| `crates/bumbledb-test-support/src/reference.rs` | 68 | `clippy::too_many_arguments` | Replace with `#[expect(..., reason = "reference recursion carries explicit state")]` or refactor test support recursion. |

**Dependency Footprint Before Hardening Dependencies**
| Scope | Dependencies |
|---|---|
| Root workspace path crates | `bumbledb-core`, `bumbledb-lmdb` |
| Root workspace third-party crates | `blake3 1.8.2`, `csv 1.4.0`, `heed 0.22.1` with default features off, `proptest 1.9.0`, `rusqlite 0.37.0` with `bundled`, `tempfile 3.23.0`, `thiserror 2.0.17`, `tracing 0.1.41`, `tracing-subscriber 0.3.20` with `env-filter` and `fmt`, `trybuild 1.0.114` |
| Fuzz workspace | `bumbledb-core` path dependency, `libfuzzer-sys 0.4` |
| Not present yet | `smallvec`, `arrayvec`, `dhat`, jemalloc allocator/profiling crates, `tracing-chrome`, `tracing-flame`, `pprof` |

**Known Expected Work For PRD 01**
- Add `[workspace.lints]` with `unused = "deny"` and strict Clippy bans.
- Add `[lints] workspace = true` to every normal workspace crate.
- Mirror lint policy in `fuzz/Cargo.toml` or enforce equivalent fuzz checks.
- Update `scripts/bench-quick.sh` to run Clippy with `--all-features`.
- Resolve the `Failpoint::name` release warning before `unused = "deny"` is enforced.

**Known Expected Work For PRD 02**
- Remove the five production library panic/smell sites listed above.
- Convert benchmark CLI and row conversion helper panics to typed errors.
- Convert test unwraps to `Result`-returning tests and `?`.
- Replace broad `#[allow]` sites with removals or targeted `#[expect(..., reason = "...")]`.

**Next PRD**
- `docs/todos/observability_lints_allocation_hardening/01_workspace_lints_and_clippy_policy.md`.
