# 00: Baseline Inventory And Guardrails

**Goal**
- Establish a concrete baseline before changing lints, panic policy, timing, allocation profiling, or hot-path allocation strategy.
- Make sure the hardening pass stays focused and does not accidentally become another query-engine rewrite.

**Current Code Evidence**
- Root `Cargo.toml` has workspace members, workspace package metadata, and shared dependencies, but no lint policy.
- `rust-toolchain.toml` already provides nightly, `rustfmt`, and `clippy`.
- `scripts/bench-quick.sh` runs tests, Clippy, fuzz check, and a scale-2000 benchmark, but Clippy is not run with `--all-features`.
- `scripts/bench-focused.sh` and `scripts/bench-extreme.sh` benchmark generated workloads and are the right smoke surfaces for regression checks.
- `crates/bumbledb-lmdb/src/query.rs` is the main integration point for normalization, image acquisition, planning, Free Join dispatch, LFTJ execution, HashProbe execution, output sinks, counters, and tests.
- `crates/bumbledb-lmdb/src/query_image.rs` owns QueryImage construction and caches for planner stats, sorted tries, and hash tries.
- `crates/bumbledb-lmdb/src/sorted_trie.rs` owns the LFTJ iterator API and currently has a production `expect` in `SortedTrieIter::key`.
- `crates/bumbledb-lmdb/src/hash_trie.rs` owns hash-trie prefix probes and currently has an `unreachable!()` inside `insert_row`.
- `crates/bumbledb-bench/src/main.rs` owns benchmark CLI parsing, trace subscriber setup, timing loops, gate evaluation, markdown rendering, and still uses `expect`/`panic!` for CLI errors.
- `PlanCounters` has many structural counters but no phase timings or allocation snapshots.

**Required Work**
- Capture the current clean git commit before starting implementation work.
- Run the global gates once and save the command output or benchmark markdown path in the implementation notes.
- Record a focused benchmark baseline for at least `ledger`, `sailors`, `joinstress`, and `tpch` at scale 10000 with 3 repeats.
- Run a smell inventory using `rg` over Rust files for `unwrap(`, `expect(`, `panic!(`, `todo!(`, `unimplemented!(`, `unreachable!(`, and `dbg!(`.
- Split the smell inventory into production code, test code, benchmark code, and fuzz code.
- Identify all existing `#[allow(...)]` sites and classify them as remove, replace with `#[expect(..., reason = "...")]`, or keep temporarily with a follow-up.
- Record the current dependency footprint before adding `smallvec`, allocation profiler dependencies, or tracing/profiling output dependencies.

**Implementation Notes**
- Do not edit runtime behavior in this PRD except to add baseline documentation if needed.
- Do not add lint denies before the smell inventory exists, because the first failing Clippy run will otherwise hide the total cleanup scope.
- Do not use benchmark numbers from trace-enabled runs as the latency baseline.
- The benchmark baseline should include both text or markdown results and the exact command line.

**Passing Requirements**
- A baseline note exists in the implementation PR or commit message with current HEAD, gate commands, and benchmark command.
- The smell inventory identifies production panic/smell sites separately from tests.
- Existing `#[allow(...)]` sites are classified before lint policy changes begin.
- The next PRD can start with a known list of expected lint failures.

**Completion Artifact**
- `00_baseline_results.md`

**Stop Conditions**
- Stop if the worktree is dirty with unrelated changes that directly conflict with this hardening pass.
- Stop if baseline tests or current benchmark smoke fails before any hardening edits; diagnose that first.
- Stop if benchmark output is missing hash/trie/image counters added by the previous PRDs.
