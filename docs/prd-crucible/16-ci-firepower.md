# PRD 16 — Full firepower: the all-cores launcher and the CI spine

**Depends on:** 11–15 (it orchestrates everything Phase D built). The
campaign's terminal PRD.
**Modules:** `scripts/fuzz.sh` (new), `scripts/miri.sh` (from 15,
wired), `.github/workflows/` (the CI spine), `fuzz/corpus/` management,
the trophy→regression pipeline in `crates/bumbledb/tests/`.
**Authority:** the owner's directive verbatim: fuzzing "running against
all my cores at full firepower." The machine is an M2 Max, 12 cores.
And the standing law that a trophy is only banked when it becomes a
permanent regression test — findings that live in a corpus directory
are not findings, they are homework.
**Representation move:** none. This PRD makes the infrastructure a
one-command habit — the difference between fuzzing that exists and
fuzzing that runs.

## Context (decided shape)

**`scripts/fuzz.sh`** — the local firepower launcher:
- No args: run ALL five targets, time-sliced, libFuzzer fork mode
  (`-fork=12 -ignore_ooms=0 -ignore_crashes=0`), 12 workers — the
  all-cores default IS the default, not a flag.
- `fuzz.sh <target> [minutes]`: one target, all cores, bounded session.
- `fuzz.sh --asan <target>`: the sanitizer lane from PRD 15.
- Corpus discipline built in: `cargo fuzz cmin` per target after each
  session (corpus minimization keeps the checked-in seed corpus lean);
  artifacts land in `fuzz/artifacts/<target>/` with the session's
  digest printed; the script REFUSES to run on a dirty `fuzz/artifacts`
  (un-triaged findings block new sessions — triage is not optional).
- Session summary on exit: execs/sec per target, corpus growth, new
  coverage edges (libFuzzer's own stats, parsed minimally), findings
  count. Honest zero: "0 findings in N executions over M minutes" is
  the recorded result of a session, appended to a `fuzz/SESSIONS.md`
  log line by the script.

**CI spine** (GitHub Actions, `macos-latest` arm64 runners):
1. **check lane:** the existing `scripts/check.sh` (fmt, clippy,
   engine tests, the `fold-off` matrix build from PRD 13) — the
   workspace gate, every push.
2. **corpus-replay lane:** NOT libFuzzer-in-CI (fork mode and long
   sessions don't belong in CI). Plain `#[test]`s in the fuzz crate
   (`fuzz/tests/replay.rs`) that iterate every checked-in corpus file
   through the target lib functions directly — every push replays the
   accumulated corpus deterministically in seconds. New corpus entries
   from local sessions get committed and are thereafter CI-guarded.
3. **Miri lane:** `scripts/miri.sh`, both interpretation targets —
   scheduled (nightly cron) rather than per-push if wall time demands;
   measure first, decide by the number, record it in the workflow file
   comment.
- CI does NOT run benches, does NOT run asm gates (aarch64 codegen
  gates on a shared runner are noise — they stay local per the
  measurement discipline), does NOT run long fuzz sessions. Each
  exclusion commented in the workflow file.

**The trophy pipeline:** every artifact that ever reproduced becomes a
named `#[test]` in the engine's test tree (minimized input inlined as
bytes, a comment naming the fuzz session and the defect), THEN the
artifact is deleted. `fuzz/SESSIONS.md` cross-references trophy tests.
The pipeline is documented in the fuzzing charter and enforced by the
dirty-artifacts refusal above.

## Technical direction

1. Write `fuzz.sh` against cargo-fuzz's actual flag surface (verify
   `-fork` behavior with the pinned nightly first; if fork mode
   misbehaves on darwin, fall back to `-jobs=12 -workers=12` and record
   the substitution here).
2. The replay tests import the fuzz crate's runner functions (the
   harness lib from PRD 11 pays off: targets are thin, runners are
   callable) — no libFuzzer linkage in CI.
3. Workflow YAML: three jobs as above; cache cargo + the pinned
   toolchain; total per-push wall target under 15 minutes (measure,
   record).
4. Run one real all-cores session per target locally (10+ minutes each)
   as the commissioning run; log them in `SESSIONS.md`; hand the
   overnight cadence to the human register.

## Passing criteria

- `[shape]` `scripts/fuzz.sh` runs all five targets fork-mode across 12
  workers by default; the dirty-artifacts refusal works (test it with a
  planted file); sessions append to `fuzz/SESSIONS.md`.
- `[test]` The corpus-replay lane green in CI on a real push; the
  Miri lane green on its schedule; the check lane green.
- `[shape]` The commissioning sessions logged: five entries in
  `SESSIONS.md` with execs/sec and outcomes; any trophies already
  converted to named regression tests with artifacts deleted.
- `[shape]` Every CI exclusion (benches, asm gates, long sessions)
  commented in the workflow file with its reason.
- `[gate]` Full workspace gates green — the campaign closes here.

## Doc amendments (rule 5)

The fuzzing charter's operations section: the launcher, the session
log, the trophy pipeline, the CI lanes and their deliberate exclusions.
README gains the one-line "fuzzed continuously" claim ONLY after the
commissioning sessions exist — the claim cites `SESSIONS.md`.
