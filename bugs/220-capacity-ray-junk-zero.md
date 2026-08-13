# Lean Duration weight of a ray is 0; engine refuses the commit
- id: 220
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: spec
- components: lean/Bumbledb/Capacity.lean, lean/Bumbledb/Values.lean, docs/architecture/30-dependencies.md, crates/bumbledb/src/storage/commit/judgment.rs
- status: open (do not fix)

## Summary
When a parent *is* present, Lean `Value.durationNat` reads a ray as 0 (`measure.getD 0`). `CapacityLaw` then admits or convicts that 0 against the window. The engine never forms that measure: `end == MAX` → `CapacityRayMeasure`, "the law is not judged false; its measure is undefined." Lean records this as unobservable on judged commits; the executable denotation (`Decide` / conformance judgment) still uses the junk-0 reading unless the corpus avoids rays.

## Lean spec
```448:461:lean/Bumbledb/Capacity.lean
/-- The interval measure of a value, junk-total — … a general interval reads
`«end» − start` through `Interval.measure` with the RAY defaulting to
0 (recorded narrowing; ruling C10 makes a ray-valued Duration weight
or bound a typed COMMIT refusal … so the junk value is unobservable on
judged commits — `measure_ray_none` is the law it enforces); …
def Value.durationNat : Value → Nat
  | { type := .interval .u64, val := iv } => iv.measure.getD 0
```

`measure_ray_none` (`Values.lean:258-264`) is `none`, not 0; the capacity layer collapses `none` to 0.

## Normative docs
`30-dependencies.md:256-258`: C10 typed commit refusal, not a 0 weight. `capacity-laws.md` C10/C20: undefined, never silent MAX, never a violation.

## Rust implementation
`judgment.rs` `interval_measure`: `end == u64::MAX` → `Err(CapacityRayMeasure)`, no `end - start`. Naive twin: `Violation::CapacityRayMeasure` before measure folds (`bumbledb-bench/src/naive.rs`).

## Why this matters
A Lean `holds` / `capacityB` run on a ray-weighted instance can accept (weight 0 in window) or reject (floor missed) while the engine refuses with a non-violation error. Conformance judgment cases that include rays must special-case the engine error or they compare junk-0 to a typed abort. Combined with 200, both the absent-parent and present-parent ray cells diverge.

## Related
- 200 (absent parent)
- 218 (error not in 70-api roster)
