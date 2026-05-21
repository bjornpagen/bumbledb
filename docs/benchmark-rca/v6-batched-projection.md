# V6 Batched Encoded Projection Sink

## Purpose

Document the implementation and measurement result for replacing per-emit projection set insertion with an append-first encoded row buffer and sort/dedup finalization.

## Artifacts

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-batched-project-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-batched-project-hot-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-batched-project-job-10k.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/allocation-batched-project-hot-nonjob.json
```

Baseline artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-counters-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/allocation-hotset-nonjob.json
```

## Implementation Summary

`EncodedProjectSink` now stores projected rows as contiguous encoded bytes:

```text
[value0 bytes][value1 bytes]...[valueN bytes]
```

The sink now:

1. Builds an encoded projection layout once.
2. Appends row bytes during `emit`.
3. Sorts row indices at `finish` by comparing row byte slices.
4. Deduplicates adjacent equal encoded rows.
5. Decodes only unique rows at the final output boundary.

This removed `BTreeSet<SmallEncodedRow>` from `EncodedProjectSink` and avoided per-emit tree insertion.

## Benchmark Delta

| Query | Before us | After us | Delta | Project rows | Duplicates | Decoded values |
|---|---:|---:|---:|---:|---:|---:|
| ledger/postings_for_holder_range | 49 | 49 | 0% | 3 | 0 | 6 |
| ledger/balances_by_instrument | 50 | 52 | +4% | 0 | 0 | 0 |
| ledger/tag_lookup_join | 7069 | 5451 | -23% | 10000 | 0 | 20000 |
| sailors/red_boat_sailors | 7048 | 5296 | -25% | 16660 | 6660 | 20000 |
| sailors/sailor_range_reserves | 9 | 9 | 0% | 5 | 0 | 10 |
| sailors/high_rating_red_boats | 5504 | 4608 | -16% | 6660 | 0 | 13320 |
| joinstress/chain4_from_a | 16 | 16 | 0% | 1 | 0 | 1 |
| joinstress/triangle_count | 10579 | 10552 | 0% | 0 | 0 | 0 |
| tpch/revenue_by_customer_range | 2921 | 3003 | +3% | 0 | 0 | 0 |
| tpch/supplier_nation_orders | 3255 | 2511 | -23% | 5716 | 0 | 11432 |

## Allocation Delta

| Query | Alloc calls before | Alloc calls after | Delta | Bytes before | Bytes after | Delta bytes | Execute alloc before | Execute alloc after | Sink finish alloc before | Sink finish alloc after |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ledger/tag_lookup_join | 31889 | 30230 | -1659 | 32145530 | 30074782 | -2070748 | 21672 | 20010 | 10001 | 10004 |
| sailors/red_boat_sailors | 16112 | 14470 | -1642 | 20302951 | 18945507 | -1357444 | 1833 | 188 | 10001 | 10004 |
| sailors/high_rating_red_boats | 8787 | 7729 | -1058 | 3124542 | 1734426 | -1390116 | 1167 | 106 | 6661 | 6664 |
| tpch/revenue_by_customer_range | 12322 | 12322 | 0 | 49956607 | 49956607 | 0 | 2550 | 2550 | 2002 | 2002 |
| tpch/supplier_nation_orders | 14678 | 13575 | -1103 | 50755332 | 49265832 | -1489500 | 1332 | 226 | 5717 | 5720 |

## Target Results

Hard gates:

- non-JOB gates: pass
- JOB 10k gates: pass

Optimization targets:

- `red_boat_sailors`: target 15%, actual 25%, pass
- `high_rating_red_boats`: target 15%, actual 16%, pass
- `supplier_nation_orders`: target 10%, actual 23%, pass
- `tag_lookup_join`: no more than 5% regression, actual 23% improvement, pass
- `triangle_count`: no more than 5% regression, actual flat, pass

`revenue_by_customer_range` regressed by about 3%, but it is aggregate-heavy and does not use encoded projection rows. This is within acceptable noise for this PRD and remains under gates.

## Interpretation

The trace-based hypothesis was correct. Per-emit projection set insertion was a material cost for high-output materialized queries.

Important outcomes:

- high-output projection-heavy queries improved substantially
- allocation calls dropped in target paths
- execute-phase allocations dropped sharply for LFTJ projection-heavy queries
- sink-finish allocations remain approximately one per final output row because public `Vec<Vec<Value>>` output materialization still allocates final rows

The next bottleneck moved toward direct-chain mechanics and LFTJ iterator/key operations.

## Follow-Up

PRD 04 should target direct-chain batching for `tag_lookup_join`, which still has:

```text
direct chain step rows: 20000
direct chain output rows: 10000
sink emits: 10000
```

PRD 05 should target LFTJ iterator mechanics for:

```text
triangle_count
red_boat_sailors
high_rating_red_boats
supplier_nation_orders
```

## Compatibility Statement

No backwards compatibility. No migrations. Projection output semantics remain set-based and unchanged.
