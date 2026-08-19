# 09 — `profile` is still off the sealed `Instance`; `hit` has two derivation sites

- **Status:** **fixed this pass** — `Instance::profile` on both arms
  via `profile_on`; `KeyProbeStats::from_emitted` owns `hit`; tests:
  `profile_on_owned_and_lease_agrees`,
  `profile_returns_structured_stats_matching_the_execution`.
- **Severity:** should-fix.
- **Supersedes:** PROP-008, EXEC-03.
- **Adjudication (third pass): keep CONTESTED on the promotion half —
  owner ruling required; EXEC-03 open by the ruling's own text.** The
  keep-ruling conflates `profile` with `staleness`. Staleness is the drift
  clock, is lease-only, and stays lease-only (recorded in kept.md — no
  disagreement). `profile` is counting instrumentation: run the query,
  return `ExecutionStats`. A frozen catalog runs queries, so it profiles;
  no generation clock is consulted and no drift is fabricated. The
  proposal's promotion gate was "`RuleStats` becomes a sum first" — that
  landed. If the owner nonetheless rules profile lease-only, the sealed
  trait comment and the proposal's roster must be edited to match, so the
  contradiction does not stand silently.

## Principle

The proposal promoted `profile` onto the sealed trait *gated on* the stats
shape becoming a sum — the sum landed (`RuleStats` is an enum), the promotion
didn't. And the one remaining stats fact derived twice (`hit`) is the exact
drift shape the promotion gate named: two sites can disagree the day one is
edited.

## Evidence

- `crates/bumbledb/src/api/db/instance.rs` — the `Instance` trait has
  `execute`/`scan`/`scan_facts`/`get`/`contains`/`row_count`; **no
  `profile`**. It remains `#[doc(hidden)]` on the lease type only.
- `crates/bumbledb/src/api/prepared/execute.rs:220` and
  `introspect.rs:204` — `hit: emitted > 0` written at both sites.

## The fix

1. One constructor owns the derivation: `KeyProbeStats::from_emitted(emitted:
   u64)` (or the `RuleStats::KeyProbe` arm's builder) — both call sites use
   it; the expression exists once.
2. Add `fn profile(&self, prepared, params) -> Result<(Answers,
   ExecutionStats)>` to the sealed `Instance` trait, implemented by both
   arms via the existing generic execute path with counting counters. The
   stats shape stays explicitly unfrozen (documented as diagnostic), exactly
   as the proposal's promotion note requires.
3. The lease's `#[doc(hidden)]` inherent `profile` becomes the trait impl —
   one entry, not a hidden twin beside a public one.

## Acceptance

- `grep "hit: emitted" crates/` returns one line (the constructor).
- `Instance::profile` exists and is exercised by a test on **both** arms
  (owned and lease) with identical stats for identical inputs.
- The introspection goldens are unchanged (v7).
