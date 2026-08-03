## validate_capacity's closed-both-sides arm re-implements the measure reading inline — the second copy has already drifted on the ray arm

unification | low | CONFIRMED | lean-capacity-drift
outcome: fixed d831280a

### Summary

The closed×closed capacity decision at declaration (`crates/bumbledb/src/schema/validate.rs:928-1006`) hand-rolls the entire measure semantics — dependent-bound resolution (:946-962), per-weight child reading (:979-996), and the window verdict (:998) — instead of consuming the one definition in `crates/bumbledb/src/storage/commit/judgment.rs` (`child_weight` :116-140, `interval_measure` :164-182, `resolve_hi` :1187-1212, verdict compare :1170). The copy has already diverged from its original on the ray arm: both validate-side Duration reads compute `end - start` with no ray check, where the judge-side original raises the typed C10 refusal. The divergence is benign today only via a non-local invariant that nothing at the fold's site records.

### Evidence (verified)

- **The inline copy.** `validate.rs:946-962` resolves the dependent ceiling per parent axiom (Lit / TargetField / TargetDuration — the same three arms as `resolve_hi` at `judgment.rs:1192-1211`); `validate.rs:979-996` reads each matching child's weight (Unit=1 / Field word / DurationOf `end - start` — the same three arms as `child_weight` at `judgment.rs:121-139`); `validate.rs:998` is the window compare, character-for-character the same shape as `judgment.rs:1170` (`measure < lo || hi.is_some_and(|hi| measure > hi)`).
- **The ray drift.** Both validate-side Duration reads end in `u128::from(end - start)` (`validate.rs:960` and `:994`) behind `.expect("sealed rows hold canonical interval bytes")`, with no `end == u64::MAX` check. The judge-side original, `interval_measure` (`judgment.rs:164-182`), refuses a ray with the typed `Error::CapacityRayMeasure` at :175-180 (ruled 2026-07-24, C10).
- **The non-local invariant.** The copy is safe only because sealed extensions refuse ray intervals at `validate.rs:1748-1764` (`SchemaError::ExtensionIntervalRay`), whose own comment says "Rays stay honest values everywhere else." Nothing at :946-996 cites or enforces this dependency.
- **The repo's own discipline.** `judgment.rs:890-894` ("consumed by three callers … never a copy") and the `check_capacity` doc at :1091-1093 ("Shared verbatim by the commit path … and `Db::verify_store` — one definition, never a sweeper copy") state the anti-copy rule for exactly this walk. The Lean side already ran this consolidation once: `lean/Bumbledb/Capacity.lean` § "The pigeonhole, consolidated" (line 49) merged two drifted downstream proofs into one home. The naive twin (`bumbledb-bench/src/naive.rs:726`) is the deliberate independent third; validate.rs is engine-side and has everything `child_weight` needs — weight, sealed tails, layout, fact — in hand at `validate.rs:854-914`.
- **The corner is live.** `judgment.rs:103-106` records "Semantic corner, OWNER RULING OWED" on ray-valued Duration weights — this exact posture is acknowledged as unsettled, so the three engine-side spellings with three ray postures are a real drift surface, not a frozen one.

### Failure scenario / impact

None today — the extension-seal ray refusal makes the missing check unreachable. But if a future change relaxes that refusal (its own comment leans that way: rays are "honest values everywhere else"), the validate-time fold silently computes `u64::MAX - start` as a huge finite measure. Concretely: a closed statement `{5..}` with one ray child — engine measure `u64::MAX - start ≥ 5`, statement **accepted at declaration**; Lean denotation `Value.durationNat` reads a ray as 0 (`Capacity.lean:452-456`, `iv.measure.getD 0`), measure 0 < 5, statement **refuted**. An engine-vs-Lean wall break decided at declaration, where no judge or sweeper ever re-checks (closed relations take no writes).

### Suggested fix

Refactor `child_weight`/`interval_measure` to take `(weight, weight_tail, layout, fact)` (dropping the `&CapacityStatement` dependency) so the sealed-row fold at `validate.rs:979-996` and the bound resolution at :946-962 spend the judge's own readers, mapping the ray `Result` into the `ClosedStatementRefuted`/typed-refusal arm explicitly. The window compare at :998 stays as the one inline line; the naive twin stays independent by design. This also localizes the currently-unrecorded dependency on the extension ray refusal: relaxing :1748-1764 would then hit a typed error at validate time instead of a silent wrong measure.