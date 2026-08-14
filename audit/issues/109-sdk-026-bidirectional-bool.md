# sdk-026: `==` is two constructors flattened to `bidirectional: bool`

- **Severity:** medium
- **Tree:** sdk (cpp schema + ts statements)
- **Status:** FIXED(bfaf20cb)
- **Source:** audit/sdk-rest.md #4
- **Depends on:** sdk-024 (C++ `statement_data` reshape); TS half is parallel-safe
- **Conflicts with:** sdk-024 (same C++ files)

## The bug

Hosts already have two constructors — TS `contained()` / `mirrors()` (`ts/src/statements.ts:243-277`), C++ `containment_law<S,T,false>` / `mirrors()` → `<S,T,true>` (`cpp/src/schema/contained.cc:28-46`, `cpp/src/schema/mirrors.cc:16`) — and then throw the distinction away in the recorded data:

```typescript
interface ContainmentData {
	readonly kind: "containment"
	readonly bidirectional: boolean
}
```

```cpp
struct statement_data {
	statement_form form;
	side_data source;
	side_data target;
	bool bidirectional;  // spec.cc:163
	// …
};
```

`mirrors()` writes `bidirectional: true` into the same type `contained()` writes `false` into (`statements.ts:247-251,272-276`; `classes.cc:132`). Render re-learns the constructor from the flag (`statements.ts:449` `==` vs `<=`; `classes.cc:193` `mirrors(` vs `contained(`). A `contained` value with `bidirectional == true` leftover faces is representable. raii `owned_containment.bidirectional` (`cpp/foreign/raii.cc:926-932,1184`) echoes the same flag into the ABI byte.

This is sdk-010 (EDB polarity is a sum, interior polarity is a bool), on statements.

The engine `StatementSpec::Containment { bidirectional: bool }` (`crates/bumbledb-theory/src/schema/spec.rs:215-220`) and C ABI `uint8_t bidirectional` are the hostile boundary. They stay (C1 / essential C). They do not license the *dialect recorded* state.

## Why it's wrong

One concept, two encodings, chosen by which layer you look at (Insight 8). The constructors already parsed the sum; the recorded type discarded the proof (Insight 6) and every renderer re-checks the flag (Insight 4).

## The fix

Per `audit/CONTRACT.md §C1` (recorded SDK state is a sum) and §C6's polarity precedent:

- TS recorded data: `kind: "containment" | "mirrors"`, no boolean. `contained()` / `mirrors()` inhabit different arms. `lower()` (`ts/src/lower.ts:85-99`) writes the engine/wire `bidirectional` byte **once**, at the boundary.
- C++: `statement_form` gains `mirrors`, or `statement_data` is a variant; `containment_law<S,T,bool Bidirectional>` may stay as the *call-site* template (sdk-010's ruling). Flattened `bool bidirectional` deletes. `wire.cc` / raii project to the ABI flag at the view.
- Engine `StatementSpec` and C ABI layout **do not change** (C1).

## Acceptance criteria

- [ ] Gone: `rg -n 'bidirectional: boolean' ts/src/statements.ts` → no matches; `rg -n 'bool bidirectional' cpp/src/schema` → no recorded-IR matches (ABI / raii view projection may keep the byte).
- [ ] Unchanged tests: cpp `ctest` + `cd ts && pnpm test` green with zero assertion edits; lowered wire IR identical (`bidirectional: true` still crosses for `mirrors`); fingerprints identical.
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`; `cd ts && pnpm test`.

## Constraints

- Semantics identical, including engine split of `==` into two adjacent containments (source `<=` target first). No Program vocabulary. Coordinate with sdk-024's `statement_data` reshape.
