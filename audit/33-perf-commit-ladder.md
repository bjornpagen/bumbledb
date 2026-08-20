# 33 — The NOSYNC commit ladder 1.24–1.44: re-baseline on the bench flag, then attribute

- **Status:** **fixed this pass** — `StoreMode::Nosync` is the crate-private
  NosyncLane flag (`Db::create_nosync` / `Db::open_nosync`); `--nosync`
  (`--ephemeral` alias) opens the stamped durable-shaped corpus. New pin
  below. Tests: `bench_parses_every_knob` (`--nosync`),
  `the_durability_axis_has_exactly_two_points`.
- **Severity:** performance debt.

## The situation

TODO.md records the NOSYNC commit ladder at 1.24–1.44 versus the campaign
baseline. The purge (20) deletes the ephemeral store kind; the lane
re-anchors on the bench-private NOSYNC open flag over a durable-shaped
store. The old numbers are not comparable across that substrate change.

## Protocol

1. Land 20; wire the ladder onto the `NosyncLane` flag.
2. Re-pin the baseline on the new substrate (fresh reps, same protocol).
3. If the regression survives the re-pin, attribute via the traced twin
   from 32's tooling (same applier/judgment suspects) before any fix.

## The fix

`StoreMode::Ephemeral` / `Db::ephemeral` die. The flag is
`StoreMode::Nosync`: create via `Db::create_nosync`, open via
`Db::open_nosync`. `bench --nosync` (and `--ephemeral`) attaches the
stamped corpus — no reload/compact twin. `DurabilityLane::Nosync` and
the churn `nosync` row ride the same arm. `bench_viz.py` treats
`config.store == "nosync"` as the former ephemeral pool.

## New pin (2026-08-20, Apple M2 Max, obs release, scale S, seed 1, 8 samples)

`writes --lanes nosync --batches 1,10,100,1000 --samples 8`:

| family | ours p50 ns | sqlite p50 ns | ours/sqlite |
| --- | ---: | ---: | ---: |
| `commit_b1` | 47_542 | 37_125 | **1.28×** |
| `commit_b10` | 218_542 | 317_042 | **0.69×** |
| `commit_b100` | 1_433_875 | 1_940_750 | **0.74×** |
| `commit_b1000` | 9_345_041 | 8_105_709 | **1.15×** |
| `delete_b1` | 45_708 | 28_583 | 1.60× |
| `delete_b10` | 175_000 | 343_667 | 0.51× |
| `delete_b100` | 1_425_958 | 2_494_417 | 0.57× |
| `delete_b1000` | 9_338_959 | 9_764_833 | 0.96× |
| `insert_stream` | 688_915_791 | 480_292_042 | 1.43× |

The campaign 1.24–1.44 cluster does not survive the substrate change.
`commit_b1` is 1.28× (single-row commit tax); `commit_b10`/`commit_b100`
win; `commit_b1000` is 1.15×. No ranked engine fix — the old ratio is
not a number on this flag. Windowed-ephemeral 1.07–1.17 is the same
substrate change (sweep cells now `create_nosync`).

32's `delete_b100` traced twin (2 samples, same machine): `apply_deletes`
750.7 µs / `lmdb_commit` 731.9 µs / `judgment_source` 0.3 µs — the
campaign T8 suspect does not rank on the new pin either.

## Acceptance

- The ladder runs on the flag; a new pinned baseline is recorded in the
  bench docs; the TODO.md row is closed against the new pin or carries the
  trace that keeps it open.
