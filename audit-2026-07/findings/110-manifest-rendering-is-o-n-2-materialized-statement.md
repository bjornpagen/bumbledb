## Manifest rendering is O(n^2): materialized statements re-cloned per statement

category: perf | severity: low | verdict: CONFIRMED | finder: lean:schema-values

### Summary

`ManifestDescriptor::manifest` renders the canonical spelling of every materialized statement by calling `render_declared(self, id)` once per statement, and `render_declared` re-runs `SchemaDescriptor::materialized_statements()` from scratch on every call — rebuilding and re-cloning the entire statement vector n times, plus an O(n) `mirror_of` scan per containment. One `manifest()` call is therefore O(n²) statement clones (each clone allocating its `Box<[FieldId]>` projections and selection literal values) where a single upfront materialization threaded into the renderer is linear.

### Evidence (all verified against the code)

- `crates/bumbledb/src/schema/manifest.rs:91-101` — `manifest()` materializes once for the outer enumeration, then per entry calls `spelling: super::render::render_declared(self, id)`.
- `crates/bumbledb/src/schema/render.rs:184-187` — `render_declared` opens with `let materialized = descriptor.materialized_statements();` on every call.
- `crates/bumbledb-theory/src/schema.rs:411-447` — `materialized_statements` builds a fresh `Vec` each call: walks every relation's sealed fields to mint fresh auto-keys, every relation again for closed auto-keys, then `statements.extend(self.statements.iter().cloned())` (line 445) clones every declared statement — projections, selections, and their `Value` literals.
- `crates/bumbledb/src/schema/render.rs:201` + `crates/bumbledb/src/schema/validate.rs:265-282` — each containment additionally runs `mirror_of(&materialized, index)`, a linear scan over the whole materialized list, because a rejected declaration seals no `mirror` field to read.
- Bound: `crates/bumbledb/src/error.rs:179` — `SchemaError::TooManyStatements` caps materialized statements at the u16 id space (65,536), so multi-thousand-statement dynamic schemas are legal inputs.

The diagnostic excuse does not apply here: `render.rs:7-8` scopes rendering's allocations to "Display/diagnostic contexts … never on a write or query path", but `docs/architecture/70-api.md` § "Id constants and the manifest" defines `Theory::manifest()` as the advertised runtime surface — the statement table with canonical spellings is exactly what a foreign host takes as data. The same re-materialize-per-call pattern also sits in `render_rejection` (render.rs:74 materializes once, then line 96 calls `render_declared` which re-materializes per violation), though violation lists are small in practice.

### Bench impact

A dynamic `SchemaSpec`/ETL schema with a few thousand statements — well inside the 65,536 cap — pays roughly n² statement-descriptor clones plus n² mirror comparisons for one `manifest()` call: at 2,000 statements that is ~4M statement clones with their per-clone heap allocations, at 10,000 it is ~100M. It runs once per host bootstrap, not per query, and macro-declared schemas number tens of statements, so severity is low — but the manifest lane of any host-bindings benchmark over a large dynamic schema improves from quadratic to linear with a one-line-shaped fix.

### Suggested fix

Materialize once and thread the list: add a renderer entry that takes the already-materialized slice — e.g. `render_materialized(names: &DeclaredNames, materialized: &[StatementDescriptor], index: usize) -> String` — and have both `manifest()` (manifest.rs:91) and `render_rejection` (render.rs:74) pass the single vector they already built; `render_declared` becomes a thin wrapper that materializes and delegates, preserving its descriptor-only contract (render.rs:55-57: pure over the descriptor, no database handle) for the schema-error diagnostic path. Optionally hoist the per-containment `mirror_of` scan into one O(n) pre-pass over the materialized list, though the clone elimination is the dominant win.
