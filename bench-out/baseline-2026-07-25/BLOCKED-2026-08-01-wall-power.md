# Bench-debt segment 1 — BLOCKED on wall power (2026-08-01)

The C17 measurement run was fully staged and then hard-blocked by the
protocol's wall-power gate: the machine left AC at ~17:11 and stayed on
battery through 18:10+ (watched continuously; `pmset -g batt` said
`Battery Power` the whole window). No timed window ran on battery — the
gate held.

## What is DONE (untimed, verified)

- **Both C17 arm binaries built and stashed**: `/tmp/c17/bench-fetch`
  (CAPACITY_WEIGHT_SLOT = false, the committed tree) and
  `/tmp/c17/bench-slot` (= true, the one-constant flip). Rebuild at any
  time: flip the constant in
  `crates/bumbledb/src/storage/commit/judgment.rs:108`,
  `cargo build --release -p bumbledb-bench`, stash, flip back.
  The working tree is restored to the committed fetch arm (git-clean).
- **Both arms oracle-gated GREEN** (the differential-vs-naive verdict
  gate plus the SQLite SUM-trigger parity, and the full
  `cargo test -p bumbledb capacity` battery under each arm):
  - fetch: 35 lib capacity tests + 13 bench capacity + 3 bench windowed
    (logs `/tmp/c17/test-fetch-bumbledb.log`, `test-fetch-bench.log`)
  - slot: same counts, same suites, green
    (logs `/tmp/c17/test-slot-bumbledb.log`, `test-slot-bench.log`)
- **Corpus verified + stamped**: `bench-data/fa73e680324f9b26`,
  `verify: OK — 2889 cases` (the bench stamp gate is satisfied; no
  `--i-am-lying` anywhere).
- **The lane driver is ready**: `run-capacity-debt.sh` beside this file
  — one lane per invocation, measure.sh mutex, BUMBLEDB_BENCH_BOOST=1,
  AC asserted before AND after every lane, digests pinned per lane.

## What REMAINS (blocked, in order)

1. `./run-capacity-debt.sh c17-fetch-durable`
2. `./run-capacity-debt.sh c17-slot-durable`
3. `./run-capacity-debt.sh c17-fetch-ephemeral`
4. `./run-capacity-debt.sh c17-slot-ephemeral`
5. Decide C17 on the power-budget rows (`commit_capacity_sum` −
   `commit_capacity_baseline`, durable AND ephemeral): LAND the winner —
   flip or keep the constant, DELETE the losing arm + the flag
   everywhere (judgment.rs, plan.rs `MarkEdgeOp::weight` derivation,
   applier.rs put, verify_store expected_slot_weight — zero traces),
   record both arms' numbers as the CONSTRAINT comment at the site;
   update docs (capacity-laws.md STATUS tail, capacity-cutover.md C17
   bullet, feature-register.md C17 entry, 50-storage.md § key layout).
   If the SLOT arm wins: report the ray-at-write-time corner for owner
   ruling (do NOT rule it) — a ray Duration weight under the slot arm
   refuses at write time, strictly stronger than C10's judge-time
   refusal, visible only for a ray child under an absent parent.
6. Rebuild `target/release/bumbledb-bench` on the landed arm, then
   `./run-capacity-debt.sh windowed-durable`, `windowed-ephemeral`,
   `lawful` — the windowed/lawful re-pins under the capacity spelling.
   The calendar capacity row (`commit_capacity_duration`, fresh twin
   world) rides every c17 cell; the winning arm's cells ARE its pins.
7. Commit artifacts per segment (`git add -f` the report/log/digest
   files only, never `scratch/`), push to `bugbash-perf`.

DNF/exclusion rule and protocol as the campaign-2026-07-23 layout.
