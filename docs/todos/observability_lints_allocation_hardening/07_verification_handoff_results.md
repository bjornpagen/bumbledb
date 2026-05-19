# 07 Verification And Handoff Results

**Completed Scope**
- The observability/lints/allocation hardening suite is complete.
- All direct panic/debug smell call sites remain removed from `crates/` and `fuzz/`.
- The workspace lint policy remains enforced with all-features Clippy.
- Query plans now carry runtime kind, phase timings, node summaries, and allocation summaries.
- Benchmark markdown and JSON now include phase timings, distribution stats, allocation summaries, and allocation phase details.
- `alloc-profile` remains opt-in and benchmark-binary-only.

**Global Verification**
- `cargo fmt --all --check`: pass.
- `cargo check --workspace --all-targets --all-features`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass.
- `cargo test --workspace --all-features`: pass.
- `cargo check --manifest-path fuzz/Cargo.toml`: pass.
- `scripts/check-cutover.sh`: pass.
- `scripts/check-prd-map.sh`: pass.
- `scripts/check-performance-kill-list.sh`: pass.
- `cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 3 --format markdown`: pass.

**Focused Benchmark**
Command:

```sh
BUMBLED_BENCH_REPEATS=3 scripts/bench-focused.sh
```

Latest focused generated run:

| Dataset | Query | Runtime | Rows | Bumbledb Avg Us | Gate |
|---|---|---|---:|---:|---|
| ledger | postings_for_holder_range | HashProbe | 3 | 221 | pass |
| ledger | balances_by_instrument | HashProbe | 3 | 222 | pass |
| ledger | tag_lookup_join | HashProbe | 10000 | 19327 | pass |
| sailors | red_boat_sailors | HashProbe | 10000 | 38158 | pass |
| sailors | sailor_range_reserves | HashProbe | 5 | 56 | pass |
| sailors | high_rating_red_boats | MixedFallback | 6660 | 12052 | pass |
| joinstress | chain4_from_a | HashProbe | 1 | 124 | pass |
| joinstress | triangle_count | Lftj | 1 | 16502 | pass |
| tpch | revenue_by_customer_range | HashProbe | 2000 | 54477 | pass |
| tpch | supplier_nation_orders | HashProbe | 5716 | 40154 | pass |

All focused structural counters stayed clean: `cursor_seeks=0`, `rows_scanned=0`, and `dictionary_reverse_lookups=0` for every generated query.

**PRD 05 Target Baseline**
Single-query commands:

```sh
cargo run -p bumbledb-bench --release -- --dataset sailors --query sailor_range_reserves --scale 10000 --repeats 3 --warmup 1 --format markdown
cargo run -p bumbledb-bench --release -- --dataset joinstress --query chain4_from_a --scale 10000 --repeats 3 --warmup 1 --format markdown
```

Prepared/cached baseline for direct selective kernels:

| Dataset | Query | Runtime | Rows | Prepare Us | Warmup Avg Us | Measured Avg Us | Plan Us | Hash Index Us | Execute Us | Sink Finish Us |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| sailors | sailor_range_reserves | HashProbe | 5 | 36291 | 75 | 44 | 16787 | 19427 | 19441 | 3 |
| joinstress | chain4_from_a | HashProbe | 1 | 16519 | 124 | 91 | 6924 | 9470 | 9531 | 1 |

The worst PRD 05 setup profile is `sailors/sailor_range_reserves`: high prepare time, high planning time, and high hash-index setup despite only 5 output rows.

The worst PRD 05 execution profile is also `sailors/sailor_range_reserves`: measured cached runtime is already small, but prepared/cached warmup still pays hash-index work that direct kernels should bypass.

The worst PRD 05 sink profile is neither target: sink finish is `3us` for `sailor_range_reserves` and `1us` for `chain4_from_a`, so PRD 05 should focus on setup/probe path bypass, not sink changes.

**Allocation-Profile Baseline**
Command:

```sh
cargo run -p bumbledb-bench --features alloc-profile --release -- --dataset joinstress --query chain4_from_a --scale 10000 --repeats 3 --format json
```

Observed target allocation data:

| Dataset | Query | Alloc Calls | Bytes Allocated | Net Bytes | Dominant Phase |
|---|---|---:|---:|---:|---|
| joinstress | chain4_from_a | 43350 | 42213759 | 17943451 | hash_index / execute |

From the full allocation-profile target run:

| Dataset | Query | Alloc Calls | Bytes Allocated | Net Bytes | Dominant Phase |
|---|---|---:|---:|---:|---|
| sailors | sailor_range_reserves | 71109 | 39808560 | 29801697 | hash_index / execute |
| joinstress | chain4_from_a | 43350 | 42213759 | 17943451 | hash_index / execute |

PRD 05 should use these as the starting allocation baselines. Direct kernels should sharply reduce allocation calls by bypassing generic HashProbe index setup and recursive probe execution for these simple shapes.

**Trace Verification**
Command:

```sh
RUST_LOG=bumbledb_lmdb=debug cargo run -p bumbledb-bench --release -- --trace-output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-handoff-trace.jsonl --trace-format json --dataset joinstress --query chain4_from_a --scale 100 --repeats 1 --format json
```

The trace contains required span lifecycle records, including:

- `bumbledb.query.validate_inputs`
- `bumbledb.query.normalize`
- `bumbledb.query.encode_inputs`
- `bumbledb.query.image`
- `bumbledb.query.plan`
- `bumbledb.query.plan.stats`
- `bumbledb.query.plan.variable_order`
- `bumbledb.query.plan.optimize_free_join`
- `bumbledb.query.free_join.dispatch`
- `bumbledb.query.hash.build_indexes`
- `bumbledb.query.hash.execute`
- `bumbledb.query.sink.emit`
- `bumbledb.query.sink.finish`
- `bumbledb.sorted_trie.build`
- `bumbledb.hash_trie.build`

Trace grep found no raw value payload examples such as `Cash`, `USD`, `Alice`, `Bob`, `String(`, or `Bytes(`.

**Lint Exceptions**
- Remaining `#[expect(...)]` annotations are documented PRD 01 exceptions for retained diagnostics fields, optimizer helper arity, compiled-plan scaffolding, and explicit-state reference recursion.
- No `#[allow(...)]` attributes remain under `crates/` or `fuzz/`.

**Profiler Dependencies**
- Default profiler dependency footprint remains light.
- `alloc-profile` uses a benchmark-local global allocator wrapper and `bumbledb-lmdb/allocation-telemetry`.
- No `dhat`, jemalloc profiler, `tracing-chrome`, `tracing-flame`, or `pprof` dependency was added; those remain optional future deep-profiling choices.

**Next PRD**
```text
Next PRD: docs/todos/performance_kill_list/05_direct_selective_query_kernels.md
```

The performance kill-list order remains intact. Resume with direct selective kernels next.
