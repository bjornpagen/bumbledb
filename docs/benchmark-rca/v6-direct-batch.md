# V6 Direct Batch Materialization

## Purpose

Document the direct materialization batching pass.

This PRD added a direct projection path that appends direct materialized rows into the encoded projection sink without going through generic `TupleSink::emit`.

## Artifacts

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-direct-batch-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-direct-batch-focused.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-direct-batch-job-10k.json
```

Comparison baselines:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-counters-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-batched-project-nonjob.json
```

## Implementation Summary

`OutputSink` now exposes a direct projection append path for direct materialized execution:

```text
emit_direct_project(query, binding, counters)
```

For `OutputSink::Project`, this calls into the unified encoded projection sink and appends encoded row bytes directly.

For non-project sinks, it reports a fallback and lets the caller use generic emission.

Direct storage and direct chain materialized paths now use this path for project outputs.

## Direct Batch Counter Evidence

| Query | Runtime | Generic sink emits | Direct batch rows | Direct batch bytes | Fallback rows | Direct step rows | Direct output rows | Direct storage rows |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| ledger/tag_lookup_join | IndexNestedLoop | 0 | 10000 | 160000 | 0 | 20000 | 10000 | 0 |
| sailors/sailor_range_reserves | DirectKernel | 0 | 5 | 80 | 0 | 0 | 0 | 5 |
| joinstress/chain4_from_a | IndexNestedLoop | 0 | 1 | 8 | 0 | 3 | 1 | 0 |

The key structural result is that direct materialized outputs no longer use generic sink emission.

## Benchmark Delta From Original V6 Counter Baseline

| Query | Before us | After us | Delta |
|---|---:|---:|---:|
| ledger/tag_lookup_join | 7069 | 5391 | -24% |
| sailors/sailor_range_reserves | 9 | 9 | 0% |
| joinstress/chain4_from_a | 16 | 16 | 0% |

## Benchmark Delta From PRD 03 Batched Projection Baseline

| Query | Before us | After us | Delta | Generic sink emits | Direct batch rows |
|---|---:|---:|---:|---:|---:|
| ledger/tag_lookup_join | 5451 | 5391 | -1% | 0 | 10000 |
| sailors/sailor_range_reserves | 9 | 9 | 0% | 0 | 5 |
| joinstress/chain4_from_a | 16 | 16 | 0% | 0 | 1 |

## Target Results

Hard gates:

- non-JOB gates: pass
- JOB 10k gates: pass

Optimization targets:

- `tag_lookup_join`: target 25% over original v6 baseline, actual 24%, near miss
- `chain4_from_a`: no regression, pass
- `sailor_range_reserves`: no regression, pass

The near miss is acceptable because the previous PRD already captured most of the available projection-path win, and this PRD still removed the remaining generic sink-emission branch from direct materialized paths.

## Interpretation

The data says direct batching was architecturally correct but not the main remaining cost.

Before this PRD, PRD 03 had already made projection appends cheaper. This PRD then removed generic sink dispatch from direct paths, but the marginal runtime improvement was small.

Remaining `tag_lookup_join` cost is likely in direct chain traversal and binding mechanics:

```text
direct chain step rows: 20000
direct output rows: 10000
direct bind attempts/successes: high
```

PRD 05 or a follow-up should consider compact direct-chain binding state and avoiding repeated `EncodedBinding` operations.

## Compatibility Statement

No backwards compatibility. No migrations. Public output semantics remain unchanged.
