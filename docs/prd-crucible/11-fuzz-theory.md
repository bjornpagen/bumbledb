# PRD 11 — fuzz target: theory (schema, dependencies, judgment)

**Depends on:** 10 (the entropy seam), 01 (nightly + cargo-fuzz).
**Modules:** new `fuzz/` crate at repo root (cargo-fuzz layout, OUTSIDE
the workspace — its own `Cargo.toml` with `[workspace]` empty table so
workspace gates never build fuzz artifacts), `fuzz/fuzz_targets/
theory.rs`, a `fuzz/src/lib.rs` harness module shared by all five
targets; `crates/bumbledb-bench` gains whatever `pub` surface the
harness needs (generation entry points — audit what is currently
test-only).
**Authority:** the fuzzing charter: targets drive REAL public API
against the independent oracles; the harness owns no logic worth fuzzing
(refusal: we do not fuzz the harness). First target is theory because it
is the trust root — everything else assumes accepted schemas are sound.
**Representation move:** none in the engine. The new artifact is the
`fuzz/` crate whose shape the other three target PRDs inherit — get the
skeleton right here.

## Context (decided shape)

- **Crate layout:** `fuzz/Cargo.toml` (libfuzzer-sys, `bumbledb` +
  `bumbledb-bench` by path; `[workspace]` detached), `fuzz/src/lib.rs`
  (the shared harness: byte-slice → `Rng::Bytes` → `Scale::Tiny`
  generation → scenario runners returning typed verdicts),
  `fuzz_targets/*.rs` each a thin `fuzz_target!` calling one runner.
  Targets: `theory` (this PRD), `ops` (12), `query` + `rewrites` (13),
  `crash` (14).
- **The theory runner:** from fuzzer bytes, generate a
  `SchemaDescriptor` through a RANDOM-descriptor arm — unlike the
  differential's always-valid generator, this arm deliberately emits
  invalid structures (dangling column refs in FDs/INDs, arity
  mismatches, duplicate names, closed-relation member abuse, interval
  misuse) alongside valid ones, by generating structurally-free
  descriptors and letting the engine judge.
- **Oracles (all three per iteration):**
  1. **No-panic totality:** `Db::create`/schema acceptance returns
     `Ok` or a typed error — any panic/abort is a finding by
     definition.
  2. **Typed rejection:** every `Err` is a named schema/validation
     error variant; the harness matches exhaustively and treats any
     catch-all path as a finding (the error enum is closed — prove it
     stays that way under hostile input).
  3. **Judgment determinism:** accept the same descriptor twice (fresh
     stores) → identical verdict; accepted schemas re-open cleanly and
     `verify_store` passes on the empty store.
- **Corpus:** `fuzz/corpus/theory/` seeded by a small generator run
  (checked in); `fuzz/artifacts/` gitignored.

## Technical direction

1. Build the crate skeleton + shared harness first; keep the harness
   under ~200 lines — it maps bytes to existing generators and matches
   verdicts, nothing else.
2. The random-descriptor arm lives in `bumbledb-bench`'s generator
   (beside the valid-schema generator, sharing its vocabulary), not in
   the fuzz crate — the fuzz crate stays logic-free.
3. Exhaustive error matching: write the verdict matcher as a total
   `match` on the error enum so a future variant addition is a compile
   error in the fuzz crate — the matcher is itself a census instrument.
4. Smoke it: `cargo +nightly fuzz run theory -- -runs=100000` locally
   must complete finding-free before the PRD closes (a real bug found
   instead is a BETTER outcome: file it, fix it in a standalone commit,
   record the trophy in the README ledger).

## Passing criteria

- `[shape]` `fuzz/` crate exists, detached from the workspace
  (`cargo check` at repo root does not build it; `cargo +nightly fuzz
  check` builds all declared targets).
- `[shape]` The verdict matcher is a total match — zero `_ =>` arms
  over engine error enums in `fuzz/src`.
- `[test]` The 100k-run smoke completes with zero findings (or each
  finding fixed + trophy-recorded).
- `[shape]` Seed corpus checked in; artifacts gitignored.
- `[gate]` Workspace gates unaffected (the detachment proof).

## Doc amendments (rule 5)

New section in the measurement/testing doc: the fuzzing charter (targets,
oracle discipline, corpus policy, trophy ledger location) — written here
because this PRD creates the substrate; later target PRDs append one
line each.
