# V6 Mechanical Performance PRD Roadmap

## Purpose

This suite turns the current heavy-trace research into an ordered, implementation-ready mechanical performance program.

The v4/v5 work made benchmark behavior honest and deleted redundant algorithm regimes. The remaining performance problem is not lack of algorithms. The retained engine is intentionally small:

- direct kernels
- static empty proof
- factorized count
- pure Free Join/LFTJ
- encoded projection and aggregate sinks

The next work is to make those retained paths mechanically fast: fewer per-binding operations, fewer allocations, better locality, width-specialized comparisons, cleaner memory layout, and less branching.

## Trace Evidence

Primary RCA document:

```text
docs/benchmark-rca/current-heavy-trace-analysis.md
```

Primary artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces/nonjob-benchmark.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces/nonjob-trace.jsonl
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces/job-10k-benchmark.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/current-traces/job-10k-trace.jsonl
```

Critical findings:

- High-output non-JOB materialized queries explode under tracing by 13x to 27x.
- JOB queries mostly slow only 1.1x to 2.4x under tracing.
- Non-JOB trace contains about 1.55M `sink.emit` events.
- `lftj.execute -> sink.emit` accounts for about 1.22M child spans.
- `execute_prepared_with_options -> sink.emit` accounts for about 330K direct child spans.
- `sink_finish_us` is small, so the materialization cost is happening during execution/emission, not only finalization.
- JOB ingest is dominated by `insert`, `dict_intern`, and index-entry writes.

## Optimization Thesis

Optimize retained primitives, not algorithm selection.

The next wins should come from:

- batch-oriented encoded projection
- direct-chain batch materialization
- LFTJ emit-path and iterator mechanics
- width-specialized encoded comparison/intersection
- query image and trie memory layout locality
- dictionary and bulk-load write-path layout

Do not start by adding algorithms. Do not add permanent feature flags. Do not preserve compatibility with old internal layouts. This is a research database.

## Explicit Non-Goals

- No backwards compatibility.
- No migrations.
- No old storage readers.
- No permanent feature flags.
- No preserving internal APIs.
- No relation-name-specific benchmark hacks.
- No adding a new join algorithm.
- No reintroducing mixed/hash-probe/tiny-project regimes.
- No SQL/Datalog frontend work.

## Ordered PRDs

Implement in order:

1. `01-measurement-counters-and-hotset.md`
2. `02-allocation-and-hardware-profiling.md`
3. `03-unified-batched-encoded-projection-sink.md`
4. `04-batched-direct-materialization.md`
5. `05-lftj-emission-and-iterator-mechanics.md`
6. `06-width-specialized-encoded-operations.md`
7. `07-query-image-and-trie-memory-layout.md`
8. `08-ingest-dictionary-and-index-write-layout.md`
9. `09-final-v6-validation-and-cleanup.md`

## Required Final State

- Benchmark artifacts expose enough cheap counters to explain hot loops without heavyweight traces.
- Materialized projection has a unified, batch-oriented encoded row path.
- Direct materialized chains avoid per-binding sink overhead where possible.
- LFTJ emits and iterator operations are measured and mechanically optimized.
- Width 1 and width 8 encoded operations have specialized fast paths where benchmarks justify them.
- Query image/trie memory layout is either improved or defended with measured evidence.
- Ingest bottlenecks are measured and targeted, especially dictionary interning and index-entry writes.
- Non-JOB gates pass.
- JOB 10k gates pass.
- q09/q16/q24 cache behavior remains honest.

## Final Validation Gates

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
cargo run -p bumbledb-bench --release -- --preset nonjob --format json
cargo run -p bumbledb-bench --release -- --preset job --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb --open-limit 10000 --format json
```

## PRD Discipline

Each PRD must be deleted after it passes and its changes are committed.

Each PRD must either:

- implement the optimization and prove it with measurements, or
- document why the measured evidence rejected the optimization and leave the code simpler than before if possible.

No PRD may leave permanent diagnostic-only branches or disable switches in production code.

## Passing Requirements

- Ordered PRD suite exists under `docs/v6-mechanical-performance-prds`.
- Every implementation PRD has strict validation gates.
- Every implementation PRD explicitly rejects backwards compatibility and permanent disable switches.
- Roadmap captures trace evidence, optimization thesis, final gates, and PRD discipline.

## Completion Criteria

- Future agents can execute v6 mechanically from this directory without reading the conversation.
- This roadmap is deleted and committed when the v6 PRD loop begins.
