# sdk-025: schema sugar caps — `max_closed_handles = 8` is the `max_query_rules` that survived onto the schema

- **Severity:** medium
- **Tree:** sdk (cpp schema + closed)
- **Status:** FIXED(ab28ac2d)
- **Source:** audit/sdk-rest.md #3
- **Depends on:** sdk-023 (a sum may change what needs a cap at all)
- **Conflicts with:** sdk-023, sdk-024 (same files; land after or with them)

## The bug

`cpp/src/schema/spec.cc:18-33` and `cpp/src/closed/axioms.cc:13-18`:

```cpp
inline constexpr std::size_t max_projection_width = 8;
inline constexpr std::size_t max_relation_fields = 16;
inline constexpr std::size_t max_face_selections = 4;
inline constexpr std::size_t max_selection_literals = 4;
inline constexpr std::size_t max_closed_handles = 8;
inline constexpr std::size_t max_closed_columns = 4;
```

Engine: `MAX_EXTENSION_ROWS = 256` (`crates/bumbledb-theory/src/schema.rs:400`), `MAX_DETERMINANT_WIDTH = 496` bytes, **no** SDK field-count cap of 16, **no** face-selection cap of 4. Comments confess "Phase-C capacity" / "Phase-F capacity; the engine's bound is far higher" — sdk-012's "SDK bounds only" sentence, on the schema table. Constexpr traps (`relation_exceeds_max_relation_fields` at `classes.cc:22,44-45`; `face_has_too_many_selection_bindings` at `where.cc:18,154-155`; `static_assert(… <= max_projection_width)` at `key.cc:59`, `face.cc:38,53`) are flowcharts on invented sizes.

`max_relation_fields` also bounds query-IR atom bindings (`cpp/src/query/ir.cc:81,269`) — one invented schema cap leaked into a second coordinate.

## Why it's wrong

A number invented by the SDK is a fact about nothing (Insight 10: magic sizes are a second, undocumented theory the engine will disagree with). A closed vocabulary of 9 handles is legal at the engine and unwritable in the dialect — the wall is a sugar fiction, not a denotation.

## The fix

Per `audit/CONTRACT.md §C6` (SDK-invented caps die; the engine's number is the one number) applied to the schema lane:

- Import/mirror `MAX_EXTENSION_ROWS = 256` as the closed-handle array bound, cited to the engine constant by name; delete `max_closed_handles = 8`.
- Projection width: either the engine's determinant-width constraint or a pack-length / template count (the face's `width` IS the bound, like query `NI`). Delete `max_projection_width = 8`.
- `max_relation_fields` / `max_face_selections` / `max_selection_literals` / `max_closed_columns`: pack length or engine caps where they exist; no invented 4/16.
- Query-IR bindings stop importing the schema cap (sdk-012 owns the query-side rewrite; this issue deletes the schema-side number they share, so coordinate).

## Acceptance criteria

- [ ] Gone: `rg -n 'max_closed_handles = 8|max_projection_width = 8|max_face_selections = 4|max_relation_fields = 16' cpp/src` → no matches (or the remaining number is the engine's, cited).
- [ ] Unchanged tests: cpp `ctest` green; existing schemas (all ≤ old caps) lower identically; fingerprints unchanged.
- [ ] New lock: a compile-success fixture with >8 closed handles (legal per engine, unwritable before this fix), or a documented engine-cap fixture if the bound becomes 256.
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`.

## Constraints

- Semantics identical. No new caps. Coordinate with sdk-012 (query caps) so `max_relation_fields` is not deleted here and re-invented there. Corpus / fingerprints / locked names untouched.
