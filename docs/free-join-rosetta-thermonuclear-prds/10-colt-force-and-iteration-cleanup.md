# PRD 10: COLT Force And Iteration Cleanup

## Purpose

After physical reads and planner shape are fixed, attack the remaining runtime cost: COLT force, cover iteration, probe allocation, and map construction.

## Rosetta Alignment

This is internal execution machinery. It must preserve exact set semantics and LMDB snapshot visibility.

## Paper Alignment

COLT must remain lazy. It builds subtries only when `get` or non-suffix `iter` requires them. Optimizing COLT must not turn it into an eager trie.

## Current Trace Evidence

In the baseline exclusive runtime cost after base-image loading:

| phase | exclusive ms |
| --- | ---: |
| `ColtForce` | 22.63 |
| `ColtIter` | 22.61 |
| `ProbeSibling` | 4.32 |
| `BindingExtend` | 3.49 |
| `ColtGet` | 0.78 |

## Required Work

Eliminate avoidable allocation in `force`:

- remove per-force `child_counts: Vec<u32>` allocation from the hot path;
- use arena scratch or preallocated query-local scratch;
- avoid second pass when all child counts are one;
- avoid allocating offset ranges for singleton children;
- keep duplicate-heavy behavior proportional to distinct keys plus duplicate ranges.

Optimize cover iteration:

- verify suffix iteration does not allocate per tuple;
- avoid repeated tuple schema/key setup when possible;
- keep `InlineTuple` hot and bounded;
- trace tuple yield cost accurately without overwhelming trace size.

Optimize probes:

- ensure probe key construction uses scratch only;
- avoid source-frame churn when sibling probe cannot change source;
- do not allocate labels in no-trace release.

## Tests Required

- Duplicate-heavy force allocation remains proportional to distinct keys, not rows.
- Distinct-heavy force allocation is bounded and traceable.
- Repeated probes after force allocate zero or a small constant.
- Suffix iteration allocation is independent of row count.
- COLT laziness tests from the paper-shaped clover fixture still pass.
- All vectorized/scalar equivalence tests still pass.

## Trace Requirements

Add counters if missing:

- force first pass rows;
- force second pass rows;
- singleton children;
- duplicate children;
- scratch reuse hits;
- source replacements avoided.

## Benchmark Passing Criteria

Run full traced JOB sample.

Required evidence:

- `ColtForce` exclusive time drops materially against PRD 09.
- `ColtIter` exclusive allocation drops materially against PRD 09.
- `colt_offsets_scanned` does not increase without an explained plan improvement.
- Exact SQLite comparisons pass for all 8 JOB sample queries.
