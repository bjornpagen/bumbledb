# 70-api write-error roster omits CapacityRayMeasure
- id: 218
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: docs
- components: docs/architecture/70-api.md, crates/bumbledb/src/error.rs, docs/design/capacity-laws.md, crates/bumbledb/src/storage/commit/judgment.rs
- status: fixed (2026-08-13)

## Summary
The embedding-surface architecture doc lists write errors as `CommitRejected`, `GenerationMoved`, `ForeignSnapshot`, `FreshExhausted`, `FactShape`, `Corruption`, `Io`/`Lmdb`. A Duration-weighted capacity insert of a ray fails with `Error::CapacityRayMeasure` — not a violation set, not `CommitRejected`. Lean has no such constructor (junk-0 / empty-parent vacuity). Hosts following 70-api will not handle the typed refusal C10/C20 made load-bearing.

## Lean spec
Silent. `Txn.WriteResult` is `ok | violations | generationMoved`. Capacity rays are not a write-result constructor (`Capacity.lean` junk-0 + C10 named as engine mechanism).

## Normative docs
`70-api.md:838-849` write-error roster: no `CapacityRayMeasure`. `rg CapacityRayMeasure docs/architecture` is empty except the 30-dependencies C10 sentence (`:256-258`), which does not name the error type. Design doc `capacity-laws.md:411-423` names the engine pin.

## Rust implementation
`error.rs:1488-1497` `CapacityRayMeasure { statement, fact }`: "never a violation (the law is not judged false; its measure is undefined)." Raised from `interval_measure` / `child_weight` at plan or judge. Distinct from `CommitRejected`.

## Why this matters
Error-handling code generated from 70-api will treat `CapacityRayMeasure` as an unexpected `Error` variant (or map it to Corruption). The distinction "undefined measure vs false law" is the C10/C20 design; the public API doc drops it.

## Verification (2026-08-12)
Re-read the 70-api write roster, C10/C20 docs, and `Error::CapacityRayMeasure`. **Confirmed.** Stronger than filed: `rg CapacityRayMeasure docs/architecture` is empty (C10 in `30-dependencies.md:256-258` does not name the constructor). `wrong-side: docs`. Lean is silent (no such write-result constructor).

**Lean:** Silent. `Txn.WriteResult` is `ok | violations | generationMoved`. Capacity rays are junk-0 / C10-as-mechanism (`Capacity.lean:87-92`).

**Docs:** `docs/architecture/70-api.md:838-849` write errors: `CommitRejected`, `GenerationMoved`, `ForeignSnapshot`, `FreshExhausted`, `FactShape`, `Corruption`, `Io`/`Lmdb`. Design record `docs/design/capacity-laws.md:411-423` names the engine pin, not the public API type.

**Rust** (`crates/bumbledb/src/error.rs:1488-1497`): `CapacityRayMeasure { statement, fact }` — “never a violation (the law is not judged false; its measure is undefined).” Raised from `interval_measure` (`judgment.rs:228-240`) at plan or judge.

## Related
- 200 (C20 behavioral split)
- 210 (runtime roster also incomplete)

## Resolution (2026-08-13)
`70-api.md` write-error roster includes `CapacityRayMeasure` (C10 judge-time, C20 parent-blind).
