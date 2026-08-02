# C17 slot-vs-fetch — the measured choice (2026-08-01)

The owed C17 measurement (capacity-laws.md §8b; deferred at the 0.8.0
close): both `measure_children` arms of the weighted-capacity judge ran
the power-budget lane under the same protocol, and the winner landed
engine-side with the loser and the `CAPACITY_WEIGHT_SLOT` flag deleted.

## Protocol

- Two binaries off the same tree differing in exactly the one constant:
  `fetch` (= false, the shipping baseline: empty `R` values, one child
  `F` get per walked edge) and `slot` (= true: a weighted statement's
  `R` edges carry the child's u64 weight LE, paid at write time).
- Each arm ORACLE-GATED before any timing: the full
  `cargo test -p bumbledb capacity` battery + the bench capacity (13)
  and windowed (3) differential/SQLite-parity suites green under each
  arm, and each binary earned its own `verify` stamp over the stamped
  corpus (`fa73e680324f9b26`, 2889 cases — the stamp hashes the binary
  fingerprint, so each arm re-proved the whole ledger+calendar
  differential before its timed window).
- Wall power (pmset AC) asserted before and after every lane; the
  measurement mutex (`scripts/measure.sh`) held; shared-machine boost
  on; the first attempt was refused for ~80 minutes while the machine
  sat on battery (no sample was taken on battery).
- Families: `commit_capacity_baseline` (statement-free control),
  `commit_capacity_sum` (u64 weight, dependent bound),
  `commit_capacity_duration` (Duration weight and ceiling, the fresh
  calendar twin world) — 8 warmups + 64 samples each, one legal commit
  per sample.
- Cells: {fetch, slot} × {durable, ephemeral}, plus two extra ephemeral
  reps per arm (min-of-3, the campaign convention). The ephemeral lane
  is the discriminator: the durable lane's per-commit fsync (~5 ms)
  swamps a ~15 µs judge and its clock proxy flags the fsync DVFS shadow
  (CONTAMINATED on every durable cell — the known write-lane condition,
  same as campaign-2026-07-23).

## Numbers (p50 µs; ephemeral = min of 3 reps)

| row | fetch | slot | judged surface (row − control) |
|---|---|---|---|
| commit_capacity_baseline (eph) | 18.2 | 18.2 | — (control, arm-blind: unit machinery only) |
| commit_capacity_sum (eph) | 35.2 | 32.3 | +17.0 → +14.1 (−17%) |
| commit_capacity_duration (eph) | 34.2 | 30.8 | +16.0 → +12.6 (−21%) |
| commit_capacity_sum (durable, 1 rep) | 5365.1 | 5093.4 | direction agrees, fsync-shadowed |
| commit_capacity_duration (durable, 1 rep) | 6007.1 | 5253.7 | direction agrees, fsync-shadowed |

Every rep individually agreed (slot sum 32.3/32.6/34.2 vs fetch
35.2/35.2/35.7; slot duration 30.8/30.8/31.8 vs fetch 34.2/34.5/34.5;
mins likewise: slot 25.3/22.2 vs fetch 26.6/25.4 µs).

## Verdict

**The slot arm wins on every weighted row in every rep.** Landed
2026-08-01: the value slot is the only form, the fetch arm and the flag
are deleted, the numbers above are the CONSTRAINT comment at the walk
(`crates/bumbledb/src/storage/commit/judgment.rs`).

**Owner ruling now owed (recorded, not ruled):** the slot arm's
write-time ray corner is live — a ray-valued Duration weight refuses at
WRITE time (the slot needs a finite u64), strictly stronger than C10's
judge-time refusal, visible only for a ray child under an absent
parent.

Cells: `fetch-durable/`, `slot-durable/`, `fetch-ephemeral[-r2,-r3]/`,
`slot-ephemeral[-r2,-r3]/` — each with `report.json`/`report.md`/
`run.log` (+ corpus digests where the driver pinned them).
