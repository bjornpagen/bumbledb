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

## Results (2026-07-13, executed at eba32a5 + this change)

**The launcher.** `scripts/fuzz.sh` landed against cargo-fuzz 0.13.1's
verified flag surface (`cargo fuzz run --help` read first; libFuzzer
knobs pass through after `--`). **Fork mode works on this darwin host**:
the pinned nightly's libFuzzer ran `-fork=N -ignore_ooms=0
-ignore_crashes=0 -max_total_time=…` cleanly in every verification
session (jobs cycling, cumulative stats lines, clean exit 0) — the
`-jobs=12 -workers=12` fallback this file reserved was NOT needed. Two
lanes: the default builds `-s none` (throughput fuzzing — the oracles
are debug assertions and harness panics; cargo-fuzz's ASAN default
would tax every exec for coverage PRD 15 already assigns to its own
lane), and `--asan` builds `-s address`, with query carrying
`-rss_limit_mb=4096` there per PRD 15's disposition (the flag was
verified present in the ASAN session's binary invocation). Sessions on
a machine IN USE today were deliberately bounded (1 minute, 2–4
workers via the `FUZZ_WORKERS` env knob — the DEFAULT stays
`-fork=12`); the 10-minute-plus all-cores commissioning cadence is the
human register's, and the overnight firepower session already on the
books (`fuzz/SESSIONS.md`) is its precedent.

**Mode verification (all three, bounded, zero findings each):**

| mode | session | evidence |
| --- | --- | --- |
| single target | `FUZZ_WORKERS=4 fuzz.sh theory 1` | 54,142 execs (873/s), cov 1,419, corpus 298→310 post-cmin, logged |
| all five (default) | `FUZZ_WORKERS=2 fuzz.sh 1` | theory 53,577 (824/s) / ops 2,256 (34/s) / query 50 / rewrites 27,425 (421/s) / crash 2,246 (36/s); each cmin'd + logged; exit 0 |
| `--asan` | `FUZZ_WORKERS=2 fuzz.sh --asan query 1` | 73 execs, cov 20,608, `-rss_limit_mb=4096` in the invocation, clean |

**The dirty-artifacts refusal, proven:** a planted
`fuzz/artifacts/ops/planted-untriaged` made `fuzz.sh theory 1` refuse
with the file named and exit 1; removed, the session ran. The same
refusal is the trophy pipeline's enforcement arm.

**Artifact-shelf triage (the refusal's first real workout):** the shelf
held five files from the morning's sessions. Four crash `slow-unit`s
replay in 21–65 ms each on a quiet machine — child-spawn latency under
the overnight session's load, environmental. One query
`oom-d9bbe585…` is PRD 15's dispositioned ASAN-quarantine case
(replays clean: 10.5 s under ASAN defaults there, 1.31 s at `-s none`
here). All five dispositions recorded in `fuzz/SESSIONS.md`, artifacts
deleted; **no new trophies** (the ledger's two ops rows stand,
cross-referenced from `SESSIONS.md`).

**Corpus minimization (the one sanctioned corpus commit):**
`cargo fuzz cmin` per target over the ~12.6k accumulated files:

| target | before | after cmin | after the verification sessions (committed) |
| --- | --- | --- | --- |
| theory | 398 | 298 | 316 |
| ops | 3,379 | 2,416 | 2,393 |
| query | 3,329 | 2,441 | 2,418 |
| rewrites | 4,835 | 3,372 | 3,380 |
| crash | 615 | 420 | 441 |
| total | 12,556 | 8,947 | 8,948 |

ops/query/rewrites stay above the 2k-per-target line — recorded and
checked in anyway: corpus is coverage capital, and the replay lane
guards every entry. **Replay tests green post-cmin** (the critical
gate): `cargo test` in `fuzz/` — 4 replay suites + the crashpoint
sweep, 463 s wall locally, query's three-way replay dominating at
~400 s.

**CI spine** (`.github/workflows/ci.yml`, macos-latest arm64, cargo +
toolchain cached, YAML parse verified): check lane (`scripts/check.sh`,
fold-off matrix included) and corpus-replay lane (plain `cargo test` in
`fuzz/` — no libFuzzer in CI) per push; **Miri lane on the nightly
cron by measurement** — `scripts/miri.sh` is 752 s ≈ 12.5 min locally,
over the 10-minute per-push budget, the number recorded in the
workflow comment. Local walls: check lane 178 s (`scripts/check.sh`,
all gates green, the x86_64 cross check self-skipping as recorded),
replay lane 463 s test wall — per-push wall = max of the parallel
lanes, estimated under 15 minutes warm-cached (the first cold run
exceeds it once). Exclusions each commented in the workflow with the
reason: no benches, no asm gates (shared-runner timing/codegen is
noise — local measurement discipline), no long fuzz sessions and no
ASAN lane (firepower is the owner's machine; CI replays the corpus
deterministically). **First-run verification on a real push: pending**
— CI cannot be exercised from this machine; the `[test]` criterion for
the three lanes closes on the first push.
