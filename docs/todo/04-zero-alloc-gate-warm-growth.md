# 04 — Zero-alloc contract: warm-path growth past the warmup high-water

**Kind:** contract hole — the allocation contract (`40-execution.md`) promises zero
heap allocations on a warm execution; three scratch structures can grow on a warm
re-bind whose intermediates exceed anything warmup saw. The CI gate's fixed
parameter set hides the hole by construction. **Decided: amend the contract to the
true, stronger-because-true invariant** — monotone high-water convergence — and
make the gate able to see violations. Presizing to "provable bounds" is rejected:
no useful static bound on absorb-node survivors exists, and estimates-as-bounds
would violate the estimates-are-non-semantic separation.

## Current behavior

- `cancel_origin` does `self.cancelled.resize(idx + 1, …)`
  (`crates/bumbledb/src/exec/run/cancel.rs:14-21`), and `next_origin` grows to the
  per-execution survivor count at the absorb node
  (`exec/run/probe_pass.rs:295-301`). A warm execution producing more absorb-node
  survivors than any warmup run reallocates.
- Same class: `pending_bindings` / `pending_cursors` `Vec` growth when a warm bind
  yields a larger node-to-node intermediate than warmup ever produced.
- The gate protocol draws warm parameters **from the same fixed set** as warmup
  (`40-execution.md`, CI gate protocol), so the gate can never observe this.

Related but distinct (documented, not a hole): first execution and post-commit
rebuild allocations are sanctioned; this item is only about *warm* growth.

## The work

- **The contract becomes:** a warm execution performs zero heap allocations
  *unless its intermediate sizes exceed every prior execution's* — scratch is
  monotone retained-capacity, allocations occur only on strictly-increasing
  input-shape high-waters, and pools reach a fixpoint per (data generation,
  parameter envelope). This is what the code already does; the work is making the
  docs and the gate say it.
- **Gate variant:** an escalating-parameter run — parameter selectivity
  deliberately increased across the measured window — asserting allocations occur
  *only* on strictly-increasing high-waters (monotonicity check), alongside the
  existing fixed-set zero check (which remains the steady-state assertion).
- **Mutation check (the gate must not be theater):** the variant demonstrably
  fails against a deliberately-introduced per-execution allocation, and passes on
  the real engine. Record the demonstration in the test's doc comment.

## Acceptance

- The gate suite (`scripts/check.sh`) includes the escalating-parameter variant;
  the mutation check is recorded.
- `40-execution.md`'s contract paragraph states the actual invariant, and the
  "pools reach a fixpoint for a given data generation" sentence gains "and
  parameter envelope".

## Doc amendments (rule 5)

`40-execution.md` allocation contract + CI gate protocol.
