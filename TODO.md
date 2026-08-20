# TODO — trued at the bugbash-perf campaign close (2026-08-03)

The 2026-07-25 bug-bash + perf campaign is **CLOSED** (44 findings, 44
fixed). The campaign lived on branch `bugbash-perf` (PR #14); main takes
it only as a completed state. Charts live in `assets/`; raw traces and
finding stamps live in git history.

## Owed — the campaign's own residue (data first, no intuition fixes)

- **The storage-lane regression cluster** — **closed**. Delete lane:
  audit/32 (`image_distincts` 7577 µs, insert-touch sibling 903 µs,
  `delete_b100` 0.89×). NOSYNC commit ladder: audit/33 / `crates/bumbledb-bench/NOSYNC-BASELINE.md`
  (NosyncLane pin: commit_b1 1.28×, b10 0.69×, b100 0.74×, b1000 1.15×;
  old 1.24–1.44 is not a number on this flag). Windowed-ephemeral
  1.07–1.17 is the same substrate change (sweep cells now
  `create_nosync`). Flamediffs from the close live in git history
  (`writes.durable.delete_b100.diff.svg`, `writes.nosync.commit_b100.diff.svg`).
- **r6_two_path_count 1.46** — **closed** (audit/34): fresh flame,
  8-sample p50 328 ms vs SQLite 694 ms (**0.47×**). `jp_descend_n1`
  exclusive 197.7 ms is the 2-path walk; `jp_force_n0` 1.25 µs. Finding
  044 (audit/37) skipped on that force number.
- **Heap-arm ladder** — **closed** as a lane (audit/39 /
  `crates/bumbledb-bench/HEAP-BASELINE.md`): heap get 167/292 ns
  (0.57× vs LMDB), admit prefixes 693→41432 facts at 1.26× ns/fact,
  A/I/R/F/J on every row.
- **Overlap constants re-pin**: `OVERLAP_CROSSOVER = 16`
  (`exec/run/overlap_leaf.rs`) and `FLAT_SWEEP_CEILING = 128`
  (`interval/overlap.rs`) are rig-pinned provisional — re-pin both from the
  `overlap_profile` sweep on a quiet machine, never by inspection (the
  finding-013 phase attribution now decomposes build vs walk vs residual,
  so the sweep finally has its signal).

## Open items (pre-existing, unchanged)

- **1.0.0 close** — owner-gated, explicitly deferred 2026-07-18. Owner
  ceremony only.
- **crashpoint + image-oracle** — deleted (owner kill: consumer-less
  test-support; `crashpoint!` sites remain as no-op atomicity names).
- **Audit-2026-07 deferred findings**: 014 closed-by-measurement
  (`audit/36-perf-leaf-batch.md`: o4 0.07×, 25_541/375_923 µs; 500k
  leaf descends at 24 ns, was 53–69 ns/tuple; `run_leaf_pinned_run`
  stays HANDOFF), 044 skipped (`audit/37-perf-telescoped-count.md`:
  `jp_force_n0` 1.25 µs), 009 step-2 closed (`audit/38-perf-minmax-fence.md`:
  Acc::Min/Max live under GroupState; o5 692/174954 µs, no jp_force).
  Finding 053
  (FilterPredicate interpreters) closed in
  [`proposals/exec-representation.md`](proposals/exec-representation.md).
  (The R5 ArgMax/ArgMin tail, including measure-keyed keys, is killed
  with the rest of Arg/CountDistinct.)
- **Feature-register triggers, recorded and waiting**: C19 balance laws
  (`Sum == Sum` per group — double-entry); temporal capacity (per-instant
  stabbing-set windows — mechanism sketch recorded beside the trigger:
  half-open boundary sweep, 1-D Helly, the overlap index as the judge's
  walk, polarity intact). Min/Max-window refusal trigger likewise.
- **Primer follow-through**: expect their P2.4 cutover questions; the
  expressibility test
  (`ts/test/expressibility-operand-views.test.ts`) is the living evidence
  to point at.

## Retired this campaign (previously owed here)

- C17 slot-vs-fetch: measured, slot landed, fetch arm + flag deleted
  (`484c3871`).
- The C17 write-time ray corner: RULED C20 (owner, 2026-08-03) — the
  write-time refusal is doctrine (`docs/design/capacity-laws.md` §8b C20),
  pinned by `capacity_duration_ray_under_an_absent_parent_still_refuses`.
- The capacity/windowed/lawful bench lanes + re-pins under the capacity
  spelling (`e511b540`), the calendar capacity twin world.
- The writes-ladder and churn wall-power reruns (the campaign manifest's
  PENDING rows) — landed at the baseline, re-run at the rebench; every
  README chart now regenerates from a local night run into `assets/`.
- C10 ray-Duration verdict parity engine-vs-naive: the refusal is a
  compared verdict on the differential wall (finding 018, `b0fa69f0`).
- The sweep-commit T8 lane (`0e7d1d42`) and the T8 mechanism (`128e4504`)
  — with the b100 caveat recorded under Owed above.

## Shipped (compressed ledger — detail in git history and the stamped docs)

- **0.15.0** (2026-08-19): admitted-instance / ABI-3 / format-8; Phase 2
  lanes D+E on main. Shared-machine night at `22e618d9` re-pins the README
  charts (durable read geomean 26.6×, scenarios 21.8×, compacted storage
  167 / 228 B/fact). Audit 35 remains owner quiet-machine.
- **bugbash-perf campaign** (2026-07-25 → 2026-08-03): instrument → gate →
  baseline → hunt → 44/44 findings fixed across seven file-disjoint lanes
  → rebench (reads 0.87–0.94 on five of six reps, scenarios 0.88 with the
  Hunt cluster reversed, storage lane honestly NOT CASHED). Detail in git
  history.
- **0.9.0** (2026-07-25): comparators, bool-order tail, pin injection,
  primer pins. CI green all lanes.
- **0.8.0** (2026-07-25): the capacity cutover whole — `<=[w]{lo..hi}`,
  format v7, fingerprint tag 4/label v5, corpus `judgment-capacity-*`,
  zero-trace gate green. Design stamped LANDED in
  `docs/design/capacity-laws.md`.
- **0.7.0** (2026-07-24): the audit campaign — 162 findings (158 fixed),
  22 rulings (21 implemented, R5 partial), GJ split, overlap join, point
  path, R16/R17/R18, wall-power re-bench, R21 re-pins. Detail in git
  history.
- Earlier: 0.6.0 destructure, 0.5.0 surface pair, cleanup-0.5.0 (PR #11),
  incremental images (PR #10) — see git history.
