## SchemaError::statement() is a 44-line variant roster that a two-level representation would erase

category: inappropriate-branching | severity: low | verdict: CONFIRMED | finder: engine:interval-allen
outcome: fixed 99f157dd

### Summary

`SchemaError` (crates/bumbledb/src/error.rs:132-440) has 40 variants that partition cleanly in two: 15 declaration-scoped variants that carry no statement id, and 25 statement-scoped variants that every one begins with `statement: StatementId`. That partition exists in the code only as comment banners (`// --- Statement roster (30-dependencies § validation roster) ---`, error.rs:243) and as a hand-maintained 44-line exhaustive match in `SchemaError::statement()` (crates/bumbledb/src/error/display.rs:1070-1113) that lists every variant by name into a `None` arm or a `Some(*statement)` arm. A representation that reifies the partition — declaration variants beside one `Statement { statement: StatementId, kind: StatementErrorKind }` arm — makes `statement()` a field access, makes the `display_with` statement-citation path total by construction, and turns the comment banners into types. This is the project's own doctrine (representation over control flow; make illegal states unrepresentable) applied to its own error type — the enum's doc comment (error.rs:124-130) even celebrates that two roster lines are "unrepresentable rather than rejected", while the statement/declaration split itself stays comment-level.

### Evidence

All citations verified against the working tree at HEAD (89086d4f):

- crates/bumbledb/src/error/display.rs:1070-1113 — `fn statement(&self) -> Option<StatementId>`: a two-arm match; the None arm ORs 15 variants (`Self::DuplicateRelationName { .. } | … | Self::FreshOnClosedRelation { .. } => None`), the Some arm ORs 25 (`Self::StatementUnknownRelation { statement, .. } | … | Self::CardinalityIntervalPosition { statement, .. } => Some(*statement)`). Both arms use `{ .. }` field-ignoring patterns, so variant placement is unchecked by the compiler beyond mere exhaustiveness.
- crates/bumbledb/src/error.rs:243 — the partition as a comment: `// --- Statement roster (30-dependencies § validation roster) ---`; every variant from `StatementUnknownRelation` (error.rs:246) through `DuplicateStatement` (error.rs:436) repeats `statement: StatementId` as its first field — shared structure the flat enum erases.
- crates/bumbledb/src/error/display.rs:1134-1146 — `SchemaDisplayWith::fmt` branches on `self.error.statement()`; the `Some` path appends `— in \`{rendered}\`` via `render::render_declared`. This is the only consumer of `statement()`, and the value of the whole adapter: the None path is just plain `Display`.
- Test coverage: exactly one test exercises the citation path — `schema_error_diagnostics_render_the_offending_statement` (crates/bumbledb/src/schema/render/tests.rs:220), covering `NoMatchingTargetKey` alone. No test enumerates variants against the adapter; the reject-suite (schema/tests/reject.rs) asserts variants, not their `display_with` rendering.
- Spec check: docs/architecture/30-dependencies.md § "Validation roster (statements; exhaustive)" requires "one variant per roster line, no catch-all" (restated at error.rs:124-127). The proposed `StatementErrorKind` preserves this — each roster line remains a distinct kind variant; only the shared `statement` field is factored out, so the refactor does not diverge from the spec.
- Non-refuting sibling: `Violation` (error.rs:933) also flattens a shared `statement` field across variants, but its `statement()` (error.rs:979) is total — no `Option`, no misplacement hazard — so it does not license the SchemaError shape.

### Failure scenario

A future statement-scoped variant (the roster doc explicitly reserves lifting triggers, e.g. the `CardinalityIntervalPosition` and `ClosedContainmentInterval` v0 refusals both name conditions under which new statement shapes arrive) is added to the enum. The exhaustive match in `statement()` forces the author to touch it — but adding `Self::NewVariant { .. }` to the None arm compiles identically to adding it to the Some arm. The result ships with `display_with` silently omitting the rendered `schema!` citation for that variant: no compile error, no test failure, a degraded diagnostic that only a user filing a confusing-error report would surface.

### Suggested fix

Split the enum in two levels: keep the 15 declaration-scoped variants at the top level and collapse the 25 statement-scoped ones into one arm, `SchemaError::Statement { statement: StatementId, kind: StatementErrorKind }`, where `StatementErrorKind` carries today's per-roster-line payloads minus the id (one kind variant per roster line, preserving the 30-dependencies "no catch-all" mandate). `statement()` becomes a one-arm match (or disappears into a field access at the `SchemaDisplayWith` call site), the error.rs comment banners become types, and misplacing a future statement variant becomes unrepresentable rather than untested.
