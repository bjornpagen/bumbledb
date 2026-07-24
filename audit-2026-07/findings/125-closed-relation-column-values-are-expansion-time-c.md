## Closed-relation column values are expansion-time constants but only readable through runtime queries

category: missing-free-feature | severity: low | verdict: CONFIRMED | finder: macros:core

### Summary

The `schema!` macro projects each closed relation into a host enum with const `id()`/`from_id()` welds on the doctrine "one vocabulary, two checkers, zero drift" — but the projection stops at the handle. The declared columns' literals are ground axioms fully typed at expansion, yet the only host-side way to read `Kind::DirectPass`'s `mastered` column is a runtime db query (or a hand-duplicated table that can silently drift). The same emission loop that builds the id welds already holds the typed values and could emit `const fn mastered(self) -> bool` per column for free, extending exactly the projection-for-rustc argument the host enum itself is justified by.

### Evidence

- `crates/bumbledb-macros/src/lib.rs:2158-2222` (`emit_closed`): the per-handle loop (2177-2186) builds `id_arms`/`from_arms`/`weld` from `row.handle` only; `extension.rows[*].values` is in scope and never emitted. The output (2187-2218) is the enum, `const fn id`, `const fn from_id`, and the weld test — nothing else.
- `crates/bumbledb-macros/src/lib.rs:207-211` (`ClosedRow`): rows already carry `(column, Literal)` pairs reordered to declaration order at parse, coverage-checked (every declared column exactly once).
- `crates/bumbledb-macros/src/lib.rs:1376-1404` (`lower_relations`): every row's every column goes through `typed_literal` at expansion — the values are typed compile-time constants the macro holds in hand.
- `docs/architecture/70-api.md:92-107` (§ closed-relation emission — the spec for this code): records the host enum, the id/from_id weld, and the weld test; says "The host enum is the constant namespace — no separate per-handle constants exist." The "reads go through queries and the dyn surface" sentence belongs to the fact-struct/writability refusal ("a writable struct would be a lie the type system tells"), not to a ruling on column projection. The docs record refusals explicitly elsewhere ("the recorded refusal" pattern, e.g. nested-closed-refs, rays, open extensions in `10-data-model.md:353-418`); no refusal of host column accessors is recorded anywhere.
- `docs/architecture/10-data-model.md:353-418`: ground-axiom columns are "constants at every instance" (`den_closed_constant`), and the intrinsic-vs-policy law makes intrinsic columns part of the theory — so host code branching on `mastered` is branching on a compile-time theory constant.
- Real schemas hit this today: `crates/bumbledb-query/tests/cookbook.rs:179-186` (r07: `Kind { mastered: bool, rank: u64 }`) and `:203-210` (r08: `Severity { pages: bool }`). Inside the query IR the engine handles these fine (`Kind(id: k, mastered == true)`); the gap is host-side Rust branching outside queries.
- Kind coverage correction to the original finding: `str` intrinsic columns are refused on closed relations (`docs/architecture/10-data-model.md:384-387` — "the handle IS the label"), so every column kind that can legally occur (bool, u64, i64, `bytes<N>`, interval) is const-representable; no partial-coverage carve-out is needed.

### Failure scenario

Schema declares `closed relation Kind as KindId { mastered: bool } = { DirectPass { mastered: true }, Failed { mastered: false } }` (the exact shape of cookbook r07 and the spec's own example at `70-api.md:72-77`). Host code that must branch on mastered-ness outside a query either (a) opens a read transaction and runs a query per call to read a value that was constant at expansion, or (b) writes `matches!(kind, Kind::DirectPass)` by hand — a duplicated extension table with no weld test, which silently drifts when a new handle (`JudgedPass { mastered: true }`) is added to the schema. Both outcomes are what the emitted weld machinery exists to prevent for ids.

### Suggested fix

In `emit_closed`, per declared column, emit a const accessor on the host enum in the same style as `id()` — an explicit match per handle, values rendered from the already-typed `ClosedRow.values` (newtyped columns return the newtype, exactly as `rust_field_ty` already decides). Because the accessor is emitted from the same parsed literals that seed the engine's extension (`lower_relations`), host and engine cannot drift by construction — no query-backed weld extension is required, keeping the emitted weld test db-free.
