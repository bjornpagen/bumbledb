# Schema fingerprint bytes are engine/docs law; Lean has no hash of a theory
- id: 223
- severity: low
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: unspecified
- components: lean/Bumbledb/Schema.lean, crates/bumbledb/src/schema/fingerprint.rs, docs/architecture/10-data-model.md, docs/architecture/70-api.md, docs/architecture/75-cpp-lowering.md
- status: fixed (2026-08-13)

## Summary
Open/create identity is blake3 of canonical descriptor bytes (`bumbledb-schema-v5`), including materialized statement order and C2 capacity field order. Lean `Theory` has no fingerprint; C2 order is cited in `Statement.capacity` comments only. Cross-host "byte-exact fingerprint parity" (`75-cpp-lowering.md`) is an engine obligation with no Lean theorem.

## Lean spec
`Schema.lean` `Theory` = header + closed map + statement list. No blake3, no version label, no canonical byte string. `Statement.capacity` doc (`:511-514`) pins operator order (target, weight, window, source) as C2 — syntax order, not a hash.

## Normative docs
`10-data-model.md` fingerprint inputs; `50-storage.md` open-time `SchemaMismatch`; `75-cpp-lowering.md:1-20`: C++ and TS must lower to the identical fingerprint via `SchemaSpec::descriptor()`.

## Rust implementation
`fingerprint.rs:39-46`: `FORMAT_VERSION_LABEL = b"bumbledb-schema-v5"`; blake3 of canonical bytes; enforcement plans and mirror links not hashed (`:10-16`).

## Why this matters
A Lean-equal theory (same `holds`) can be a `SchemaMismatch` if statement order, closed-row order, or C2 encoding differs. Frontends that "match the spec" without matching `fingerprint.rs` cannot open each other's stores. Lean cannot prove the parity `75-cpp-lowering.md` claims.

## Verification (2026-08-12)
Re-read `Theory`, fingerprint inputs, and `fingerprint.rs`. **Confirmed.** `wrong-side: unspecified` stays: Lean has no hash; docs and Rust agree on blake3 v5. Original citation of `50-storage.md` for `SchemaMismatch` is the wrong file — open-time mismatch lives in `70-api.md:815` and `10-data-model.md:575-578`.

**Lean** (`lean/Bumbledb/Schema.lean:521-534`): `Theory` = header + closed map + statement list. No blake3, no version label. `Statement.capacity` (`:511-514`) pins C2 operator order as syntax, not a hash.

**Docs:** `docs/architecture/10-data-model.md:575-584` fingerprint inputs; `70-api.md:815` `SchemaMismatch`; `75-cpp-lowering.md:1-7` byte-exact fingerprint parity via `SchemaSpec::descriptor()`.

**Rust** (`crates/bumbledb/src/schema/fingerprint.rs:10-16`, `:40`): `FORMAT_VERSION_LABEL = b"bumbledb-schema-v5"`; blake3 of canonical bytes; enforcement plans and mirror links not hashed.

## Related
- 207 (closed-target acceptance also differs while theories look equal)

## Resolution (2026-08-13)
Schema fingerprint documented as extra-theoretic engine identity (`Theory` has no hash) in `Schema.lean`, `10-data-model.md`, and `75-cpp-lowering.md`. No blake3 in Lean.
