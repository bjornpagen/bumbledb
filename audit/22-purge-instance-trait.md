# 22 — Purge the sealed `Instance` trait and both promotions

- **Status:** **fixed this pass** — sealed `Instance` deleted; inherent methods on both concrete types; `row_count` is `pub(crate)`; `profile` is doc-hidden on both; tests: `profile_on_owned_and_lease_agrees`.
- **Severity:** purge, small.
- **Owner ruling:** "generic host code" is a speculative consumer. TS types
  structurally; Rust hosts hold concrete types. Deleting the trait deletes
  the roster debates permanently.

## Principle

A public abstraction with no consumer is a branch factory in waiting: every
future method must be adjudicated onto or off the trait (the `row_count` /
`profile` promotion fights were exactly this). Concrete types make the
question unrepresentable.

## Cascade

- The sealed `Instance<S>` trait in `api/db/instance.rs` deletes; the
  generic algorithm bodies (`prepare_on`, `execute_on`, `profile_on`, the
  `InstanceCore` internals) **stay** — they are crate-private sharing, which
  was always the real value. Both public types keep their inherent methods.
- `row_count` returns to `pub(crate)` (it was promoted only for the trait
  roster).
- `profile` returns to `#[doc(hidden)]` harness surface on both concrete
  types — this un-does fix 09's promotion, which is hours old and
  consumer-less; `KeyProbeStats::from_emitted` (the one-`hit` constructor)
  stays.
- The TS structural-compat test from fix 03 ("a generic host function
  compiles against ReadInstance and OwnedInstance") stays — structural
  typing is the SDK's genericity story and needs no engine trait.

## Acceptance

- No `trait Instance` in the engine; no public `row_count`; `profile`
  doc-hidden on both types.
- The proposal's roster section rewritten (see 40).
- All suites green; the one-body generic internals untouched (grep:
  `prepare_on`/`execute_on` still single-definition).
