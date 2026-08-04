## The commit plan derives the capacity slot weight for DELETE ops the applier never reads

perf | low | CONFIRMED | cross-branching-new
outcome: fixed 2b1e87b0

### Summary

`plan_commit` runs the identical `fact_op` → `mark_ops` pipeline for delete and insert dispositions, and `mark_ops` has no polarity input: every φ-satisfying child of a weighted capacity statement derives `MarkEdgeOp::weight` via `child_weight`, both directions. But the applier's delete side is key-only by design — its own comment says the plan's delete-side weight is unread. For `Weight::DurationOf` the dead derivation is an interval-tail parse plus the C10 ray-refusal check per deleted child, pure plan-time waste on the delete half of every weighted-capacity write.

### Evidence (verified)

- `crates/bumbledb/src/storage/commit/plan.rs:241-266` — `plan_commit` pushes both `delta.deletes()` and `delta.inserts()` through the same `fact_op(...)`; no polarity flag exists anywhere in the signature chain.
- `crates/bumbledb/src/storage/commit/plan.rs:445-455` — `mark_ops` computes `Some(child_weight(statement, layout, fact)?)` for every φ-satisfying child of a non-`Unit` weighted statement, unconditionally, and stores it in `MarkEdgeOp::weight`.
- `crates/bumbledb/src/storage/commit/applier.rs:61-68` — `delete_fact`'s capacity-edge loop reads only `edge.statement` and `edge.key_bytes`; the doc comment states outright: "the removal is key-only, so the plan's delete-side weight is unread."
- `crates/bumbledb/src/storage/commit/applier.rs:178-185` — `insert_fact` is the sole consumer of `MarkEdgeOp::weight` (the C17 value-slot put). Grep confirms no other reader of the delete ops' `capacity_edges`.
- `crates/bumbledb/src/storage/commit/judgment.rs:116-140, 164-182` — `child_weight` for `Weight::DurationOf` calls `interval_measure`: `tail.words(bytes)` parse plus the `end == u64::MAX` ray refusal (`Error::CapacityRayMeasure`), per fact.

Scope nuance: `Weight::Unit` short-circuits to `None` (no cost) and `Weight::Field` is a single 8-byte word read, so the material waste is specifically `DurationOf` statements — the calendar/booking shape.

### Failure scenario / impact

A churn-profile commit deleting N bookings under a duration-weighted capacity law pays N dead interval-tail parses and ray checks at plan time — on exactly the weighted-capacity write path the C17 slot measurement optimized on the insert side. Latent secondary hazard: the dead derivation is *fallible* — a ray-valued Duration child in storage would refuse its own deletion at plan time (`CapacityRayMeasure`). Unreachable today only because the write-time ray refusal keeps such facts out of storage; the polarity-blind shape carries the trap structurally.

### Suggested fix

Thread op polarity into `mark_ops` (or split `MarkEdgeOp` into a keyed-delete shape and a weighted-put shape) so the weight derives only on the insert side; the delete side keeps `key_bytes` alone. This also removes the delete-side refusal trap and lets the plan-doc's "one fallible slice" comment (plan.rs:216-221) narrow to inserts, matching what the applier actually spends.