# fuzz sessions — the campaign log

One line per session per target, appended by `scripts/fuzz.sh` (the
firepower launcher; crucible packet, git ecec1dc3). The honest
zero is the point: "0 findings in N executions over M minutes" is a
recorded result, not an absence. Real findings become trophy-ledger
rows in `fuzz/README.md` and named regression tests; environmental
artifacts get their disposition noted here and are deleted — the
launcher refuses to start while `fuzz/artifacts` holds anything.

Rows before 2026-07-13T18Z are reconstructed from the commissioning
smokes recorded in the PRD Results sections (11–15) and the overnight
firepower session; `cov` and exact wall figures were not all captured
then and are dashed where unknown.

## Triage dispositions (environmental — no trophies, artifacts deleted)

- **2026-07-13, query smoke (PRD 13):** three crash artifacts, all
  affected jobs failing in the same wall-second with identical
  `Lmdb(Io(EINVAL))` from `Db::prepare` under a concurrent
  `cargo test --workspace` compile storm; all replayed clean on a quiet
  machine. Environmental (tool-level resource pressure), deleted.
- **2026-07-13, rewrites smoke (PRD 13):** eight `slow-unit` artifacts —
  heavy join fan-out at Tiny scale, slow but correct. Not findings,
  deleted.
- **2026-07-12→13, overnight session:** eleven `Lmdb(Io(EINVAL))`
  artifacts, same environmental class as the query-smoke storm; all
  replayed clean, deleted.
- **2026-07-13, PRD 15 ASAN lane:** one query `oom-d9bbe585…` at
  libFuzzer's default `-rss_limit_mb=2048` — ASAN quarantine/metadata
  accounting across the largest seed corpus, NOT an engine leak (live
  heap at kill: 41 MB); the input replays clean alone (10.5 s under
  ASAN defaults; 1.31 s at `-s none`). Dispositioned in
  the crucible packet § Results (git ecec1dc3); the launcher
  carries `-rss_limit_mb=4096` for query's ASAN mode. Deleted.
- **2026-07-13, morning crash session:** four `slow-unit` artifacts;
  each replays in 21–65 ms on a quiet machine — child-spawn latency
  under the overnight session's machine load, not slow inputs.
  Environmental, deleted.
- **2026-07-15, query session (memory mode):** five `slow-unit`
  artifacts, all replaying to exit 0 (every oracle agrees). Root cause
  pinned by a mid-replay `sample` of the worst input (169 s): 100% of
  wall time inside the NAIVE oracle — `NaiveDb::query → rows_for →
  rule_bindings → enumerate` recursing per atom, the model's
  by-design nested-loop join fanning out at Tiny scale. The engine
  never appears in the stacks. Same slow-but-correct class as the
  2026-07-13 rewrites smoke disposition; oracle cost, not an engine
  finding. Deleted. The same input (`slow-unit-d5eadb07…`, kept in the
  corpus by cmin) re-tripped the slow threshold under the ASAN lane's
  tax the same day — same root cause, same disposition, deleted again.
