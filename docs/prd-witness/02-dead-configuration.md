# PRD 02 — Dead configuration: fold-off, skip_free, and the defunct dump

**Depends on:** baseline only.
**Modules:** `crates/bumbledb/Cargo.toml`, `crates/bumbledb/src/lib.rs`,
`crates/bumbledb/src/ir/normalize.rs`, `ir/normalize/fold.rs`,
`crates/bumbledb/src/plan/fj.rs`, `plan/fj/validate.rs`,
`api/prepared/introspect.rs`, `api/stats.rs` (mechanism name:
the `skip_free` stat), `crates/bumbledb/src/alloc_counter.rs`,
repo root `audit/`, `.claude/settings.local.json`, `scripts/bench.sh`.
**Authority:** the 2026-07-12 audit (dead-code census + rulings track);
the owner's rulings of 2026-07-12: fold-off DELETE, skip_free DELETE,
audit/ DELETE, bench.sh DELETE.
**Representation move:** dead configuration is a lie in the build
surface — a feature no configuration enables, a flag no executor reads,
a directory no reference reaches. Deletion is the proof.

## Context (decided shape)

- **`fold-off` is enabled by no build configuration.** Declared
  (`crates/bumbledb/Cargo.toml`, `fold-off = []`) but — unlike
  `chase-off`, which `bumbledb-bench/Cargo.toml` enables as a dev-dep —
  nothing turns it on. The `#[cfg(feature = "fold-off")]` re-export of
  `with_fold_disabled` in `lib.rs` compiles in no configured build, and
  the `feature = "fold-off"` disjuncts inside
  `#[cfg(any(test, feature = "fold-off"))]` never activate (the `test`
  arm covers every in-crate consumer). The bench fold differential
  deliberately drives its off-leg through `with_chase_disabled` — the
  evaluator lives inside the same fixpoint, so the chase switch covers
  it; that argument is already in `fold.rs`'s doc and is the reason
  DELETE beats WIRE. The rationale comments in `Cargo.toml` and
  `lib.rs` ("enabled only as a bench dev-dependency") are false and die
  with the feature.
- **`skip_free` is a dead eligibility flag.** Its charter
  (`plan/fj.rs`: "SkipSuffix can never cross a node, so the pipelined
  executor's cross-node batching needs no cancellation machinery")
  describes a precondition the executor dropped when origin cancellation
  landed — the pipeline is built unconditionally for ≥2 nodes and always
  carries cancellation state. Only reader: the introspection stat.
- **`alloc_counter::reset_peak`** is public test-support surface whose
  only caller is its own unit test. The peak-window API goes if and only
  if nothing else reads it (verify `peak_live_bytes` consumers before
  cutting; the bench harness uses `reset`/`snapshot` only).
- **`audit/`** (repo root) is the July-2 pre-reset review dump — its own
  chapter numbering, zero inbound references, superseded by the rebuilt
  chapters. **`scripts/bench.sh`** has zero inbound references and is
  absent from the README's scripts inventory. **`.claude/settings.local.json`**
  carries allowlist grants for deleted scripts and a one-off wipe command.

## Technical direction

1. Delete the `fold-off` feature: the `Cargo.toml` line, the `lib.rs`
   gated re-export and its stale rationale comment, and the
   `feature = "fold-off"` disjuncts in `ir/normalize.rs` and
   `ir/normalize/fold.rs` (the gates become plain `#[cfg(test)]` /
   `#[cfg(any(test))]` → `#[cfg(test)]`). `with_fold_disabled` itself
   stays — it is live under `cfg(test)` for the fold-preservation and
   statically-empty suites. Fix `fold.rs`'s doc sentence to say the
   bench's off-leg is the chase switch, by design.
2. Delete `skip_free`: the `ValidatedPlan` field, its computation in
   `plan/fj/validate.rs`, the accessor, and the introspection stat that
   reads it (update the explain/introspect tests that pin the stats
   struct shape). If any tripwire pins the stat, the tripwire row is
   deleted with it — the flag gates nothing, so no structural signal is
   lost.
3. Delete `alloc_counter::reset_peak` and its self-test; delete the peak
   bookkeeping it fronted iff the census confirms zero remaining readers
   (one grep for `peak` under `crates/`); otherwise stop at the fn and
   record the reader found.
4. `git rm -r audit/`; `git rm scripts/bench.sh`; prune the dead grants
   from `.claude/settings.local.json` (deleted scripts, the one-off `rm`
   grant). `docs/brainlift-sources/` is NOT touched (set refusal).
5. Expect fallout in `Cargo.lock`? None — no dependency changes. Expect
   fallout in `scripts/check.sh`? None — it never referenced fold-off or
   bench.sh; verify by grep.

## Passing criteria

- `[shape]` `grep -rn "fold-off" . --include='*.toml' --include='*.rs'
  --include='*.sh'` → zero hits; `with_fold_disabled` reachable only
  under `cfg(test)`.
- `[shape]` `grep -rn "skip_free" crates` → zero hits; the introspection
  stat struct carries no skip-related field; explain tests updated and
  green.
- `[shape]` `grep -rn "reset_peak" crates` → zero hits (or the recorded
  reader, per direction 3).
- `[shape]` `audit/` and `scripts/bench.sh` do not exist;
  `.claude/settings.local.json` names no deleted script.
- `[test]` The fold-preservation suite (`ir/normalize/fold` tests,
  `api/prepared/tests/statically_empty.rs`) and the bench fold
  differential still pass unmodified in behavior — the switches they use
  (`cfg(test)` fold switch, `chase-off`) are untouched.
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`docs/architecture/70-api.md` (feature inventory, if it lists fold-off —
verify) and the repo `README.md` scripts inventory (bench.sh was never
listed; confirm and leave). 40-execution's fold section: one sentence —
the test-only fold switch rides `cfg(test)`; the dual-run differential's
off-leg is the chase switch, which subsumes the evaluator.
