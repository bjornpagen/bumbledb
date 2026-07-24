## validate_cardinality copy-pastes containment's whole side-pair gate

category: unification | severity: low | verdict: CONFIRMED | finder: lean:schema-values
outcome: fixed 4d19deb8 (validate_side_pair extracted with the R16 validate rework)

### Summary

The cardinality acceptance gate in `crates/bumbledb/src/schema/validate.rs` duplicates the containment gate's entire side-pair validation prefix. A direct diff of the two ranges (649-686 vs 797-833) shows **every code line identical; only comments differ**. The Lean model states this as one rule (`lean/Bumbledb/Admission.lean`: `containmentForm` at line 343 and `cardinalityForm` at line 453 both take `Atom` sides through one acceptance structure), and `validate_cardinality`'s own doc comment (validate.rs:749-761) describes its premises as "the shared side shapes, the containment target-key rule reused verbatim." The target-key step *is* shared (`resolve_target_key`, called at validate.rs:710 and 850) — but the ~35 lines upstream of it are a second copy of the first mechanism.

### Evidence

All verified by reading the file and diffing the two ranges:

- validate.rs:649-650 vs 797-798 — identical `validate_side_shape(id, source, …)` / `validate_side_shape(id, target, …)` pair.
- validate.rs:653-659 vs 802-808 — byte-identical arity check; **both return `SchemaError::ContainmentArityMismatch`** (the cardinality gate borrows containment's error variant, confirming it is semantically the same rule).
- validate.rs:668-683 vs 815-830 — identical positional `positional_types_match` loop over zipped projections; both return `ContainmentTypeMismatch`.
- validate.rs:685-686 vs 832-833 — identical `validate_side_selection` pair.
- validate.rs:710 and 850 — `resolve_target_key` already shared, proving the intended shape (one rule, two callers) and making the unshared prefix the anomaly.
- The genuinely form-specific pieces are cleanly separable: the window vocabulary bans (validate.rs:780-795), the v0 interval refusal (838-845), and the closed-count arm (857-898) all sit *outside* the duplicated block.
- The repo's own one-definition-site doctrine: `value_matches` is deliberately single-sited so "the σ rules cannot drift" (schema.rs:6, validate.rs:1134) — the duplicated Q1 loop is exactly the drift surface that doctrine exists to erase.

Spec check: `lean/Bumbledb/Admission.lean` — `containmentForm` and `cardinalityForm` share the side-pair acceptance premise structure; the model proves one rule, the Rust writes it twice. The only semantic nuance between the forms — Q1 interval-width freedom being moot for cardinality — is enforced by the interval refusal *after* the shared block (validate.rs:838-845, with the comment at 810-812 explicitly calling it "moot for acceptance here"), so a shared helper is behavior-preserving.

### Failure scenario

Not a runtime bug — a drift trap. Any future edit to the containment side-pair rule (e.g. tightening fixed-vs-general width pairing in the Q1 loop, or changing the arity error payload) that misses the cardinality copy silently forks the acceptance semantics of two statement forms that the Lean model proves share one rule. The shared error variants (`ContainmentArityMismatch`/`ContainmentTypeMismatch` emitted from both sites) would mask the fork rather than surface it.

### Suggested fix

Extract the shared prefix into one helper, mirroring how `resolve_target_key` is already shared:

```rust
fn validate_side_pair(
    id: StatementId,
    source: &Side,
    target: &Side,
    relations: &[Relation],
) -> Result<Projection, SchemaError>  // the target projection both callers need
```

covering: both `validate_side_shape` calls, the |X|=|Y| arity check, the Q1 positional-type loop, and both `validate_side_selection` calls. `validate_containment` and `validate_cardinality` call it first, then apply their form-specific refusals (closed-interval refusal for containment; window bans + interval refusal for cardinality). One definition site restores the code to the shape the Lean model already has.
