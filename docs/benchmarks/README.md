# The benchmark & observability suite — build plan

The ordered PRD set that takes the repo from "engine built, no numbers" to "fully
observable, oracle-verified, benchmarked against SQLite, with traces" — the remaining
process, documented once and for all. `docs/architecture/` remains the sole design
authority; where a PRD and an architecture doc disagree, one of them is wrong and must
be amended in the same change (README rule 5 of the architecture suite).

## Rules (binding on every PRD)

1. **Order is execution order.** A PRD may begin only when every earlier PRD's passing
   criteria hold. No PRD is checked off with unmet criteria — if one cannot be met,
   stop and escalate to the owner.
2. **Passing criteria are strict.** Unit tests named in a PRD are part of that PRD.
   The global commands must be green after every PRD:
   `scripts/check.sh` (fmt --check, clippy -D warnings across the workspace including
   the bench crate, workspace tests, doctests, the release-mode allocation gate).
3. **No smoke-test or end-to-end-test PRDs exist in this suite** — that work is
   human-owned, deliberately. PRDs specify code and its unit tests only. Likewise
   **no migrations, ever** — humans own all data movement.
4. **Dependency quarantine.** The engine crates (`bumbledb`, `bumbledb-macros`) keep
   exactly `heed + blake3`. The bench crate (`bumbledb-bench`) may depend on
   `rusqlite` (bundled) and **nothing else** — no clap, no serde, no criterion, no
   rand. Argument parsing, JSON emission, statistics, and randomness are hand-rolled
   (the house LCG). Any new dependency anywhere is an owner decision, not a PRD.
5. **Observability is zero-cost when off.** Every instrumentation feature (`trace`,
   the extended `alloc-counter`) compiles to nothing under default features: no
   branches, no atomics, no fields on hot paths. The release-mode allocation gate is
   the proof and must stay green after every PRD. No `dyn` in hot paths, no
   always-on counters in release execution paths (`00-product.md` doctrine).
6. **No `#[allow(dead_code)]`, no mechanism without a reader** — same discipline as
   the engine build. Instrumentation points name the consumer that reads them.
7. **Determinism everywhere.** Every generator (data, queries, params) is seeded LCG;
   identical config ⇒ identical bytes. Timing numbers are the only nondeterminism in
   the whole suite.
8. **Verify before time.** No timing path may run without a fresh verification stamp
   (PRD 12). The tool enforces the ordering of `00-product.md` success criterion 1
   (exactness before any timing claim) mechanically.

## The set

Engine-side observability:

- 00 — Bench crate skeleton and the dependency-quarantine ruling
- 01 — Allocation observability v2: bytes, live, peak
- 02 — Trace core: spans, events, capture, the `trace` feature
- 03 — Read-path instrumentation
- 04 — Write-path and image instrumentation
- 05 — ExecutionStats and cache/storage statistics surfaces

The bench crate:

- 06 — The ledger schema
- 07 — The deterministic corpus generator
- 08 — Corpus loading: bumbledb and SQLite
- 09 — The IR→SQL translator
- 10 — Canonical results and multiset comparison
- 11 — The randomized query generator
- 12 — `verify`: the oracle command and the stamp
- 13 — Harness core: protocol, percentiles, windows, cold
- 14 — The read query families
- 15 — Write and cold benchmarks
- 16 — The SQLite runner and the fairness contract
- 17 — Trace export: Chrome Trace Format and the flame summary
- 18 — The report: gates, budgets, artifacts
- 19 — CLI assembly and the final doc reconciliation

Humans own after 19: running `verify` + `bench` at the L scale on the canonical
machine, reading the report, making (or refusing) the performance claim, and every
end-to-end/e2e/smoke concern.

## Fairness addendum (docs/perf/08)

Under `synchronous=FULL`, both engines pay flush-to-media: the SQLite session
pins `fullfsync=ON` / `checkpoint_fullfsync=ON` because LMDB's macOS commits
issue `F_FULLFSYNC` unconditionally while SQLite's default lies to the drive
cache. `FairnessCheck` enforces it; write numbers predating this rule
under-priced SQLite's commits ~40×.

## How to run

The suite is built; three commands (or `scripts/bench.sh`, which runs the last two):

```sh
cargo run -p bumbledb-bench --release -- gen    --scale S --dir bench-data
cargo run -p bumbledb-bench --release -- verify --scale S --dir bench-data
cargo run -p bumbledb-bench --release -- bench  --scale S --dir bench-data
```

S is the human smoke scale; **the claim runs at L** (`--scale L`, same three
commands, canonical machine) and needs the report's gate verdict to read ALL-WIN
with the p99 budget passing. `bench` refuses without a fresh `verify` stamp for
the exact corpus; traces come from `bench --trace` or the quick-look
`trace --family NAME`; allocation windows need the obs build
(`--features obs`). Artifacts land in `bench-out/<timestamp>/` — publishing a
report into the repo is a deliberate human copy, never the tool's doing.
