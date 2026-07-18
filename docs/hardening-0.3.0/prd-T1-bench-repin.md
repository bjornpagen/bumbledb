# PRD-T1 — Bench repin: one rev, matching artifacts, honest numbers

Wave T · Repo: bumbledb · depends on: **idle machine + owner go** · exclusive

## Objective

Repair the torn bench pin. The 2026-07-18 truing committed a new
`bench-out/run1/report.md` (rev `5f531e17`) but left `run1/report.json` — the
file `scripts/bench_viz.py` actually consumes — at the 2026-07-16 run (rev
`adac4010`), same as run2/run3 and eph1–3. The published 21×/20.9× therefore
mixes two engine revs ("min-of-3" across two binaries), which the measurement
law calls void, and the committed tree cannot reproduce the charts. Recomputing
the README's own recipe over the committed JSONs yields 19.4× durable / 18.1×
ephemeral. Re-run everything on ONE rev and commit a self-consistent pin.

## Context (verified)

- `bench-out/` is gitignored; artifacts are force-added deliberately. The torn
  state exists because a committer staged only `run1/report.md` + the SVGs.
- eph1 (2026-07-16) has five contaminated READ blocks (`range`,
  `entries_for_account_set`, `busy_scan`, `meets_chain`, `disp_stream`) — the
  README's exclusion rule voids those percentiles.
- `meets_chain` p99 (ours ≈1.35–1.42 ms vs SQLite ≈135–146 µs) falsifies the
  README's universal "tails sit an order of magnitude inside SQLite's".
- The bench binary refuses to time without a fresh per-binary verify stamp.

## Work

1. Machine idle (measurement law). Release build at current `origin/main`
   HEAD; record the rev. `bumbledb-bench gen` (digest cache fine), then
   `bumbledb-bench verify` — the full 2,862-case two-oracle gate. A red oracle
   is a stop-the-line correctness regression: STOP and report; do not proceed.
2. Through `scripts/measure.sh`, per the README's reproduction recipe: 3
   durable `bench` runs + 3 ephemeral runs + `scenarios`. Fresh data per rep,
   min-of-3 across processes. If the clock proxy flags a READ block
   contaminated after retry, re-run that whole run — do not ship a durable
   number the exclusion rule voids; write-lane contamination is excluded and
   counted per protocol, as today.
3. Force-add, for EVERY run dir (`run1 run2 run3 eph1 eph2 eph3 scen`): BOTH
   `report.json` and `report.md` (and `scenarios.md`), all from the same rev.
   Delete stale leftovers from the old runs in those dirs.
4. `scripts/bench_viz.py` over run1+run2+run3 `--scenarios .../scenarios.md` →
   regenerate all five `assets/*.svg`.
5. Re-true `README.md`: the read-family geomean (currently `21×`), the
   ephemeral geomean (`20.9×`), the scenario geomean (`17×`), and every other
   numeric claim, computed FROM THE COMMITTED JSONs by the README's own recipe.
   Rewrite the tails sentence to name the `meets_chain` p99 exception (or scope
   the claim to the families where it holds) — whichever the fresh numbers
   support; no universal claim that one committed row falsifies.
6. ALL-WIN check from the committed JSONs: every gated read family beats SQLite
   p50. A LOSS is a stop-the-line regression: report, do not chart around it.
7. Commit in the repo's voice with the numbers + machine conditions in the
   body; push.

## Passing criteria

- Every committed `bench-out/**/report.{json,md}` carries the SAME engine rev,
  equal to the HEAD the run built (grep the rev field in every file; zero
  mismatches).
- `scripts/bench_viz.py` re-run over the committed inputs reproduces the five
  committed SVGs (data-identical; byte-identical if the script is
  deterministic).
- Every numeric claim in `README.md` §benchmarks is derivable from the
  committed JSONs by the stated recipe, to the rounding printed. Specifically
  the three geomeans match recomputation, and no sentence is falsified by any
  committed row (the `meets_chain` tails fix included).
- The verify stamp exists for the exact benched binary (2,862 green — the
  bench's own refusal mechanism is the proof).
- No contaminated read block backs any published percentile.
- `git status` shows no unstaged bench artifacts left behind; commit pushed.
