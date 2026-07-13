# PRD 12 — fuzz target: ops (the flagship lifecycle interleaver)

**Depends on:** 11 (the fuzz crate + harness), 10 (Tiny scale).
**Modules:** `fuzz/fuzz_targets/ops.rs`, the ops runner in
`fuzz/src/lib.rs`, the op-sequence arm of the generator in
`bumbledb-bench` (extend the existing opgen with the missing lifecycle
verbs if any: reopen, re-prepare, judgment-rejection continuation).
**Authority:** the Turso/Quint lesson translated to our stack: the bugs
worth the most are LIFECYCLE bugs — sequences of writes, deletes,
constraint judgments, reopens, and re-prepares that no single-shot
differential case ever exercises. This is the flagship target; if only
one target runs overnight on all cores, it is this one.
**Representation move:** none in the engine. The runner reifies "a
legal interaction with a bumbledb store" as a generated op sequence with
a parallel naive model — the two-oracle discipline extended over TIME.

## Context (decided shape)

- **The op alphabet** (drawn per-step from fuzzer bytes at Tiny scale):
  insert batch, delete batch, mixed batch, commit (invoking dependency
  judgment), rollback/abandon, prepared-query execution (against live
  params from earlier draws), re-prepare, view read, `reopen` (drop the
  env, reopen from disk), and `verify_store`. Sequence length bounded by
  Tiny.
- **The model:** the existing independent naive model (the second
  oracle) run in lockstep — it applies the same logical ops to its own
  state, including judging FDs/INDs naively at each commit.
- **Oracles per step:**
  1. **Verdict parity:** commit accept/reject matches the naive
     judgment (same verdict; on reject, same statement class cited via
     `StatementRef`).
  2. **Query parity:** every executed query's result set equals the
     model's (set-semantic compare, the differential's comparator).
  3. **Reopen equivalence:** after any reopen, all relations' full
     contents equal the model's state (the reopen changed nothing).
  4. **`verify_store` green** after every commit and every reopen —
     the store's own internal auditor agrees continuously, not just at
     test end.
  5. **Rejected commits change nothing:** after a judged rejection,
     re-read equals the model's pre-commit state.
- **Determinism:** the whole run derives from the byte string; the
  harness prints the seed-bytes digest on failure so any finding replays
  exactly (`cargo fuzz run ops <artifact>`).

## Technical direction

1. Audit the existing opgen: it already generates write/delete/query
   interleavings for the differential — extend, don't duplicate. The
   new verbs (reopen, re-prepare, rejection-continuation) join its
   alphabet behind the same `Rng` seam.
2. The model must already support everything the alphabet does; where
   it lacks a verb (reopen is a no-op for it; rejection-continuation
   means "don't apply"), the mapping is stated in a comment table at
   the runner's head.
3. Keep per-iteration cost honest: budget an iteration at Tiny scale to
   low single-digit milliseconds; if LMDB env setup dominates, use a
   tempdir-per-iteration but a shared parent to keep the OS happy —
   measure, note the per-iter cost in this file.
4. Smoke: 50k runs finding-free locally (or trophies fixed + recorded),
   then a longer session goes to the human register (the overnight
   all-cores run is the human's — PRD 16 builds the launcher).

## Passing criteria

- `[shape]` The op alphabet covers all ten verbs (grep the runner's
  drawing match); the model-mapping comment table present.
- `[test]` 50k-run smoke finding-free (or trophies fixed and recorded
  in the README ledger).
- `[test]` A checked-in regression test replays any trophy artifacts
  found during development (empty is fine going in; the slot exists).
- `[shape]` Per-iteration cost measured and recorded in this file.
- `[gate]` Workspace gates unaffected.

## Doc amendments (rule 5)

One line appended to the fuzzing charter section: the ops target's
alphabet and its five oracles.
