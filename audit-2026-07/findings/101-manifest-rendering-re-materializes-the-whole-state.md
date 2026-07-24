## Manifest rendering re-materializes the whole statement list per statement — O(N^2) clones

category: perf | severity: low | verdict: CONFIRMED | finder: engine:schema-api

### Summary

`ManifestDescriptor::manifest` already holds the materialized statement list, but the per-statement spelling call `render_declared` recomputes that list from scratch on every invocation — a full re-derivation that deep-clones every declared statement and rebuilds every auto-key — and, for containments, additionally runs a linear `mirror_of` scan over the freshly rebuilt list. One manifest render therefore performs O(N) full statement-list materializations (each itself O(N) allocating clones) plus O(N^2) side comparisons, where a single materialization threaded through would do. `render_rejection` has the same compute-then-recompute shape per violation.

### Evidence (verified against the code)

- `crates/bumbledb/src/schema/manifest.rs:92` — `manifest()` calls `self.materialized_statements()` and iterates it; at `manifest.rs:100`, inside the per-statement map, it calls `super::render::render_declared(self, id)`.
- `crates/bumbledb/src/schema/render.rs:184-185` — `render_declared` opens with `let materialized = descriptor.materialized_statements();`, recomputing the list per call.
- `crates/bumbledb-theory/src/schema.rs:411-446` — `materialized_statements` allocates a fresh `Vec`, walks all relations' sealed fields to mint fresh auto-keys, walks relations again for closed auto-keys, then at line 445 clones every declared statement: `statements.extend(self.statements.iter().cloned())`. `StatementDescriptor`/`Side` (schema.rs:270-313) carry `Box<[FieldId]>` and `Box<[(FieldId, LiteralSet)]>`, so each clone allocates.
- `crates/bumbledb/src/schema/render.rs:201` — per containment, `mirror: super::validate::mirror_of(&materialized, index)`; `crates/bumbledb/src/schema/validate.rs:265-283` confirms `mirror_of` is a linear scan of the whole statement list.
- `crates/bumbledb/src/schema/render.rs:74, 95-96` — `render_rejection` computes `materialized` solely to bounds-check `statement.0` at line 95, then calls `render_declared` at line 96, which re-materializes the list again per violation.

Context checked: `render.rs:7-8` documents "Rendering allocates; it runs only in Display/diagnostic contexts... never on a write or query path" — the cold-path nature is deliberate. `.manifest()` is called only from tests today (`tests/dyn_surface.rs:417`, `tests/schema_macro.rs:402,818`); the manifest is the documented foreign-host bindings boundary (`schema.rs:166-177`, `docs/architecture/70-api.md` § the manifest), rendered once per Theory handoff. `docs/design/representation-first.md`'s lens applies structurally: the materialized list is data the boundary already owns; each callee re-deriving it is control flow standing in for a value that should be passed.

### Bench impact

Cold-path only, no incorrect output: a 200-statement theory's manifest render performs ~201 full list materializations (~200×200 boxed-side clone allocations) plus quadratic mirror scans. Not on any write/query path; matters only at the (per-handoff) bindings boundary and in rejection rendering with many violations. Severity low is correct.

### Suggested fix

Split `render_declared` into the existing public wrapper plus an internal `render_materialized(descriptor: &SchemaDescriptor, materialized: &[StatementDescriptor], id: StatementId) -> String`. `manifest()` (manifest.rs:92) and `render_rejection` (render.rs:74) already hold the materialized list — pass it down. This also retires `render_rejection`'s compute-for-bounds-check-then-recompute at render.rs:74/96. The `mirror_of` scan per containment becomes a scan over the one shared slice; if desired, a single O(N) pre-pass computing all mirrors would drop the quadratic comparisons too, but threading the slice is the essential fix.
