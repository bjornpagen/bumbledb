# PRD 05 — allow becomes expect: stale suppressions fail the gate

**Depends on:** 01–04 (closes Phase A: converts whatever suppressions
survived them).
**Modules:** every `#[allow(...)]` in `crates/` (~121 sites at audit
time), `crates/bumbledb/src/lib.rs` / crate roots (crate-level allows),
`Cargo.toml` `[lints]` tables if any allow lives there.
**Authority:** the gate suite (`scripts/check.sh` runs
`clippy -D warnings`); the pinned toolchain (1.96) supports `#[expect]`
with `reason =`. The audit's finding: two of the ~121 allows were already
stale — an `#[expect]` regime makes that entire defect class
self-detecting, forever.
**Representation move:** an `#[allow]` is a guard with no expiry — it
suppresses whether or not the lint still fires. `#[expect]` is the same
suppression carrying its own proof obligation: the moment the underlying
code stops triggering the lint, `unfulfilled_lint_expectations` fires and
`-D warnings` fails the gate. The suppression becomes a checked claim.

## Context (decided shape)

- Every item-level and block-level `#[allow(clippy::…)]` and
  `#[allow(unsafe_code)]`-adjacent lint suppression converts to
  `#[expect(…, reason = "…")]`, carrying the reason that today lives in
  a neighboring comment (most sites already have one — lift it into the
  attribute; keep the prose comment only where it says more than the
  reason).
- Exceptions, exhaustive:
  1. **cfg-conditional code** where the lint fires under one cfg and not
     another (e.g. `obs.rs`'s trace/no-trace twin bodies,
     aarch64/portable kernel pairs): `#[expect]` would be unfulfilled in
     one configuration. These stay `#[allow]` with a comment naming the
     cfg asymmetry — and the PRD records each kept site in the commit
     body.
  2. **`#![allow]` crate/module-level policy lints** (if any exist for
     pedantic-group tuning in `[lints]` workspace tables): lint-group
     policy is configuration, not suppression; stays.
  3. **Macro-expansion suppressions** where the attribute lands on
     emitted code paths the macro cannot predicate (verify in
     bumbledb-macros; expected: none).
- The two known-stale allows are already deleted by PRD 04; this PRD's
  first full clippy run proves no OTHER stale suppression exists — any
  `unfulfilled_lint_expectations` error found during conversion is a
  stale allow discovered, and the fix is deletion, not softening.

## Technical direction

1. Mechanical sweep, file by file: `grep -rn "#\[allow(" crates` is the
   worklist. For each: does the lint still fire? (Convert to `expect`,
   build; unfulfilled → the allow was stale → delete outright.) Does a
   reason comment exist? (Lift to `reason =`.) Is it cfg-asymmetric?
   (Keep `allow`, record.)
2. Convert in one pass per crate, running
   `cargo clippy -p <crate> --all-targets -- -D warnings` between crates
   — the unfulfilled-expectation lint IS the verification, so the PRD is
   self-checking as it proceeds.
3. Test modules included — test-only allows rot the same way.
4. Do not touch `#[allow(unused)]`-family in generated/emitted code
   (expected: none exist; the macro emits clean code by construction).

## Passing criteria

- `[shape]` `grep -rn "#\[allow(" crates | wc -l` → only the recorded
  cfg-asymmetric exceptions (each named in the commit body with its cfg
  pair); every other suppression is `#[expect(…, reason = "…")]`.
- `[shape]` Every `#[expect]` carries a non-empty `reason`.
- `[test]` `cargo clippy --workspace --all-targets -- -D warnings` green
  under the default feature set AND under `--features trace` AND with the
  bench crate's `obs` feature (the three configurations the gate suite
  exercises) — proving no expectation is unfulfilled in any gated config.
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

Repo `README.md` measurement-discipline section: one sentence — lint
suppressions are `#[expect]`-checked claims; a suppression that stops
being needed fails the gate by itself.
