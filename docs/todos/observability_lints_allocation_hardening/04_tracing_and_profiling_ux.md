# 04: Tracing And Profiling UX

**Goal**
- Make benchmarks and trace runs useful enough to diagnose correctness and performance regressions without custom timestamp archaeology.
- Preserve the rule that the library never installs a tracing subscriber.

**Current Code Evidence**
- `crates/bumbledb-bench/src/main.rs` installs a `tracing_subscriber::fmt()` subscriber only when `--trace` is passed.
- The benchmark CLI currently supports `--scale`, `--repeats`, `--trace`, `--format`, `--markdown`, `--fail-gates`, `--dataset`, and open dataset directories.
- Markdown output currently has `Benchmark Results` and `Counter Gates` sections.
- Timing loops currently report only total repeated duration and average duration.

**Required CLI Additions**
- Add query filtering with `--query NAME`, repeatable.
- Add warmup count with `--warmup N`, defaulting to a documented value.
- Add sample count or preserve `--repeats` as sample count with clear semantics.
- Add prepared/warm timing mode that separates initial typecheck/image/planning warmup from repeated cached execution.
- Add structured output mode, preferably `--format json` or `--json`, for machine-readable benchmark results.
- Add optional trace output path with `--trace-output PATH`.
- Add optional trace format selection such as `fmt`, `json`, `chrome`, or `flame`, but only implement formats whose dependencies are acceptable and gated.

**Benchmark Statistics Requirements**
- Keep current average timing for continuity.
- Add min, max, p50, p90 or p95, and sample count for Bumbledb and SQLite.
- Keep SQLite comparison, but clearly distinguish Bumbledb phase timings from SQLite total timings.
- Include warmup timings separately from measured samples.
- Include runtime kind in every result row.
- Include phase timing summaries from `QueryPlan.timings`.
- Include allocation summaries when allocation profiling is enabled.

**Markdown Requirements**
- Preserve existing `## Benchmark Results` and `## Counter Gates` sections unless a migration note is added.
- Add `## Phase Timing`.
- Add `## Runtime Kind` or include runtime kind in main results and phase timing tables.
- Add `## Allocation Summary` with disabled/zero values when allocation profiling is off.
- Add `## Distribution` with min/p50/p95/max.
- Add interpretation notes for high image time, high planning time, high index build time, high execution time, high sink finish time, and high allocation counts.

**Tracing Requirements**
- Keep default benchmark runs free of subscriber setup unless `--trace` is passed.
- Respect `RUST_LOG` when provided.
- Use safe default filter when tracing is enabled and `RUST_LOG` is absent.
- Do not emit raw values, strings, bytes, row payloads, or query result values.
- Do not emit per-candidate or per-row trace events by default.
- Add profile-summary events at query, phase, runtime kind, and node summary granularity.

**Optional Profiler Dependencies**
- `tracing-chrome` is acceptable for Chrome trace output if feature-gated in the benchmark crate.
- `tracing-flame` is acceptable for flamegraph-like tracing if feature-gated.
- `pprof` is acceptable for CPU profiling if feature-gated and documented as platform-dependent.
- Optional profiling dependencies must not become transitive defaults for normal library users.

**Tests**
- CLI parser accepts repeated `--query` filters.
- CLI parser rejects unknown args and invalid numeric values with typed errors.
- Markdown renderer includes phase timing, distribution, runtime kind, and allocation sections.
- JSON or structured output round-trips enough fields to support future CI comparison.
- Trace subscriber setup remains benchmark-binary-only.

**Passing Requirements**
- Existing documented benchmark commands still work.
- `--dataset` plus `--query` can run a single query for focused diagnosis.
- Benchmark markdown can explain where query time went without reading raw trace logs.
- Disabled tracing/profiling does not regress focused scale-10000 smoke by more than 5%.

**Stop Conditions**
- Stop if profiling dependencies affect default build time or binary behavior significantly.
- Stop if benchmark UX changes break existing scripts without compatibility wrappers.
- Stop if trace output includes user data or row payloads.
