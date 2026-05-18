# 05: Allocation Recording And Heap Observability

**Goal**
- Add idiomatic Rust allocation recording so query benchmarks can attribute heap churn and peak memory to query phases.
- Keep allocation recording opt-in and off by default for normal users.

**Rust Mechanism**
- Use `#[global_allocator]` with a `GlobalAlloc` wrapper for cheap allocation counters.
- Use atomics and fixed-size histograms inside allocation hooks.
- Do not allocate, lock, format strings, capture backtraces, or emit tracing events inside allocation hooks.
- Use deeper opt-in profilers such as `dhat` or jemalloc profiling for callsite attribution, because backtraces on every allocation will distort benchmarks.

**Required Allocation Counters**
- Allocation calls.
- Deallocation calls.
- Reallocation calls.
- Bytes allocated.
- Bytes deallocated.
- Current live bytes.
- Peak live bytes.
- Net bytes.
- Allocation size-class histogram using fixed buckets.

**Required Phase Attribution**
- Capture snapshots around query phases introduced in `03_query_observability_data_model.md`.
- At minimum attribute allocation deltas to validate, normalize, encode inputs, query image acquisition, planning, hash index build/lookup, LFTJ build, execution, sink finish, and total query.
- If per-phase attribution requires library hooks, add a minimal no-op observer interface used by normal `execute_query` and enabled by the benchmark/profile path.
- Do not introduce a permanent second executor API. An observed query path must delegate to the same normalized QueryImage/Free Join execution.

**Crate Placement Requirements**
- Normal library users must not get a custom global allocator by default.
- Prefer putting the global allocator wrapper in the benchmark binary behind a feature such as `alloc-profile`.
- If library phase hooks are needed, put only the hook trait/data structs in `bumbledb-lmdb`; keep allocator implementation in `bumbledb-bench` or another explicitly opt-in crate.
- Do not make allocation profiling a default workspace dependency for normal tests unless the dependency is tiny and has no runtime effect.

**Data Model Requirements**
- `QueryPlan` must carry allocation summary fields, defaulting to disabled/zero when allocation profiling is not active.
- Benchmark result rows must capture query-level and per-phase allocation deltas.
- Markdown and structured output must include allocation calls, bytes allocated, net bytes, and peak live bytes.
- Include size-class histogram in detailed output or JSON if markdown would become too wide.

**Correctness Requirements**
- Allocation counters must be monotonic where appropriate.
- Peak live bytes must be tracked with compare-exchange or equivalent atomic logic.
- Realloc accounting must handle both old and new layouts correctly.
- Counter snapshots must be cheap and not require stopping the world.
- Multi-threaded safety must be explicit even if current benchmark execution is single-threaded.

**Deeper Heap Profiling Requirements**
- Add docs for running a deeper callsite heap profiler after the cheap counters identify a problem.
- Acceptable options include `dhat` for Rust-oriented heap profiling or jemalloc profiling where supported.
- Deep heap profiling runs should be separate from normal benchmark gates.
- Deep heap profiling output should not be required for CI.

**Tests**
- Allocation counter snapshots increase after allocating a known vector in a profile-enabled test or bench-only test.
- Allocation deltas can be subtracted without underflow.
- Disabled allocation profiling reports `enabled=false` and zero plan allocation fields.
- Benchmark markdown renders allocation summary whether profiling is on or off.

**Passing Requirements**
- Default `cargo test --workspace --all-features` and benchmark smoke work without requiring special allocator environment setup.
- A profile-enabled benchmark run reports allocation counts and peak live bytes.
- Per-phase allocation deltas are visible for at least total query, query image, planning, index build, execution, and sink finish.
- Disabled allocation instrumentation does not regress focused scale-10000 smoke by more than 5%.

**Stop Conditions**
- Stop if the allocator hook allocates, locks, traces, or formats inside allocation/deallocation.
- Stop if enabling allocation profiling changes query results.
- Stop if per-phase allocation attribution requires duplicating query execution paths.
