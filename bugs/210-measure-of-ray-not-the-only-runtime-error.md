# Docs call MeasureOfRay the one runtime type error; 70-api omits it
- id: 210
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: docs
- components: docs/architecture/10-data-model.md, docs/architecture/20-query-ir.md, docs/architecture/70-api.md, crates/bumbledb/src/error.rs, lean/Bumbledb/Values.lean, lean/Bumbledb/Query/Aggregates.lean
- status: fixed (2026-08-13)

## Summary
Architecture data-model and query-IR docs, and the `Error::MeasureOfRay` comment, call MeasureOfRay "the engine's one runtime type error." The embedding-surface query-error roster (`70-api.md`) lists `Overflow`, `FixpointBudgetExceeded`, and `Corruption` and never names `MeasureOfRay`. Overflow/budget/ResultBytesOverflow are other runtime aborts (range, resource, representation) and do not refute the *type*-error slogan. The slogan is still false of the engine as a whole: `CapacityRayMeasure` is the write-path twin of the same "no finite measure" refusal. Hosts following 70-api will not match on `MeasureOfRay`.

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
Hosts implementing query error handling from 70-api will not match on `MeasureOfRay`. The uniqueness slogan also hides the commit-time twin `CapacityRayMeasure` (218). Overflow and budget remain separate abort classes; they belong on the API roster but do not make MeasureOfRay a "type" error.

## Verification (2026-08-12)
Re-read the type-error slogan, the 70-api roster, and `Error`. **Confirmed**, rewritten narrower than the original "Overflow refutes 'type error'" claim. `wrong-side: docs`.

**Lean** (`lean/Bumbledb/Values.lean:258-264` `measure_ray_none`): ray measure is `none`. Overflow is a separate checked-sum law. Lean does not claim uniqueness of a ray *error constructor*.

**Docs:** `docs/architecture/10-data-model.md:223-226` and `20-query-ir.md:623-628`: “the one runtime type error … boundedness is not provable at validation.” `docs/architecture/70-api.md:832-837` runtime query errors: `Overflow`, `FixpointBudgetExceeded`, `Corruption` only. `rg MeasureOfRay docs/architecture/70-api.md` is empty. `rg CapacityRayMeasure docs/architecture` is empty.

**Rust** (`crates/bumbledb/src/error.rs:1468-1471`): MeasureOfRay repeats “the engine's one runtime type error.” Sibling: `CapacityRayMeasure` (`:1488-1497`). Also `Overflow` (aggregate + `OriginCapacity` `:1238-1246`), `ResultBytesOverflow` (`:1535-1541`), `FixpointBudgetExceeded`.

## Related
- 200, 218 (`CapacityRayMeasure`)
- 206 (`FixpointBudgetExceeded`)

## Resolution (2026-08-13)
`70-api.md` query roster lists `MeasureOfRay`; write roster lists `CapacityRayMeasure`. Measure *find* (Duration projection) remains; that is not ArgMax.
