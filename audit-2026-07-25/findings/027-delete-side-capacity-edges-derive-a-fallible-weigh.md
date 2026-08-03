## Delete-side capacity edges derive a fallible weight that is never read — the one repair path for a corrupt ray row refuses

inelegance | low | CONFIRMED | capacity-judge
outcome: fixed 2b1e87b0

### Summary

`mark_ops` (crates/bumbledb/src/storage/commit/plan.rs:423-476) derives `MarkEdgeOp::weight` uniformly for both dispositions — `fact_op` runs the same derivation for deletes (plan.rs:242-252) and inserts (plan.rs:255-265) — but the applier's delete removal is key-only and never reads the weight (applier.rs:61-69; the weight is spent only by `insert_fact`, applier.rs:178-184). The derivation is the *one fallible slice in the entire plan* (plan.rs:215-221): `child_weight` on a `Duration` weight refuses a ray typed (`Err(CapacityRayMeasure)`, judgment.rs:164-182). Consequence: a commit that only DELETES a phi-satisfying ray-weighted row is refused at plan time by a value it would have discarded — and deleting such a row is exactly the repair `verify_store` implies.

### Evidence (verified)

- plan.rs:445-455 — inside `mark_ops`, gated only on `satisfies(&selections.capacity(capacity_id).source, ...)`, never on disposition: `Some(child_weight(statement, layout, fact)?)` for any non-`Unit` weight.
- plan.rs:117-120 — `MarkEdgeOp` doc concedes the trade: "the delete removal is key-only, so the delete side's `weight` is derived and unread — the uniform derivation is cheaper than a disposition split."
- plan.rs:215-221 — plan_commit's `# Errors`: "The one fallible slice is the weighted edge's weight derivation ... a ray-valued Duration weight has no finite u64 for the value slot, so it refuses typed at plan time."
- applier.rs:61-69 — delete side: "the removal is key-only, so the plan's delete-side weight is unread." Grep confirms `capacity_edges` has exactly two consumers: the key-only delete loop (applier.rs:65) and the weight-spending insert loop (applier.rs:178-184).
- judgment.rs:170-181 — `interval_measure` returns `Err(Error::CapacityRayMeasure { .. })` on `end == u64::MAX`.
- verify_store/facts.rs:227-242 — a stored ray under a weighted capacity edge is a malformed-content finding ("R capacity weight of a ray"), and verify_store.rs:61-67 fixes the doctrine: "convict-only, never repaired silently"; "Findings are data ... the *caller* decides fatality." Repair is therefore the caller's, through the write surface.
- No alternate repair path exists: `exhume` (api/db/exhume.rs) is explicitly read-only ("No write surface exists on this type ... never takes the writer path"), and the schema fingerprint is pinned at open (storage/env/open.rs:79, `check_fingerprint`), so the state is reachable by corruption/tampering only — which is precisely the state `verify_store` exists to convict.
- judgment.rs:103-106 — the write-time ray refusal is already recorded as "OWNER RULING OWED," described as "visible only for a ray child under an absent parent"; the delete-side repair-blocking consequence is not part of that recorded corner.

### Failure scenario / impact

A store carries a consistently tampered/corrupt ray-valued row under a `Duration`-weighted capacity statement (F/M/U/R all coherent with the ray fact — a state the write path could never produce, since the insert refuses at plan time). `Db::verify_store` convicts the R slot as content ("R capacity weight of a ray") and, per doctrine, repairs nothing. The operator issues the obvious repair delta deleting the convicted row: `plan_commit` calls `fact_op` → `mark_ops` → `child_weight` → `interval_measure` → `Err(CapacityRayMeasure)`. The row is undeletable through the entire write surface — contradicting the convict-only-because-repair-is-the-caller's posture. Impact is low (corruption/tampering-only reachability) but the inelegance is structural: the delete path's only failure mode is inherited from a value it discards.

### Suggested fix

Split the derivation by disposition: have `fact_op` pass the disposition to `mark_ops` (or take it itself) and derive `weight` only for inserts, leaving delete-side `MarkEdgeOp`s weightless (`weight: None`) and the delete plan infallible. This also retires the write-time-ray-refusal-on-delete corner from the owner-ruling-owed note at judgment.rs:103-106, narrowing that recorded ruling to genuine inserts. Land with a test: a store seeded (via test-only raw writes) with a ray-valued row under a Duration-weighted capacity, convicted by `verify_store`, then successfully repaired by a delete-only delta.