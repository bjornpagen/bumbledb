# 04 — Zero-alloc contract: warm-path growth past the warmup high-water

**Kind:** contract hole — the allocation contract (`40-execution.md`) promises zero
heap allocations on a warm execution; three scratch structures can grow on a warm
re-bind whose intermediates exceed anything warmup saw. The CI gate's fixed
parameter set hides the hole by construction.

## Current behavior

- `cancel_origin` does `self.cancelled.resize(idx + 1, …)`
  (`crates/bumbledb/src/exec/run/cancel.rs:14-21`), and `next_origin` grows to the
  per-execution survivor count at the absorb node
  (`exec/run/probe_pass.rs:295-301`). A warm execution producing more absorb-node
  survivors than any warmup run reallocates.
- Same class: `pending_bindings` / `pending_cursors` `Vec` growth when a warm bind
  yields a larger node-to-node intermediate than warmup ever produced.
- The gate protocol draws warm parameters **from the same fixed set** as warmup
  (`40-execution.md`, CI gate protocol), so the gate can never observe this. A
  param set within the documented small-set assumption but selecting a hotter key
  than warmup did (or a differently-shaped one) breaks the contract in production
  while the gate stays green.

Related but distinct (documented, not a hole): first execution and post-commit
rebuild allocations are sanctioned; this item is only about *warm* growth.

## The work

Two admissible resolutions; pick one explicitly and record the decision:

- **Option A — amend the contract honestly.** The contract becomes "zero
  allocations at the input-shape high-water": scratch is monotone
  retained-capacity, and a warm execution allocates iff it exceeds every prior
  execution's intermediate sizes, after which the new high-water is retained. This
  is what the code already does; the work is making the docs and gate say so, plus
  a gate variant that *deliberately* escalates parameter selectivity across the
  measured window and asserts allocations occur only on strictly-increasing
  high-waters (monotonicity check, not zero check).
- **Option B — presize to a provable bound.** At bind time, bound absorb-node
  survivors and pipeline intermediates from the plan (estimates are not bounds —
  this needs real bounds, e.g. view lengths), and reserve before the measured
  window. Likely over-reserves and fights the "estimates are non-semantic"
  separation; only take this if A is unacceptable.

Option A is recommended: it preserves the true property (steady-state fixpoint per
data generation and parameter envelope) without pretending a stronger one.

## Acceptance

- The gate suite (`scripts/check.sh`) includes the escalating-parameter variant;
  it fails against a deliberately-introduced per-execution allocation (mutation
  check) and passes on the real engine.
- `40-execution.md`'s contract paragraph states the actual invariant, and the
  "pools reach a fixpoint for a given data generation" sentence gains "and
  parameter envelope".

## Doc amendments (rule 5)

`40-execution.md` allocation contract + CI gate protocol, per the option chosen.
