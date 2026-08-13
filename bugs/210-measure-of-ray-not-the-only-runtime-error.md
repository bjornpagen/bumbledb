# Docs call MeasureOfRay the one runtime type error; 70-api omits it
- id: 210
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: docs
- components: docs/architecture/10-data-model.md, docs/architecture/20-query-ir.md, docs/architecture/70-api.md, crates/bumbledb/src/error.rs, lean/Bumbledb/Values.lean, lean/Bumbledb/Query/Aggregates.lean
- status: open (do not fix)

## Summary
Architecture data-model and query-IR docs, and the `Error::MeasureOfRay` comment, call MeasureOfRay "the engine's one runtime type error." The embedding-surface error roster (`70-api.md` runtime query errors) lists `Overflow`, `FixpointBudgetExceeded`, and `Corruption` — and does not mention `MeasureOfRay` at all. Well-typed queries also raise `Overflow` (aggregate and origin) and `ResultBytesOverflow`; writes raise `CapacityRayMeasure`. The "one" claim is false, and the API roster is incomplete.

## Lean spec
`measure_ray_none` (`Values.lean:258-264`) and `measure_fold_laws` (`Aggregates.lean`) model the ray as `Option.none` / group poison. Overflow is a separate typed error (`checkedSum_sound`). Fixpoint budget is unmodeled (see 206). Lean does not claim uniqueness of the ray error.

## Normative docs
```223:226:docs/architecture/10-data-model.md
  error `MeasureOfRay` — the one runtime type error in the engine, since
  boundedness is not provable at validation.
```

`20-query-ir.md:628`: "the engine's one runtime type error."

`70-api.md:832-837` runtime query errors: `Overflow`, `FixpointBudgetExceeded`, `Corruption` only. `rg MeasureOfRay docs/architecture/70-api.md` is empty. Write-error roster (`:838-849`) omits `CapacityRayMeasure`.

## Rust implementation
`Error::MeasureOfRay` (`error.rs:1470+`) repeats "one runtime type error." Also: `Error::Overflow` (aggregate finalize, origin capacity), `Error::ResultBytesOverflow`, `Error::FixpointBudgetExceeded`, `Error::CapacityRayMeasure`. All abort a well-typed execution or commit.

## Why this matters
Hosts implementing error handling from 70-api will not match on `MeasureOfRay` or `CapacityRayMeasure`. Callers taught there is a single runtime type error will mishandle Overflow and budget aborts. The uniqueness slogan contradicts the engine's own error enum.

## Related
- 200, 218 (`CapacityRayMeasure`)
- 206 (`FixpointBudgetExceeded`)
