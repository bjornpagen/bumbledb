# PRD 08: Recover Non-JOB Materialized Paths

## Goal

Recover the non-JOB materialized workload regressions after static proof containment and benchmark cache-mode work.

This PRD focuses on the remaining actual engine work after hidden proof costs are removed.

## Explicit Non-Goals

- No backwards compatibility.
- No restoring old row-payload storage.
- No reintroducing primary-key concepts.
- No relation-name-specific hacks for non-JOB datasets.
- No benchmark-only shortcuts that change engine semantics.

## Current Failing Non-JOB Queries

Latest failures:

```text
ledger/tag_lookup_join
sailors/red_boat_sailors
```

After PRDs 02 through 05, rerun and update this list. Some failures may disappear once static proof is contained.

## Required RCA After Static-Proof Fixes

For each remaining slow non-JOB query, classify the bottleneck:

```text
direct/index nested loop execution
LFTJ traversal
LFTJ build
materialized output decode
sink/dedup
query image load
planner stats
unaccounted
```

Do not optimize blindly.

## Required Fix Areas

### Direct Chain Materialization

If `tag_lookup_join` remains slow:

- inspect direct chain kernel for per-row allocation
- ensure direct kernel outputs encoded rows without unnecessary full-row clones
- ensure no static proof or query image work runs before direct chain when not needed

### LFTJ Materialized Output

If `red_boat_sailors` or `supplier_nation_orders` remain slow:

- inspect `EncodedProjectSink`
- inspect dedup structures
- inspect output decode
- inspect whether count-only cache/proof paths accidentally run for materialized queries
- consider streaming output for already-distinct direct/LFTJ outputs if correctness permits

### Full-Covering Key Width

If full-covering access paths are the bottleneck:

- measure key bytes scanned
- avoid comparing/decode unused suffix bytes where possible
- keep correctness and set semantics intact

## Required Tests

- Direct chain materialized query does not allocate or decode more values than output requires.
- LFTJ materialized projection still deduplicates correctly.
- Materialized output path does not use prepared result cache.
- Materialized output path does not run static semijoin proof unless preflight proves it cheap.

## Required Benchmarks

Run non-JOB after fixes.

Targets:

```text
tag_lookup_join < 250000us gate
red_boat_sailors < 250000us gate
supplier_nation_orders < 250000us gate
```

Stretch targets are the pre-v3 values:

```text
tag_lookup_join around 6-7ms
red_boat_sailors around 6-7ms
supplier_nation_orders around 3ms
```

If stretch targets require larger architectural work, document RCA and keep hard gates conservative.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- non-JOB benchmark gates pass.

## Completion Criteria

- Non-JOB materialized path regressions are fixed or precisely documented with new targeted PRDs.
- Existing non-JOB gates pass.
- This PRD is deleted and committed after passing.
