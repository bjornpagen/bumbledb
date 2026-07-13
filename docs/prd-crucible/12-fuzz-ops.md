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

## Conflict (policy 5) — the multi-violation citation ruling

Recorded 2026-07-13, surfaced by this target's first smoke at iteration
~1,360 (full analysis in the trophy ledger row and the finding note):
`30-dependencies.md` pins that errors cite the violated statement by
materialized-order id, but is SILENT on WHICH statement a commit
violating several at once must cite. The engine convicts per affected
tuple (`storage/commit/judgment.rs`, `plan.target_checks` order); the
naive model per statement id — one delta deleting an `Account` and a
`JournalEntry` both referenced by surviving postings yields two correct
rejections citing statements 13 and 12 respectively. Ruling (engine
untouched — pinning a citation order is the engine lane's decision if
it ever wants one): the citation among simultaneous violations is
UNPINNED; the oracle is exactly the pinned contract — verdicts agree,
and the engine's cited `(statement, direction)` must be a member of the
model's complete violation set (`NaiveDb::violations`, whose head is
`apply`'s verdict — one derivation), with the single-violation case
degenerating to strict equality. The strict comparator in
`differential::run` is unchanged (every existing lane constructs
single-violation deltas). Reactivation: if the engine lane pins a
deterministic citation, collapse the oracle back to equality; the
trophy input then pins that order.

## Results (2026-07-13, executed)

- **Generator:** `corpus_gen::opgen` — `random_scenario` over the
  querygen target theory: a drawn shrunken world streamed through
  `target::corpus_row`, a 1–3 query pool from `querygen::random_query`,
  then 6–24 steps over the ten-verb alphabet (weights in `step`); the
  closed/judgment write-case arm (`querygen::writes::closed_write_cases`)
  is reused whole inside batches. Runner in `fuzz/src/lib.rs::ops`
  (model-mapping table at its head); thin `fuzz_target!` in
  `fuzz_targets/ops.rs`.
- **Verbs:** all ten reachable, pinned by
  `opgen::tests::the_alphabet_reaches_all_ten_verbs`; determinism by
  `the_same_bytes_yield_the_same_scenario`.
- **Replay slot:** `fuzz/tests/replay.rs` replays `corpus/ops/` (six
  256-byte deterministic seed streams) and `trophies/ops/` on plain
  `cargo test`; one trophy is checked in (the citation ruling above).
- **Findings (two real ones, both from the first sessions):**
  1. *The multi-violation citation gap* (§ conflict above) at iteration
     ~1,360 of the first session — ruled, trophy pinned.
  2. *A generator hang*: `querygen::shapes_interval::random_mask`
     rejection-sampled the vacuous masks and never terminates on the
     entropy seam's constant zero tail (`Rng::Bytes` exhaustion) —
     diagnosed by sampling the wedged process (8+ min at 100% CPU on
     one input). Fixed loop-free by total repair in the bench
     generator (not engine code); pinned by
     `shapes_interval::tests::random_mask_is_total_on_constant_streams`.
     `contradict::contradiction_query` is the same latent class,
     unreachable from fuzzer bytes today — recorded in the ledger.
- **Per-iteration cost (M2 Max, plain release, no sanitizer):**
  14–16 ms/iter over the seed corpus (120- and 180-iteration sessions
  timed after warm-up, under concurrent fuzz load); an EVOLVED corpus
  runs richer scenarios — the fuzzer's own worst retained inputs
  ("slow units") replay at 59–79 ms plain. Above the low-single-digit
  budget line, and the overage is the ORACLE bill, not generation:
  every iteration is a fresh LMDB env (plus one per drawn reopen), a
  ~70–110-fact seed-world commit judged on both sides, `verify_store`
  after EVERY commit and reopen (the continuous-auditor oracle this
  PRD mandates), and full-contents scans after every
  rejection/rollback/reopen. Store dirs already use a
  tempdir-per-iteration under one shared parent. The `cargo fuzz`
  build multiplies this ~3–5x (`-Cdebug-assertions` — the engine's
  invariant sweeps — plus sancov trace-compares): observed ~46–62
  exec/s early-corpus with ASan, degrading toward ~4–7 exec/s
  single-worker as the corpus evolves into commit-heavy scenarios.
  The 50k smoke therefore runs fork-mode (`-fork=8`, the charter's
  firepower idiom); long sessions are the human register's, on all
  cores. Accepted as the price of five always-on oracles; if a future
  pass needs more throughput, the first lever is amortizing
  `verify_store` to commit-sampled instead of every-commit — a
  deliberate oracle weakening that must be recorded here if taken.

