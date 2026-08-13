# C20 write-time ray refusal vs Lean empty-parent vacuity
- id: 200
- severity: high
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: split
- components: lean/Bumbledb/Capacity.lean, docs/architecture/30-dependencies.md, docs/design/capacity-laws.md, crates/bumbledb/src/storage/commit/judgment.rs, crates/bumbledb/src/storage/commit/plan.rs, crates/bumbledb/src/storage/commit/tests/marks.rs
- status: fixed (2026-08-13)

## Summary
Inserting a Duration-weighted capacity child whose interval is a ray, when no matching parent row exists, is a legal no-op under Lean `CapacityLaw` (`capacity_of_empty_parent`) and under the architecture doc that cites that theorem. The engine, by C20 (2026-08-03), refuses the write at plan time with `CapacityRayMeasure`. The design record states this is doctrine and strictly stronger than C10; the architecture docs never mention C20 and still present empty-parent vacuity as the unification stop.

## Lean spec
`CapacityLaw` quantifies only over ψ-selected parents. With no such parent the law holds vacuously:

```534:543:lean/Bumbledb/Capacity.lean
/-- **Behavior under the empty parent denotation.** Every capacity
law holds when no parent fact is selected — capacity constrains
measures PER PARENT and never manufactures a parent; existence
obligations are containments' alone (weight- and bound-independent). -/
theorem capacity_of_empty_parent ... :
    CapacityLaw ... :=
  fun g hg hψ => absurd hψ (hB g hg)
```

A ray Duration weight is modeled as junk-0 (`Value.durationNat` / `Interval.measure.getD 0`), with C10 named as engine mechanism not restated in the denotation (`Capacity.lean:87-92`). C20 is absent from the Lean tree.

## Normative docs
Architecture (`docs/architecture/30-dependencies.md:225-227`) cites the Lean theorem as the unification stop: "capacity statements never manufacture parents (`lean/Bumbledb/Capacity.lean: capacity_of_empty_parent`)." Weight typing there (`:256-258`) states only C10: "A ray-valued Duration weight or bound at judge time is a typed commit refusal naming the row." `rg C20 docs/architecture` is empty.

The design record (`docs/design/capacity-laws.md:411-423`) rules C20: a ray child under an absent parent "the judge would never measure ... but the write now refuses. RULED as doctrine, not accident."

## Rust implementation
Write-time slot derivation refuses a ray Duration weight on INSERT, parent-blind (`judgment.rs:103-110`; `plan.rs:223-228`). The pin is `capacity_duration_ray_under_an_absent_parent_still_refuses` (`marks.rs:896-905`): the exact cell where C10 (judge) and C20 (write) differ.

## Why this matters
A well-typed insert that Lean and the architecture docs treat as a no-op (no parent to constrain) is a hard commit refusal in the engine. Hosts following `capacity_of_empty_parent` will see `CapacityRayMeasure` instead of a successful empty-parent commit. The third oracle / naive twin must mirror C20 or the differential wall disagrees on this cell.

## Verification (2026-08-12)
Re-read Lean, architecture/design docs, and the write path. **Confirmed.** `wrong-side: split` is right: Lean plus `30-dependencies.md` still stop at empty-parent vacuity and C10 (judge time); the engine and `capacity-laws.md` C20 refuse at plan time.

**Lean** (`lean/Bumbledb/Capacity.lean:534-543`): `capacity_of_empty_parent` proves every `CapacityLaw` when no ψ-selected parent exists. Rays are junk-0 in `durationNat` (`:448-461`); C10 is named as engine mechanism not restated (`:87-92`). No C20 symbol in the Lean tree.

**Docs:** Architecture (`docs/architecture/30-dependencies.md:225-227`) cites that theorem as the unification stop; weight typing (`:256-258`) states only C10 (“at judge time”). `rg C20 docs/architecture` is empty. The design record (`docs/design/capacity-laws.md:411-423`) rules C20 as doctrine for the absent-parent cell.

**Rust** (`crates/bumbledb/src/storage/commit/judgment.rs:103-110`, `:228-240`; `plan.rs:223-228`): Duration weight derivation refuses `end == u64::MAX` parent-blind at plan time. Pin: `capacity_duration_ray_under_an_absent_parent_still_refuses` (`marks.rs:896-924`).

## Related
- 218 (70-api write-error roster omits `CapacityRayMeasure`)
- 220 (Lean `durationNat` junk-0 vs C10/C20 refuse)

## Resolution (2026-08-13)
Engine C20 left intact. Architecture (`30-dependencies.md`) now states C20 as write-time, parent-blind Duration-ray refusal (`CapacityRayMeasure`). Lean `Capacity.lean` records C20 as engine law: empty-parent vacuity does not license a ray Duration child insert; `durationNat?` is `none` on rays.
