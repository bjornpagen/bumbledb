# 30 — One battery

> **Decision.** "Green" is the exit code of **one committed script**,
> and every consumer — CI, the docs, the receipt, a human at the shell —
> invokes that script rather than re-spelling its contents. Benches
> measure and tests assert: no lane of the battery may fail because the
> machine was busy.

## The current representation

The battery is one fact written three ways, and the audit caught all
three disagreeing at once:

- **Prose** (`settlement/10-endgame.md` §E2) says the log crate runs
  `cargo nextest run` — edited to say so by a worker, mid-flight.
- **YAML** (`.github/workflows/bumbledb-log.yml`, uncommitted at audit
  time) installs nextest from a hardcoded `get.nexte.st/latest/linux-arm`
  artifact — the wrong binary on any non-arm runner — and runs a
  different command set than the prose.
- **The shell** has neither: nextest is not installed on the
  development machine, so the documented battery does not run where
  development happens. The config it depends on
  (`.config/nextest.toml`) was untracked — a battery setting that
  existed on one laptop.

And inside the battery sits a category error: `scripts/check.sh` runs
`bumbledb-bench`'s `tiny_end_to_end_measures_both_engines`, a
**wall-clock-measuring test**, in the correctness suite. Under the
audit's concurrent build load it failed; on a quiet machine it passes.
A gate whose verdict depends on machine load is not a gate — it is a
bench wearing a test's badge, and it teaches people to re-run reds.

## The target representation

### 1. The battery is a script, and everything else calls it

One committed entrypoint — `scripts/battery.sh` — is the definition of
green. Its body is the full set, in order, failing fast:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo nextest run --workspace` (one process pool over every unit and
   integration binary; after [10](10-one-workspace.md), `--workspace`
   includes the log crate, so the per-manifest special case is gone)
4. `scripts/check.sh` (the engine's own composite, minus what 1–3
   already ran — check.sh is *reduced*, not duplicated)
5. `scripts/lean.sh` — 0 disagreements
6. `scripts/spec-census.sh` — including the banned-token roster of
   [50](50-proof-as-gate.md)
7. `ts/`: `pnpm test`, `tsc --noEmit`, biome — via the package scripts
8. `ts-log/`: same trio, via the package scripts

CI's job body becomes an invocation of the script plus environment
setup; the endgame/receipt documents *reference* the script instead of
enumerating commands; a human types one thing. When the battery changes,
it changes in one file, and every consumer changes with it — the CI
cannot drift from the docs because neither of them owns the list.

### 2. Tools the battery needs are facts the repo carries

- `.config/nextest.toml` lives at the repo root, committed
  ([10](10-one-workspace.md)).
- The nextest install has **one way**: the battery script runs
  `cargo nextest --version || cargo install cargo-nextest --locked` —
  on CI and on laptops, the same line. No artifact URLs, no
  per-platform arms, nothing to detect: `cargo install` already knows
  the platform. "The documented command does not exist on this machine"
  stops being a discoverable state.

### 3. Benches measure; tests assert

`tiny_end_to_end_measures_both_engines` leaves the correctness battery.
The representational rule, stated once and applied everywhere: **a test
in the battery asserts a fact that is true on any machine at any load;
anything that reads a clock to decide pass/fail lives in the bench lane**
(`scripts/bench-night.sh`, `bench-out/`), where its output is a
measurement, not a verdict. Ruled, not optional: the battery keeps a
structure assertion at that site (both engines exercised end to end,
output well-formed — facts), and the duration comparison moves to the
bench lane whole. Asserting structure is a fact; asserting duration is
weather.

### 4. One entrypoint, and redundant scripts die

`scripts/battery.sh` is the only script CI or a document may invoke for
correctness. `check.sh` survives only as an internal helper the battery
calls for whatever lanes 1–3 do not already cover (the comment guard,
the census); every line of it that duplicates a battery lane is deleted,
and if that leaves nothing unique, `check.sh` is deleted whole. A CI
step or doc that names any correctness script other than
`scripts/battery.sh` is the drift this document exists to kill.

## What gets deleted

| Deleted | Because |
| --- | --- |
| the command enumerations in CI YAML and the endgame prose | both invoke `scripts/battery.sh`; the list has one writer |
| the hardcoded `linux-arm` nextest artifact | platform is detected; absence is a named refusal |
| wall-clock pass/fail from the correctness suite | benches measure, tests assert |
| the "run these eight lanes, and remember the log crate separately" litany | one command; the footnote died in [10](10-one-workspace.md) |

## The invariant

> **Green is the exit code of one script.** There is no second spelling
> of the battery to drift, no lane that exists only in CI or only in a
> doc, no tool the battery assumes but the repo does not carry, and no
> verdict that changes with machine load. A claim of green in any
> receipt names the script and its commit.

Dissolves: audit A's fake-green exposure, the nextest findings (missing
locally, wrong artifact in CI, untracked config), the bench flake, and
the three-spellings drift as a class. Depends on
[10](10-one-workspace.md); the census it runs gains its roster in
[50](50-proof-as-gate.md).
