## KeyStatement seals a bool where validation already holds the IntervalTail; the commit path re-derives it per fact and per probe

category: inappropriate-branching | severity: low | verdict: CONFIRMED | finder: lean:schema-values
outcome: fixed 4d19deb8 + f9ca914f + 6aecf449

### Summary

The sealed key witness stores `pub pointwise: bool` (`crates/bumbledb/src/schema.rs:383-392`), flattening the validator's `FunctionalityEvidence::Pointwise(DisjointDeterminantProof)` to a flag at the seal boundary (`crates/bumbledb/src/schema/validate.rs:138`). Yet `validate_functionality` computes the interval position (`validate.rs:528`) and, in its closed-relation arm, literally constructs the `IntervalTail { width }` from the field's `ValueType` (`validate.rs:602-607`) — the full descriptor is in hand at seal time and then thrown away. The commit path re-derives it repeatedly at runtime via a projection walk, and re-asserts the validator's theorem with four `expect`s. This violates the staging law the codebase itself states for σ literals (`schema.rs:341-345`; `docs/architecture/30-dependencies.md` § "The checker consumes constants": "everything whose canonical bytes are a pure function of the value seals here, once" — and, pointedly for this exact machinery, "no boolean can license the sweep"). It is the parse-don't-validate defect named in `docs/design/representation-first.md`: validation checked the condition and discarded what it learned.

### Evidence (all verified against the code)

- `crates/bumbledb/src/schema.rs:391` — `pub pointwise: bool` on `KeyStatement`; `schema/validate.rs:138` — `pointwise: matches!(evidence, FunctionalityEvidence::Pointwise(_))` flattens the minted evidence to a bool.
- `crates/bumbledb/src/schema/validate.rs:528, 602-607` — the validator computes `interval_position` and builds an `IntervalTail` from `relation.fields[idx].value_type` inside `validate_functionality`; nothing of it survives into the sealed statement.
- `crates/bumbledb/src/storage/commit/plan.rs:270-306` — `fact_op` ("Derives one fact's op", called per fact) runs, for every key statement of the relation, `statement.pointwise.then(|| schema.key_tail(statement).expect("a pointwise key has a tail"))` (plan.rs:300-304).
- `crates/bumbledb/src/schema.rs:234-235` and `schema/relation.rs:80-87` — `key_tail` delegates to `Relation::interval_tail`, a `find_map` over the projection matching on `ValueType` per call.
- `crates/bumbledb/src/storage/commit/plan.rs:126` — `DeterminantOp.pointwise: Option<IntervalTail>` already exists: the per-op plan type is exactly the type the sealed witness should carry.
- `crates/bumbledb/src/storage/commit/judgment.rs:313-331` — `source_tail: schema.source_tail(statement)` is computed inside the per-edge worklist loop (per probe). Note this also runs for every `ScalarProbe` edge, where the walk scans the entire source projection just to return `None`. Same recomputation per iteration at judgment.rs:596, and per dependent at 457-462 with two more `expect`s; `check_coverage` re-derives `key_tail(target_key)` per probe at 862-865.
- Four expects re-assert what validation proved: plan.rs:303, judgment.rs:459, 461-462, 864-865 (plus judgment.rs:859-861 asserting the probe carried its tail — erased too if `Enforcement::IntervalCoverage` sealed both tails).
- `crates/bumbledb/src/verify_store/determinants.rs:95-96` — the offline sweeper repeats the same re-derivation per key-statement transition (cold lane, listed for completeness).
- `IntervalTail` is `Copy` (`schema.rs:193-197`), so sealing it costs one `Option<u64>`-sized field. The sealed `KeyStatement` is never serialized — `schema/fingerprint.rs` and `schema/descriptor_codec.rs` contain no reference to it — so fingerprints are unmoved.

### Doctrine check

`docs/architecture/30-dependencies.md` § "The checker consumes constants" records exactly two audited stays of the staging law (the `FactLayout` rebuild at open, the fresh→FD materialization at validate); the interval tail is neither. The same paragraph rules that the pointwise/coverage judgment consumes the validator-minted `DisjointDeterminantProof` because "no boolean can license the sweep" — yet the tail that parameterizes that sweep is gated by precisely a boolean and re-derived on demand.

### Bench impact / failure scenario

Perf-lane only, and marginal (severity low): any commit into a relation with pointwise keys pays a per-fact, per-key projection walk in `fact_op`; any commit touching containment edges pays a per-probe source-projection walk in the source and target judgments — including full-projection walks that conclude `None` on every scalar probe. No correctness failure is reachable; the four `expect`s are the residue, not a live panic path (validation guarantees them).

### Suggested fix

Have `validate_functionality` return the tail it already computed (e.g. `FunctionalityEvidence::Pointwise(DisjointDeterminantProof, IntervalTail)`), and seal it on `KeyStatement`; likewise seal `source_tail: Option<IntervalTail>` on `ContainmentStatement` (or on the `ScalarProbe`/`IntervalCoverage` enforcement arms, where `IntervalCoverage` could carry both tails and erase judgment.rs:859-865 entirely). `Schema::key_tail`/`source_tail` become field reads, plan.rs:300 becomes a copy, and the four expects disappear. One API caveat verified: `pointwise` is `pub` and read outside the crate (`crates/bumbledb-bench/src/sqlmap.rs:97`, `crates/bumbledb-bench/src/calendar/tests.rs:70`, `crates/bumbledb-query/tests/cookbook.rs:1102`) while `IntervalTail` is `pub(crate)` — so either keep `pointwise` as a derived `tail.is_some()` accessor/field for the public surface, or promote `IntervalTail`.