- **2026-07-15, rewrites session (memory mode):** one
  `oom-2a21d1c6…` at libFuzzer's default `-rss_limit_mb=2048`
  (job peak_rss 2181 MB). The input replays clean standalone: exit 0,
  peak RSS 282 MB; twenty replays of the same input in ONE process
  still peak at 282 MB — zero per-iteration growth, no engine leak.
  The 2181 MB is fork-job-cumulative RSS (allocator high-water across
  a job's diverse heavy-join inputs plus libFuzzer corpus bookkeeping),
  the same process-level-accounting class as the PRD 15 ASAN oom
  disposition. Environmental, deleted.

## Trophies (cross-reference: the ledger in fuzz/README.md)

- 2026-07-13 `ops` — multi-violation citation tie: ruled (oracle 1
  accepts any citation from the model's complete violation set), pinned
  by `trophies/ops/multi-violation-citation-order` +
  `naive/tests/judgment.rs::citation_set`.
- 2026-07-13 `ops` — `shapes_interval::random_mask` rejection-sampling
  hang on constant entropy tails (generator, not engine): fixed by
  total repair, pinned by
  `shapes_interval::tests::random_mask_is_total_on_constant_streams`.

## Sessions

| date | target | lane | session | execs | execs/s | cov | corpus (post-cmin) | findings |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-07-13 | theory | none | 100k-run smoke (PRD 11) | 100,000 | — | — | seed | 0 |
| 2026-07-13 | theory | none | 20k-run follow-up smoke | 20,000 | — | — | — | 0 |
| 2026-07-13 | ops | none | first smoke, stopped at the finding (PRD 12) | ~1,360 | — | — | — | 1 (multi-violation citation → ruled + trophy) |
| 2026-07-13 | ops | none | second smoke, stopped at the generator hang | — | — | — | — | 1 (random_mask rejection-sampling hang → fixed + pinned) |
| 2026-07-13 | ops | none | 50k-run smoke, post-fixes (PRD 12) | 50,492 | — | — | — | 0 |
| 2026-07-13 | query | none | capped smoke, 8 jobs (PRD 13) | 15,671 | — | — | — | 0 (3 environmental EINVAL, triaged above) |
| 2026-07-13 | rewrites | none | 50k smoke, 8 jobs, ~475 s (PRD 13) | 50,024 | ~105/s | — | — | 0 (8 slow-units, triaged above) |
| 2026-07-13 | crash | none | 10k smoke, -fork=2, 1,393 s (PRD 14) | 10,213 | ~9/s | — | — | 0 |
| 2026-07-13 | theory | address | ASAN lane, -runs=1000 (PRD 15) | 1,000 | — | — | — | 0 |
| 2026-07-13 | ops | address | ASAN lane, full-corpus replay (PRD 15) | 3,381 | — | — | — | 0 |
| 2026-07-13 | query | address | ASAN lane, rss_limit_mb=4096 (PRD 15) | 4,738 | — | — | — | 0 (1 oom at default rss limit, triaged above) |
| 2026-07-13 | rewrites | address | ASAN lane, full-corpus replay (PRD 15) | 4,858 | — | — | — | 0 |
| 2026-07-13 | crash | address | ASAN lane, -runs=1000 (PRD 15) | 1,000 | — | — | — | 0 |
| 2026-07-13 | theory | none | overnight firepower session | ~198,000 | — | — | 398 pre-cmin | 0 |
| 2026-07-13 | ops | none | overnight firepower session | ~108,000 | — | — | 3,379 pre-cmin | 0 (EINVAL class, triaged above) |
| 2026-07-13 | query | none | overnight firepower session | ~81,000 | — | — | 3,329 pre-cmin | 0 (EINVAL class, triaged above) |
| 2026-07-13 | rewrites | none | overnight firepower session | ~776,000 | — | — | 4,835 pre-cmin | 0 (EINVAL class, triaged above) |
| 2026-07-13 | crash | none | morning session | — | — | — | 615 pre-cmin | 0 (4 slow-units, triaged above) |
| 2026-07-13 | all five | none | PRD 16 corpus minimization (`cargo fuzz cmin`, the sanctioned corpus commit) | — | — | — | theory 398→298, ops 3,379→2,416, query 3,329→2,441, rewrites 4,835→3,372, crash 615→420 | 0 |
| 2026-07-13 | theory | none | 1m x 4 workers | 54142 | 873/s | 1419 | 298 -> 310 | 0 |
| 2026-07-13 | theory | none | 1m x 2 workers | 53577 | 824/s | 1420 | 310 -> 316 | 0 |
| 2026-07-13 | ops | none | 1m x 2 workers | 2256 | 34/s | 11971 | 2416 -> 2393 | 0 |
| 2026-07-13 | query | none | 1m x 2 workers | 50 | 0/s | 13072 | 2441 -> 2430 | 0 |
| 2026-07-13 | rewrites | none | 1m x 2 workers | 27425 | 421/s | 11964 | 3372 -> 3380 | 0 |
| 2026-07-13 | crash | none | 1m x 2 workers | 2246 | 36/s | 2898 | 420 -> 441 | 0 |
| 2026-07-13 | query | address | 1m x 2 workers | 73 | 0/s | 20608 | 2430 -> 2418 | 0 |
| 2026-07-15 | theory | none | 12m x 8 workers | 597806 | 752/s | 1978 | 316 -> 406 | 0 |
| 2026-07-15 | ops | none | 12m x 8 workers | 589543 | 813/s | 12803 | 2393 -> 3508 | 0 |
| 2026-07-15 | query | none | 12m x 8 workers | 60073 | 76/s | 13493 | 2418 -> 2708 | 5 |
| 2026-07-15 | rewrites | none | 12m x 8 workers | 1038630 | 1861/s | 12323 | 3380 -> 3563 | 1 |
| 2026-07-15 | crash | none | 12m x 8 workers | 310918 | 430/s | 3151 | 441 -> 614 | 0 |
| 2026-07-15 | query | address | 10m x 8 workers | 5614 | 6/s | 20897 | 2708 -> 2725 | 1 |

## Kill sessions (the WRITEMAP commit-window sweep, `fuzz/tests/kill.rs`)

One row per statistical lane (>= 2,000 random-timing SIGKILLs per
store kind; the four-point corpse invariant per round — reopen,
`verify_store` green, all-or-nothing batch prefix, working
post-recovery commit). The in-window column is the calibrated
estimate: `mdb_txn_commit`'s interior (sync surplus) on the durable
kind, the whole write call on the ephemeral one (the torn-meta sliver
is its µs-scale subset).

| date | kind | rounds | window | in-window est | surviving batches | pre-first-commit kills | violations | seed |
|---|---|---|---|---|---|---|---|---|
| 2026-07-16 | durable | 2,000 | 20 ms (~4 periods) | ~632 in-commit | 51,257 (25.6 mean) | 37 | 0 | 1784215061224509000 |
| 2026-07-16 | ephemeral | 2,000 | 20 ms (~4 periods) | ~2,000 whole-call | 68,245 (34.1 mean) | 24 | 0 | 1784215132296546000 |
