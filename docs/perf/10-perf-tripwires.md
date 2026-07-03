# PRD 10 — Perf tripwires and the final reconciliation

Authority: everything above; `docs/architecture/README.md` rules 3/5 (docs stay
true); the suite README rule 3 (no wall-clock assertions, ever).

## Purpose

Encode every fix as a **structural regression test** — trace-event counts,
counter work bounds, allocation counts, size ratios — so the five findings can
never silently return, then reconcile the architecture docs with the engine
that now exists. After this PRD the humans take over: `scripts/bench.sh` at S,
then the L-scale ALL-WIN attempt.

## Technical direction

- **One tripwire module** (`crates/bumbledb-bench/src/tripwires.rs` or an
  engine `tests/perf_tripwires.rs` — choose by what each assertion needs;
  family-shaped assertions belong in the bench crate where `families` lives).
  All obs-dependent tests run in check.sh's obs lane. Over the pinned S corpus
  (or hand-built stores where S is overkill), pin per finding:
  1. **Access path (PRDs 00–03):** for each read family whose predicates are
     all selections (point via guard, string, fk_walk, balance, stats, skew):
     a full 2-cycle param rotation after warmup emits **zero** `view_build`
     events and only `view_memo_hit`/`select_probe`; for chain and range
     (residual ranges), a 4-window rotation emits ≤ 4 `view_build`s total
     (the LRU pin). `profile()` per family asserts node `entries` bounds
     computed from the corpus's known selectivities (write the expected
     numbers as named constants with derivations in comments — e.g.
     `STRING_SELECTED ≈ postings / MEMO_VOCAB ± the uniq share`).
  2. **Finalize (PRD 04):** fk_walk's traced warm sample emits
     `dict_resolve` ≤ distinct names in the result (== 1 for a single-account
     param); skew's hot param emits ≤ 2.
  3. **Iteration/cover (PRDs 05–06):** the balance shape's light-holder
     profile pins the chosen cover (Account) and
     `batch_entries ≤ 2 × bindings`; a stats profile pins the instrument
     node's `entries == 512` (dense iteration visits keys, not capacity —
     indirectly visible as batching counts).
  4. **Fairness (PRD 08):** already asserted inside `FairnessCheck`; the
     tripwire is that `verify`'s loader and `open_for_bench` both route
     through `configure_sqlite` — assert by construction (grep-level review),
     not test.
  5. **Store (PRD 09):** covered by its own compaction-ratio test; no
     duplicate here.
- **Re-pins, deliberately:** the est/actual family pins from PRD 07 and the
  report markdown golden from PRD 09 are already in; this PRD re-pins anything
  the intervening work moved (exec-digest `covers` strings in bench tests,
  trace-name lists) and re-runs the full gate suite as the final integration
  proof. Every re-pin's commit message names the PRD that moved it.
- **Doc reconciliation, same change:**
  - `30-execution.md`: selection levels (the probe-not-scan rule, the
    once-per-generation force amortization), the K=4 view-memo LRU and its
    memory bound, the magnitude-first cover rule, dense map iteration and the
    growth-sizing bound, the finalize intern memo and its buffer-dedup
    consequence. Each as a decision with alternatives-and-reversal notes in
    the house style.
  - `40-storage.md`: already amended by PRD 09; verify it reads true against
    the final code.
  - `50-validation.md`: status ledger entry — the five findings, fixed, with
    the tripwire module named as the enforcement; the fairness rule (PRD 08)
    in the oracle protocol; the note that the performance claim remains
    **pending a human re-run** (this suite changed the engine, so the old
    FAIL report is stale evidence, nothing more).
  - `docs/perf/README.md`: flip the status table to "landed," pointing at the
    tripwire module.
- **Dead-code sweep:** grep for the transitional names this suite created and
  retired (`selections_as_filters`, any `PRD 0x` TODO comments) — none may
  survive. `#[allow(dead_code)]` remains banned; if the cutover orphaned
  anything, delete it here.

## Non-goals

Timing assertions (rule 3 — the human's `scripts/bench.sh` run is the only
clock). New engine capability. CI wiring. Running the L-scale claim.

## Passing criteria

- The tripwire module exists, every assertion above implemented, green in both
  feature configs, and wired into `scripts/check.sh`'s obs lane (extend the
  filter list the way PRD 17 of the bench suite did).
- Grep-clean: no transitional shims, no stale `docs/perf` TODOs in code.
- All four doc amendments landed; `docs/architecture/README.md`'s rules pass a
  self-review against the diff (mechanisms name readers; no doc contradicts
  the code).
- `scripts/check.sh` green end to end — fmt, clippy both configs, full tests
  (including verify-S and the alloc gate in release), doctests, the obs lane,
  and the cross-target check.
- The suite is complete: nothing in `docs/perf/` remains unimplemented, and
  the final commit message says exactly that, handing the next step — the
  re-run and the claim — to its human owner.
