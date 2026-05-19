# 05 Direct Selective Query Kernels Results

**Completed Scope**
- Added direct kernel planning under the existing normalized QueryImage/Free Join query path.
- Added `DirectKernelPlan` summaries in executed `QueryPlan`s.
- Added `DirectKernelKind::{PointLookup, PrefixRange, ChainProbe, CountOnly}` with implemented `PrefixRange` and `ChainProbe` paths.
- Added `DirectKernel` runtime kind.
- Added direct counters: `direct_kernel_probes`, `direct_kernel_rows`, and `direct_kernel_predicates`.
- Reused existing `OutputSink` projection/aggregation machinery.
- Kept unsupported/cyclic shapes on existing Free Join/LFTJ or HashProbe fallbacks.

**Implemented Kernels**
- `PrefixRange`: equality prefix over one relation plus range predicates over one variable.
- `ChainProbe`: acyclic low-fanout chain where each later atom binds one new variable from already-bound values.

**Target Query Results**
Commands:

```sh
cargo run -p bumbledb-bench --release -- --dataset sailors --query sailor_range_reserves --scale 10000 --repeats 10 --warmup 3 --format markdown --fail-gates
cargo run -p bumbledb-bench --release -- --dataset joinstress --query chain4_from_a --scale 10000 --repeats 5 --warmup 3 --format markdown --fail-gates
```

| Dataset | Query | Runtime | Rows | Avg Us | P50 Us | P95 Us | Direct Probes | Direct Rows | Direct Predicates | Gate |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| sailors | sailor_range_reserves | DirectKernel | 5 | 37 | 36 | 41 | 1 | 5 | 10 | pass |
| joinstress | chain4_from_a | DirectKernel | 1 | 45 | 44 | 49 | 4 | 3 | 0 | pass |

Both target queries report `trie_open=0`, `trie_seek=0`, `trie_key_reads=0`, and `hash_probe_calls=0` when the direct kernel is selected.

**Generated Scale-10000 Smoke**
Command:

```sh
cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 3 --format markdown
```

Selected results:

| Dataset | Query | Runtime | Rows | Avg Us | Gate |
|---|---|---|---:|---:|---|
| sailors | sailor_range_reserves | DirectKernel | 5 | 41 | pass |
| joinstress | chain4_from_a | DirectKernel | 1 | 50 | pass |
| ledger | tag_lookup_join | DirectKernel | 10000 | 9434 | pass |
| joinstress | triangle_count | Lftj | 1 | 15977 | pass |

The direct chain matcher also selects `ledger/tag_lookup_join`, which is an acyclic chain shape and improved broad join runtime without changing semantics.

**Allocation Profile Check**
Command:

```sh
cargo run -p bumbledb-bench --features alloc-profile --release -- --dataset sailors --dataset joinstress --query sailor_range_reserves --query chain4_from_a --scale 10000 --repeats 3 --format markdown
```

| Dataset | Query | Runtime | Alloc Calls | Bytes Allocated | Net Bytes |
|---|---|---|---:|---:|---:|
| sailors | sailor_range_reserves | DirectKernel | 21612 | 34075244 | 1656205 |
| joinstress | chain4_from_a | DirectKernel | 13295 | 26943969 | 5329803 |

Allocation drops versus the hardening handoff baseline:

| Query | Before Calls | After Calls | Calls Delta | Before Bytes | After Bytes | Bytes Delta |
|---|---:|---:|---:|---:|---:|---:|
| sailor_range_reserves | 71109 | 21612 | -49497 | 39808560 | 34075244 | -5733316 |
| chain4_from_a | 43350 | 13295 | -30055 | 42213759 | 26943969 | -15269790 |

**Tests Added**
- Direct prefix/range kernel selects and filters rows.
- Direct prefix/range empty prefix returns zero rows.
- Direct chain kernel selects and follows an acyclic path.
- Direct chain broken path returns zero rows.
- Cyclic triangle remains rejected by direct planning and uses LFTJ.

**Verification**
- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- `scripts/check-cutover.sh`
- `scripts/check-prd-map.sh`
- `scripts/check-performance-kill-list.sh`
- `cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 3 --format markdown`
- Target query gates listed above.

**Next PRD**
- `docs/todos/performance_kill_list/06_optimize_lftj_inner_loop.md`
