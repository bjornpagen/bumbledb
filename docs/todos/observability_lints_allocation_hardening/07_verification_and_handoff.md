# 07: Verification And Handoff

**Goal**
- Prove the hardening/observability pass is complete, documented, and safe to build on.
- Hand execution back to the trace-backed performance kill list at direct selective kernels.

**Required Verification Commands**
```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
scripts/check-cutover.sh
scripts/check-prd-map.sh
scripts/check-performance-kill-list.sh
cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 3 --format markdown
```

**Required Focused Benchmarks**
- Run `scripts/bench-focused.sh` at scale 10000 with at least 3 repeats.
- Run a single-query benchmark using the new query filter for each target query that PRD 05 will optimize.
- Run one trace-enabled benchmark to verify spans and phase summaries are usable.
- Run one allocation-profile-enabled benchmark to verify heap counters and phase allocation summaries are usable.

**Required Documentation Updates**
- Update `docs/BENCHMARKS.md` with the new benchmark flags, timing sections, allocation sections, and profiling examples.
- Update `docs/todos/README.md` status/order notes if the hardening suite is complete.
- Update `docs/todos/performance_kill_list/README.md` only if the interstitial hardening pass changes how future PRDs should run gates.
- Document any lint exceptions that remain and why they are acceptable.
- Document any profiler dependencies and feature flags.

**Required Regression Checks**
- Query result tests still pass.
- SQLite/reference comparisons still pass in the benchmark harness.
- Structural gates still enforce no LMDB cursor recursion and no dictionary reverse lookup regressions.
- Disabled tracing/profiling benchmark smoke is not more than 5% slower than the baseline without documented cause.
- Allocation profiling is off by default for normal library users.
- `rg` smell checks find no banned panic/debug sites outside documented exceptions.

**Commit Requirements**
- Commit the suite implementation only after all required gates pass.
- The commit message should explain that this is an observability/lint/allocation hardening pass, not a direct query kernel optimization.
- Do not squash unrelated user changes into the commit.

**Handoff Requirements**
- The final implementation note must state the next PRD explicitly:

```text
Next PRD: docs/todos/performance_kill_list/05_direct_selective_query_kernels.md
```

- Summarize the allocation and phase timing baseline that PRD 05 should use.
- Identify which PRD 05 target query has the worst setup, execution, sink, and allocation profile after hardening.
- Keep the performance kill-list order intact after the handoff.

**Passing Requirements**
- All global gates pass.
- Benchmark markdown includes phase timing, runtime kind, distribution stats, and allocation summaries.
- Trace-enabled output contains the required span names without raw values.
- Allocation-profile output reports total and per-phase heap metrics.
- Working tree is clean after the hardening commit if the implementer is asked to commit.

**Stop Conditions**
- Stop if any gate failure is unrelated but blocks confidence; diagnose before moving to PRD 05.
- Stop if benchmark output cannot guide PRD 05 implementation choices.
- Stop if direct selective kernel work starts before this handoff is complete.
