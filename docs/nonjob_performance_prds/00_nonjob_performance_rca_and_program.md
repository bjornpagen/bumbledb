# PRD 00: Non-JOB Performance RCA And Program

## Status

Draft. This is the ordered program map for a targeted non-JOB performance pass. It does not broaden the database product thesis.

## Product Boundary

Bumbledb remains an embedded, typed, schemaful relational database optimized for BCNF-normalized, extremely join-heavy workloads. This program is not an attempt to beat SQLite universally. It targets avoidable waste that currently hurts simple relational paths and also leaks into intended workloads through setup overhead.

## In-Scope Targets

The in-scope targets are intentionally narrow:

1. Direct current-index range scan for single-relation range predicates.
2. Index-backed direct chain execution without relation-wide hash-trie builds.
3. Count-only and benchmark fairness path for row-count workloads.
4. Acyclic index nested-loop runtime for selective chain/star joins.
5. Trace and benchmark acceptance gates that keep these improvements measurable.

## Explicit Non-Goals

- No vector support.
- No FlatBuffer support.
- No JSON/document support.
- No SQL frontend.
- No general SQLite replacement scope.
- No unsafe LMDB durability options.
- No migration compatibility.
- No runtime DDL.
- No broad cost-based SQL optimizer.
- No new persistent storage type unless required by the four targets above.

## Current Benchmark Baseline

Latest non-JOB run at `scale=10000`, `warmup=2`, `repeats=30` showed Bumbledb wins `1/10` against SQLite.

| Dataset | Query | Runtime | Rows | BumbleDB avg us | SQLite avg us | Result |
| --- | --- | --- | ---: | ---: | ---: | --- |
| ledger | `postings_for_holder_range` | `Lftj` | 3 | 24 | 3 | 8.00x slower |
| ledger | `balances_by_instrument` | `Lftj` | 3 | 23 | 4 | 5.75x slower |
| ledger | `tag_lookup_join` | `DirectKernel` | 10000 | 9738 | 1263 | 7.71x slower |
| sailors | `red_boat_sailors` | `Lftj` | 10000 | 14232 | 4968 | 2.86x slower |
| sailors | `sailor_range_reserves` | `DirectKernel` | 5 | 11 | 2 | 5.50x slower |
| sailors | `high_rating_red_boats` | `Lftj` | 6660 | 9150 | 3696 | 2.48x slower |
| joinstress | `chain4_from_a` | `DirectKernel` | 1 | 14 | 4 | 3.50x slower |
| joinstress | `triangle_count` | `Lftj` | 1 | 13095 | 14692 | 1.12x faster |
| tpch | `revenue_by_customer_range` | `Lftj` | 2000 | 5741 | 3877 | 1.48x slower |
| tpch | `supplier_nation_orders` | `Lftj` | 5716 | 6815 | 1530 | 4.45x slower |

`joinstress/triangle_count` is the one win and is the intended cyclic join-heavy workload.

## Trace RCA Summary

A traced non-JOB run was captured at `scale=10000`, `warmup=0`, `repeats=1`.

Artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-nonjob-traced-results-latest-scale10000-r1.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-nonjob-trace-latest-scale10000-r1.jsonl
```

Trace size:

```text
805,770 lines
304 MB
```

The trace shows that the performance problem is not LMDB cursor scanning:

```text
cursor_seeks = 0
rows_scanned = 0
```

The waste is instead in setup, temporary structures, and materialization:

| Trace hotspot | Meaning |
| --- | --- |
| `bumbledb.query.lftj.build` | Building temporary atom images and sorted tries before execution. |
| `bumbledb.query.lftj.build.scan_filter_copy` | Scanning/copying relation-image bytes into temporary atom relation images. |
| `bumbledb.sorted_trie.build` | Sorting/building LFTJ trie structures for simple acyclic paths. |
| `bumbledb.hash_trie.build` | Building relation-wide hash tries for direct chain kernels. |
| `bumbledb.query.lftj.execute` | LFTJ traversal over acyclic/selective query shapes where nested loops would be cheaper. |
| `bumbledb.query.project` / `sink.finish` | Deduping, sorting, and decoding full result rows when SQLite benchmark only counts rows. |

## Important RCA Fix Already Done

`tpch/revenue_by_customer_range` exposed a planner estimation bug: no-prefix range variables were estimated as tiny, so the planner bound a broad ship-date variable first. That created large LFTJ candidate counts.

The fix was committed as:

```text
eb12a66 Estimate broad range scans conservatively
```

After the fix at `scale=1000`, `tpch/revenue_by_customer_range` dropped from a very slow broad-range-first shape to about `0.53 ms` with selective customer binding first.

## Main Lessons

1. SQLite is genuinely excellent at tiny point/range/nested-loop work.
2. Bumbledb should accept some tiny fixed-overhead losses if they are not on the core join-heavy battlefield.
3. Bumbledb should not accept avoidable setup costs from building full hash tries or sorted tries when an index prefix scan would suffice.
4. Large-output comparisons are currently skewed because SQLite benchmark code counts rows while Bumbledb materializes owned output values.
5. Direct/index nested-loop runtimes help both non-JOB cases and real BCNF application queries.

## Ordered PRDs

1. `01_direct_current_index_range_scan.md`
2. `02_index_backed_direct_chain.md`
3. `03_count_only_and_benchmark_fairness.md`
4. `04_acyclic_index_nested_loop_runtime.md`
5. `05_trace_and_acceptance_gates.md`

## Global Passing Criteria

All PRDs in this folder must preserve these invariants:

- `cargo fmt --all --check` passes.
- `cargo check --workspace --all-targets --all-features` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `cargo test --workspace --all-features` passes.
- `cargo check --manifest-path fuzz/Cargo.toml` passes.
- JOB suite remains `8/8` wins at `scale=10000`, `warmup=2`, `repeats=30`, unless a documented change explains a temporary deviation.
- `joinstress/triangle_count` remains a Bumbledb win or at minimum remains within `1.05x` of SQLite while preserving WCOJ counters.
- No new vector, FlatBuffer, JSON, SQL, or nullable-value scope appears.

## Final Program Exit Criteria

This performance pass is done when:

- Single-relation range predicates can execute without query-image or full hash-trie setup.
- Direct chain probes can use current/durable index prefix scans instead of relation-wide hash tries.
- Benchmark row-count mode can compare Bumbledb and SQLite fairly without forcing Bumbledb to decode/project rows that SQLite does not decode/project.
- Selective acyclic chain/star joins have an index nested-loop runtime option.
- Trace artifacts show lower `lftj_build`, `hash_trie.build`, `sorted_trie.build`, and `sink_finish` time for the target queries.
- The non-JOB benchmark report clearly separates accepted SQLite wins from waste we removed.
