# V6 LFTJ Emission And Iterator Mechanics

## Purpose

Document the LFTJ emission optimization pass.

Pure Free Join/LFTJ remains the backbone. This PRD optimized how completed LFTJ bindings reach the encoded projection sink.

## Artifacts

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-lftj-mechanics-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-lftj-mechanics-focused.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-lftj-mechanics-job-10k.json
```

Baselines:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-counters-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-batched-project-nonjob.json
```

## Implementation Summary

LFTJ completed bindings now try to append directly into the encoded projection sink via the batch projection path.

If the output sink is a projection sink:

- LFTJ avoids generic `TupleSink::emit` dispatch.
- Encoded rows are appended directly to the batched projection buffer.
- Set semantics and final-boundary decoding are preserved.

If the output is aggregate/count/fallback:

- LFTJ keeps the existing generic sink behavior.

## Benchmark Delta From Original V6 Counter Baseline

| Query | Before us | After us | Delta | Sink emits | Project rows | LFTJ next | LFTJ seek | LFTJ key reads |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| sailors/red_boat_sailors | 7048 | 4969 | -29% | 0 | 16660 | 34153 | 17491 | 105789 |
| sailors/high_rating_red_boats | 5504 | 4376 | -20% | 0 | 6660 | 34153 | 17493 | 105793 |
| joinstress/triangle_count | 10579 | 10278 | -3% | 0 | 0 | 90000 | 119995 | 589992 |
| tpch/revenue_by_customer_range | 2921 | 2872 | -2% | 8000 | 0 | 20000 | 4000 | 40002 |
| tpch/supplier_nation_orders | 3255 | 2383 | -27% | 0 | 5716 | 18577 | 7143 | 50013 |

## Benchmark Delta From PRD 03 Batched Projection Baseline

| Query | Before us | After us | Delta |
|---|---:|---:|---:|
| sailors/red_boat_sailors | 5296 | 4969 | -6% |
| sailors/high_rating_red_boats | 4608 | 4376 | -5% |
| joinstress/triangle_count | 10552 | 10278 | -3% |
| tpch/revenue_by_customer_range | 3003 | 2872 | -4% |
| tpch/supplier_nation_orders | 2511 | 2383 | -5% |

## Target Results

Hard gates:

- non-JOB gates: pass
- JOB 10k gates: pass

Optimization targets from original v6 counter baseline:

- `red_boat_sailors`: target 20%, actual 29%, pass
- `high_rating_red_boats`: target 20%, actual 20%, pass
- `supplier_nation_orders`: target 15%, actual 27%, pass
- `triangle_count`: no more than 5% regression, actual 3% improvement, pass
- `revenue_by_customer_range`: target 15%, actual 2%, miss with explanation

`revenue_by_customer_range` is aggregate-heavy and does not use encoded projection rows. Its remaining cost is not the LFTJ projection emission path optimized here. It needs aggregate-specific mechanics if it becomes a priority.

## Interpretation

The LFTJ emit integration was worthwhile. It removed generic sink emit calls from projection-heavy LFTJ queries and produced additional improvements on top of PRD 03.

The remaining high LFTJ operation counts show where PRD 06 should focus:

```text
triangle_count: 799987 LFTJ operations
red_boat_sailors: 157433 LFTJ operations
high_rating_red_boats: 157439 LFTJ operations
supplier_nation_orders: 75733 LFTJ operations
```

The next likely query-side wins are width-specialized encoded key operations and iterator/key-read reduction.

## Compatibility Statement

No backwards compatibility. No migrations. Pure Free Join/LFTJ remains the general join backbone.
