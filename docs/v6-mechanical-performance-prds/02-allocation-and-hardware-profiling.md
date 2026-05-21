# PRD 02: Allocation And Hardware Profiling

## Goal

Determine whether allocations, cache locality, branch behavior, or raw instruction count dominate the current hotset before changing layouts.

This PRD is measurement and analysis. It may add profiling commands/scripts if needed, but it must not optimize engine behavior.

## Background

Heavy tracing proves high-output materialized workloads are high-frequency execution/emission problems. It does not prove allocation cost. It does not prove cache misses. It does not prove bit packing is useful.

This PRD decides which mechanical hypothesis is strongest:

```text
allocation pressure
pointer chasing / cache locality
branchy iterator logic
encoded comparison throughput
dictionary/intern overhead
bulk index write amplification
```

## Explicit Non-Goals

- No backwards compatibility.
- No layout changes.
- No optimizer changes.
- No new algorithms.
- No permanent profiling feature flags.
- No benchmark threshold relaxation.

## Required Hot Query Set

Profile these queries individually:

```text
ledger/tag_lookup_join
sailors/red_boat_sailors
sailors/high_rating_red_boats
joinstress/triangle_count
job/job_q09_voice_us_actor
job/job_q16_character_title_us
job/job_q24_voice_keyword_actor
job/job_movie_link_bridge
```

Also profile ingest for JOB 10k:

```text
JOB streaming load open-limit 10000
```

## Required Profiling Dimensions

### Allocation Profiling

Use existing allocation telemetry if available. Otherwise add temporary or benchmark-only allocator profile wiring that is not left enabled by default.

For each query, report:

```text
alloc_calls
bytes_allocated
peak_live_bytes
alloc_calls_per_output_row
bytes_allocated_per_output_row
alloc_calls_per_binding
```

Break down by phase if available:

```text
normalize
query_image
plan
lftj_build
execute
sink_finish
decode
```

### Hardware Counters

On macOS, use practical local tools if available. Acceptable options:

```text
samply
Instruments CLI/manual run notes
dtrace where permitted
perf-like wrapper if installed
```

If hardware counters are not available, document that and use trace + allocation + wall-clock artifacts instead.

Desired counters:

```text
cycles
instructions
branch misses
L1/L2/cache misses
load/store stalls
```

### Flamegraph or Sampling Profile

Produce at least one sampling profile for:

```text
red_boat_sailors
tag_lookup_join
JOB load 10k
```

If tooling is unavailable, document the exact attempted command and failure.

## Required Artifacts

Store artifacts under:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-profiles/
```

Suggested files:

```text
allocation-hotset.json
allocation-hotset.md
sampling-red-boat.*
sampling-tag-lookup.*
sampling-job-load.*
hardware-counter-notes.md
```

Create durable RCA:

```text
docs/benchmark-rca/v6-allocation-hardware-profile.md
```

## Required Analysis Questions

Answer explicitly:

- Is `EncodedProjectSink` allocation-heavy?
- Are high-output LFTJ queries dominated by allocation, branchy traversal, or set insertion?
- Does direct chain materialization allocate per row?
- Are dictionary intern lookups allocating or mostly map lookup/cache misses?
- Are query image/trie builds cache-local enough for JOB?
- Does width-specialized comparison look likely to move current bottlenecks?

## Required Tests Or Checks

No new correctness tests are required unless profiling hooks require code changes.

Required checks:

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`

If profiling code is added:

- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`

## Passing Requirements

- hotset allocation profile exists
- profiling RCA exists
- attempted hardware/sampling profile is documented
- allocation-vs-layout-vs-vectorization recommendation is explicit
- no engine behavior changes are included unless needed for safe profiling hooks

## Completion Criteria

- PRD 03/04/05 can prioritize from allocation/hardware evidence instead of trace inference alone.
- This PRD is deleted and committed after passing.
