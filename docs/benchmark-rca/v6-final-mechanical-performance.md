# V6 Final Mechanical Performance

## Purpose

Final RCA for the v6 mechanical performance pass.

V6 optimized retained primitives rather than adding algorithms. Free Join/LFTJ remained the backbone. The major query-side win came from changing output materialization mechanics, not from new planner regimes.

## Completed PRDs

```text
00-roadmap
01-measurement-counters-and-hotset
02-allocation-and-hardware-profiling
03-unified-batched-encoded-projection-sink
04-batched-direct-materialization
05-lftj-emission-and-iterator-mechanics
06-width-specialized-encoded-operations
07-query-image-and-trie-memory-layout
08-ingest-dictionary-and-index-write-layout
09-final-v6-validation-and-cleanup
```

## Final Artifacts

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-final-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-final-job-10k.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-final-job-q09-prepared-result.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-final-hot-nonjob.json
```

## Validation Results

```text
cargo fmt --all --check: pass
cargo check --workspace --all-targets --all-features: pass
cargo clippy --workspace --all-targets --all-features -- -D warnings: pass
cargo test --workspace --all-features: pass
cargo check --manifest-path fuzz/Cargo.toml: pass
```

## Implemented Optimizations

### Mechanics Counters

Added cheap benchmark-visible counters for:

- sink emits
- encoded project rows seen/inserted/duplicated
- encoded project row bytes
- project decode values
- direct chain rows/outputs/batch rows
- LFTJ open/up/next/seek/key/candidate/bind/completed counts
- query image loaded relation/row/byte counts

### Batched Encoded Projection Sink

Replaced per-emit `BTreeSet<SmallEncodedRow>` projection insertion with:

```text
append encoded row bytes during emit
sort row indices at finish
```

This was the largest v6 query win.

### Batched Direct Materialization

Direct materialized project outputs now append directly to the encoded projection sink and bypass generic sink emission.

### LFTJ Projection Emission

LFTJ completed bindings now append directly to the encoded projection sink for projection outputs and bypass generic sink emission.

### Width-Specialized Scalar Comparison

Added scalar width-dispatched encoded comparison helpers for width 1, 8, and 16 and used them in LFTJ key comparison paths.

No ARM NEON was implemented yet. No x86 SIMD was introduced.

## Rejected Or Deferred Work

### Query Image And Trie Layout

Deferred. Profiling did not prove query image/trie memory layout was the next highest-leverage query bottleneck after output/direct/LFTJ mechanics.

### Ingest Dictionary Cache

Rejected and reverted. A transaction-local dictionary cache did not improve JOB 10k load time and added complexity. Future ingest work should be a true bulk dictionary/index build pipeline, not a per-call cache.

### ARM NEON

Deferred. Scalar width dispatch was neutral/noisy. Future NEON work should target batch scans/intersections after layout makes contiguous vectorizable data obvious.

## Final Benchmark Summary

| Suite | Queries | BDB wins | BDB losses | Gate failures |
|---|---:|---:|---:|---:|
| Non-JOB | 10 | 3 | 7 | 0 |
| JOB 10k | 8 | 8 | 0 | 0 |

## Non-JOB Before/After

Baseline is v6 counter baseline. Final is v6 final.

| Query | Before us | Final us | Delta | Gate |
|---|---:|---:|---:|---|
| ledger/postings_for_holder_range | 49 | 51 | +4% | pass |
| ledger/balances_by_instrument | 50 | 51 | +2% | pass |
| ledger/tag_lookup_join | 7069 | 5274 | -25% | pass |
| sailors/red_boat_sailors | 7048 | 5074 | -28% | pass |
| sailors/sailor_range_reserves | 9 | 9 | 0% | pass |
| sailors/high_rating_red_boats | 5504 | 4414 | -20% | pass |
| joinstress/chain4_from_a | 16 | 16 | 0% | pass |
| joinstress/triangle_count | 10579 | 10123 | -4% | pass |
| tpch/revenue_by_customer_range | 2921 | 2956 | +1% | pass |
| tpch/supplier_nation_orders | 3255 | 2465 | -24% | pass |

## JOB Before/After

| Query | Before us | Final us | Delta | Gate |
|---|---:|---:|---:|---|
| job_broad_cast_keyword_company | 370 | 383 | +4% | pass |
| job_broad_movie_info_star | 429 | 452 | +5% | pass |
| job_q01_top_production | 183 | 191 | +4% | pass |
| job_q09_voice_us_actor | 900 | 900 | 0% | pass |
| job_q16_character_title_us | 593 | 592 | 0% | pass |
| job_q24_voice_keyword_actor | 636 | 648 | +2% | pass |
| job_movie_link_bridge | 126 | 127 | +1% | pass |
| job_q33_linked_series_companies | 55 | 54 | -2% | pass |

JOB stayed stable. V6 was mostly a non-JOB materialized query improvement pass.

## q09/q16/q24 Cache Behavior

Prepared-plan mode:

```text
q09: 900us, prepared_result_cache_hits=0, static_semijoin_proof_us=865
q16: 592us, static_empty_cache_hits=0, static_semijoin_proof_us=571
q24: 648us, static_empty_cache_hits=0, static_semijoin_proof_us=609
```

Prepared-result q09:

```text
q09: 55us, prepared_result_cache_hits=30
```

Cache behavior remains explicit and honest.

## Mechanics Counter Conclusions

Before v6, high-output materialized non-JOB queries spent too much time in per-binding output mechanics.

After v6:

- Direct project outputs bypass generic sink emission.
- LFTJ project outputs bypass generic sink emission.
- Encoded projection rows are buffered and deduped in batch.
- High-output materialized queries improved by about 20-28%.

Remaining mechanics bottlenecks:

- `triangle_count` remains LFTJ iterator/key-read dominated.
- `tag_lookup_join` still has direct chain traversal/binding overhead.
- `revenue_by_customer_range` is aggregate-heavy and did not benefit from projection batching.

## Allocation/Profile Conclusions

Allocation profiling showed projection-heavy queries had meaningful execute/sink allocation pressure. Batched projection reduced allocation calls and bytes for key hot queries:

- `tag_lookup_join`: alloc calls down by 1659, bytes down by about 2.1MB after projection batching
- `red_boat_sailors`: alloc calls down by 1642, bytes down by about 1.4MB
- `high_rating_red_boats`: alloc calls down by 1058, bytes down by about 1.4MB
- `supplier_nation_orders`: alloc calls down by 1103, bytes down by about 1.5MB

Sink-finish allocations remain tied to final public `Vec<Vec<Value>>` output materialization.

## Query Latency Conclusions

The data validated the trace-based thesis:

```text
batch output mechanics first
direct/LFTJ emit integration second
width scalarization later
layout only when profiling proves it
```

The most successful v6 work was not SIMD or layout. It was reducing high-frequency generic output work in the retained primitives.

## Ingest Conclusions

JOB ingest remains dominated by insert/dictionary/index-write mechanics. A naive transaction-local dictionary cache was rejected.

Future ingest work should be a dedicated bulk pipeline:

- collect dictionary candidates
- sort/dedup in memory
- assign intern IDs in batches
- build index slabs per relation/index
- write sequentially
- validate constraints in bulk where safe

## Remaining Known Bottlenecks

1. Direct chain traversal/binding overhead for `tag_lookup_join`.
2. LFTJ iterator/key-read volume for `triangle_count` and high-output joins.
3. Aggregate materialization path for `revenue_by_customer_range`.
4. Public row materialization allocations at final output boundary.
5. JOB ingest dictionary/index write amplification.
6. Potential ARM NEON batch scans/intersections after layout work exposes contiguous vectorizable data.

## Compatibility Statement

No backwards compatibility. No migrations. No permanent benchmark-only switches. No x86 SIMD.
