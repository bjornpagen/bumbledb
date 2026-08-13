# CommitRejected comment says all-containment; statement phase mixes capacity
- id: 212
- severity: low
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: rust
- components: crates/bumbledb/src/error.rs, lean/Bumbledb/Txn.lean, docs/architecture/30-dependencies.md, crates/bumbledb/src/storage/commit/tests/marks.rs
- status: open (do not fix)

## Summary
The public `Error::CommitRejected` doc comment says a rejection is "all-key or all-containment." Lean, tests, and the architecture docs say the statement phase cites containments *and* capacities together, never mixed with keys. Runtime sealing matches Lean; the error-enum comment does not.

## Lean spec
`Txn.lean:49-55`: "one sealed rejection can mix containment and capacity citations in materialized statement order." `rejection_never_mixes` (`:446-452`) is the never-a-mix *with the key phase* law, not "containment-only."

## Normative docs
`30-dependencies.md` (dyn-surface / judged-on-final-states): statement-phase completeness spans non-key forms. Tests document the mix (`marks.rs:1142-1145` `statement_phase_cites_containments_and_capacities_together`).

## Rust implementation
```1330:1339:crates/bumbledb/src/error.rs
    /// … Key (`Functionality`) violations preempt the
    /// containment judgment: … so one rejection is all-key or all-containment, complete
    /// within its phase.
    CommitRejected {
        violations: Violations,
    },
```

Apply/judge implementation and `write.rs` phase-3 comments correctly mention containment/capacity. Only this public error doc is wrong.

## Why this matters
API consumers matching on citation kinds will expect no capacity ids in a non-key `CommitRejected`. Mixed statement-phase rejections are the capacity cutover's intended surface.

## Related
- 200, 218 (`CapacityRayMeasure` is a different constructor, not a violation mix)
