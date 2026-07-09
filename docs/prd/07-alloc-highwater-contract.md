# PRD 07 — The high-water allocation contract

**Depends on:** nothing.
**Modules:** `crates/bumbledb/tests/alloc_gate.rs`, `scripts/check.sh`,
`docs/architecture/40-execution.md` (contract text).
**Authority:** `40-execution.md` § the allocation contract + § CI gate protocol.

## Context (decided)

The contract promises zero heap allocations on a warm execution; three scratch
structures can grow on a warm re-bind whose intermediates exceed anything warmup
saw: `cancel_origin`'s `self.cancelled.resize(idx + 1, …)`
(`exec/run/cancel.rs:14-21`), `next_origin` growth to the per-execution
absorb-node survivor count (`exec/run/probe_pass.rs:295-301`), and
`pending_bindings`/`pending_cursors` growth on larger node-to-node
intermediates. The gate draws warm parameters from the same fixed set as warmup,
so it structurally cannot observe this. **Decided: amend the contract to the
true invariant — monotone high-water convergence — and give the gate eyes.**
Presizing to "provable bounds" is rejected: no useful static bound on
absorb-node survivors exists, and estimates-as-bounds would violate the
estimates-are-non-semantic separation. First-execution and post-commit rebuild
allocations stay sanctioned as today; this PRD is about *warm* growth only.

## Technical direction

1. **Contract text** (`40-execution.md`, both the contract paragraph and the CI
   gate protocol): a warm execution performs zero heap allocations *unless its
   intermediate sizes exceed every prior execution's* — scratch is monotone
   retained-capacity; allocations occur only on strictly-increasing input-shape
   high-waters; pools reach a fixpoint per **(data generation, parameter
   envelope)** (amend the existing fixpoint sentence with the envelope clause).
   State it as the stronger-because-true claim, not a weakening.
2. **Gate variant** (alloc_gate.rs, wired into `scripts/check.sh` alongside the
   existing gate): an escalating-parameter run — a parameter sequence whose
   selectivity strictly increases across the measured window (construct the
   fixture so each param binds a strictly hotter key; the corpus builder in the
   gate file already shapes data deliberately) — asserting (a) allocations occur
   *only* on executions that set a new intermediate high-water, and (b) a repeat
   of any previously-seen parameter allocates zero. The existing fixed-set zero
   check remains as the steady-state assertion.
3. **Mutation check — the gate must not be theater.** Demonstrate the variant
   fails against a deliberately-introduced per-execution allocation and passes
   on the real engine. Mechanism: a `#[cfg(test)]`-only injection point is NOT
   acceptable (test-only code in the hot path); instead do the demonstration
   manually during development and record it in the test's doc comment (the
   exact temporary mutation used, e.g. "a `Vec::new()` per execution in the
   sink emit path made the monotonicity assertion fail at execution 2"), plus
   assert the variant's sensitivity structurally: the test itself verifies that
   its high-water tracking observed at least one growth event across the
   escalation (a gate that never sees growth is vacuous).

## Passing criteria

- `[shape]` The contract paragraph and gate protocol in `40-execution.md` state
  the high-water invariant and the parameter-envelope clause; no "zero
  allocations, unconditionally" sentence survives.
- `[test]` The escalating-parameter gate variant: monotonicity assertion +
  repeat-parameter-zero assertion + the vacuousness guard (≥1 growth event
  observed); doc comment records the mutation demonstration.
- `[shape]` `scripts/check.sh` runs the variant.
- `[gate]` Workspace gates green, including the new variant.
