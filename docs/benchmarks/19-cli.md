# PRD 19 — CLI assembly and the final doc reconciliation

Authority: everything above; `docs/benchmarks/README.md` rule 8 (verify before
time); the architecture docs to reconcile.

## Purpose

Wire every capability into the `bumbledb-bench` binary with a hand-rolled CLI,
enforce verify-before-time mechanically, and reconcile the architecture docs with
the now-existing suite. After this PRD the humans take over: running it, reading
it, claiming or refusing.

## Technical direction

- Subcommands (hand-rolled parser from PRD 00, extended):
  - `gen --scale S|M|L --seed N --dir PATH` — generate + load both stores into a
    **digest-keyed** directory (`PATH/<corpus-digest-hex-prefix>/`): reuse if the
    digest directory exists and carries a `corpus.ok` marker (regeneration is
    identity; the cache is convenience for L).
  - `verify --scale --seed --dir --cases N` — PRD 12 against the digest
    directory; writes the stamp inside it.
  - `bench --scale --seed --dir [--families a,b,c] [--samples N] [--trace]
    [--out PATH]` — **refuses without a matching fresh stamp** (message names
    `verify` as the remedy; `--i-am-lying` overrides and stamps the report
    provenance with `UNVERIFIED`, spelled exactly that way in red-caps in the
    markdown). Runs read families (both engines), write families, cold; with
    `--trace`, the PRD 17 captures; emits PRD 18 artifacts to `--out`
    (default `bench-out/<timestamp>/`).
  - `trace --scale --seed --dir --family NAME` — one traced warm+cold pair for
    one family, artifacts only (the quick-look tool).
  - `queries` — print `QUERIES.md` to stdout.
  - `help`, unknown-flag errors naming the flag.
- `--families` filtering never bypasses the gate semantics: a filtered run's
  report marks the overall verdict `PARTIAL` (never ALL-WIN).
- `scripts/bench.sh`: `verify` (S default; `BENCH_SCALE=L` env override) then
  `bench` with the same config — the two-step ritual as one command for humans.
- Alloc-window runs require the `obs` feature build; the CLI detects (cfg!) and
  says exactly which cargo invocation to use when asked for something the build
  lacks (`cargo run -p bumbledb-bench --features obs --release -- …`).
- **Doc reconciliation, same change:** `50-validation.md` status ledger updates
  (oracle: built as 2-way + goldens — record the reference-engine deviation and
  its rationale; benchmark: built, claim pending a human L-scale ALL-WIN run);
  `00-product.md` criteria 1–2 point at `bumbledb-bench verify` / `bench` as the
  mechanisms; `docs/benchmarks/README.md` gains a final "how to run" section
  (the exact three commands, S for smoke-by-human, L for the claim).
- Everything the binary prints on failure names the next action (missing stamp →
  the verify command line, verbatim, with the user's own flags substituted).

## Non-goals

CI wiring (no remote exists). Running the L-scale claim (human-owned). Any form
of automatic doc publishing.

## Passing criteria

- Unit tests: full parse matrix (every subcommand's happy path + one malformed
  case each, golden error strings for the stamp-refusal and feature-missing
  messages with substituted flags); digest-directory reuse logic (marker present
  ⇒ no regeneration — test via a counter hook on the generator entry point);
  `PARTIAL` verdict on filtered runs; `UNVERIFIED` provenance on the override
  path (markdown contains the marker).
- An S-scale `gen → verify → bench --families point --samples 8` sequence runs
  end-of-module in a `#[test]` (the suite's own integration point, unit-scale by
  S's size) and produces the three artifacts.
- `scripts/check.sh` green. All architecture-doc amendments landed.
