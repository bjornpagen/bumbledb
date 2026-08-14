# engine-039: `prepare_rule_variant`'s `delta: Option<OccId>` is a boolean with an id stuffed in

- **Severity:** low
- **Tree:** engine
- **Status:** DUPLICATE(engine-007)
- **Source:** audit/engine.md F39

The Option side-channel is the call signature of the k-variant leftover engine-007 deletes. That issue already specifies `prepare_rule(...)` vs `prepare_rec_arm(..., delta: OccId) -> RecArm`. No separate fix lands under this id.
