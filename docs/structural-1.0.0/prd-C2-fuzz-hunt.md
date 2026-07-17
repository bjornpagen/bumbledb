# PRD-C2 — The all-cores fuzz hunt

Wave 2 · Repo: bumbledb · depends on: idle machine + owner go · after Wave 1 lands

## Objective

Run the dedicated multi-hour, all-cores fuzz hunt that Phase A-FUZZ set up but
deferred — the blazing storm on the idle machine, over the smarter generators
already landed, to drive the probability of reaching a deep engine bug as high as
the machine allows. Any finding is triaged per the fuzzing charter; a clean run is
a recorded result.

## Context

- The generators were already made structure-aware (the adversarial-in-accepted-
  shapes tier for `theory`/hostile-`irgen`, program-shaped hostile IR, the value
  dictionary, conformance-seeded corpora). `scripts/fuzz.sh`'s real default is
  `FUZZ_WORKERS=12`; the observed ~2-core sessions were the co-tenant habit plus
  fork-mode's serial-merge floor on short slices. The hunt must run LONG slices so
  merges amortize and all cores saturate.
- Sequencing (device honesty): this runs AFTER Wave 1's code has landed and any
  perf A/B (Wave 2 C1) is done — a 12-core fuzz storm swamps interleaved timing far
  past the ±2% band, so it never overlaps a measurement session.

## Work

1. **Saturation check.** Launch `scripts/fuzz.sh` with the REAL default (no
   `FUZZ_WORKERS` export) on one target; verify ~1200% CPU within the first minutes
   (`top -l 2`/`sample`). Record the observation.
2. **The hunt.** Run long-slice sessions across all five targets
   (`theory ops query rewrites crash`) — `FUZZ_MINUTES` set high enough that
   fork-mode's serial corpus-merge amortizes (≥30m/target; the multi-hour storm the
   owner asked for). All 12 workers. The launcher refuses dirty `fuzz/artifacts` —
   if it refuses, triage the existing artifact first.
3. **Findings discipline (the charter, `fuzz/README.md`).** ANY finding STOPS the
   session: minimize, root-cause, then EITHER a regression test + a trophy row in
   `fuzz/README.md` (engine-side bug) OR an environmental disposition in
   `fuzz/SESSIONS.md`'s dispositions section — and DELETE the artifact. A real
   engine bug is fixed engine-first through full gates (`check.sh` + `lean.sh`),
   pushed. Zero findings is a legitimate recorded result.
4. **The record.** The launcher appends `fuzz/SESSIONS.md` rows (date, target,
   lane, session, execs, execs/s, cov, corpus before→after, findings) — verify
   them, add the saturation note, commit + push. Honest zero-finding rows are the
   point.

## Technical direction

- Do NOT run during any timing session (measurement law) — the hunt owns the idle
  machine alone.
- Findings triage is the charter's, verbatim; do not weaken it for speed.
- If a finding is a genuine engine bug, its fix is a full landing-bar change
  (semantics move lean if touched), not a quick patch.

## Passing criteria

- Saturation verified (~1200% CPU on a long slice) and recorded.
- The hunt ran across all five targets at the long-slice budget; `fuzz/SESSIONS.md`
  carries the rows (findings or honest zeroes).
- Every finding resolved per the charter (trophy + regression test, or disposition)
  and its artifact deleted; any engine fix landed through full gates and pushed.
- Commit(s) in the repo's voice; push.
