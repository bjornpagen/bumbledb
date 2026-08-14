# sdk-023: C++ `relation_data` is a Program flag-machine — closedness belongs in the type, not a bool beside `closed_info`

- **Severity:** high
- **Tree:** sdk (cpp schema)
- **Status:** FIXED(24d201c9)
- **Source:** audit/sdk-rest.md #1
- **Depends on:** none (schema-lane foundation; co-lands with sdk-024 on `spec.cc`)
- **Conflicts with:** sdk-024, sdk-025 (same files; land per INDEX order)

## The bug

`cpp/src/schema/spec.cc:66-72` — an admitted relation is a sum (ordinary vs closed), but C++ stores both arms on one public aggregate:

```cpp
struct relation_data {
	name_text name;
	std::size_t field_count;
	std::array<field_data, max_relation_fields> fields;
	bool closed;
	closed_info closed_data;
};
```

- `closed == false` still carries a full `closed_info` (`cpp/src/closed/axioms.cc:38-43`: 8 handles + `max_closed_handles * max_closed_columns` axiom slots).
- `closed == true` with a default-empty `closed_data` is well-typed.
- `relation_entry()` (`cpp/src/schema/classes.cc:37`) writes the flag from `is_closed_facade_type`; `schema()` (`cpp/src/schema/schema.cc:233-234`) backfills `closed_data` in a second walk. Two knobs, a flowchart stitching them.
- `wire.cc:140-165` then *parses* the flag into raii's `std::optional<owned_closed>` — the sum the dialect threw away, reconstructed at the ABI projection.

This is sdk-001's `has_rec` + leftover `rec_ir`, on the schema table.

## Why it's wrong

Named fields + a closed flag + an optional axiom roster + a constructor that takes whatever you stuff in IS the old Program state space with the name scraped off (Insight 2 / Insight 4). raii already spends `std::optional<owned_closed>` (`cpp/foreign/raii.cc:901-905`), proving the dialect knows the move — and declined to spend it on the recorded table. TS `RelationSpec.closed: ClosedSpec | undefined` is the kind (R7); C++ left a bool.

## The fix

Per `audit/CONTRACT.md §C1` (every trusted layer after a parse is a sum) and dialect law (`cpp/AGENTS.md` §8 / §27):

- `closed_info` is a member only when the relation is closed (`std::optional` is NTTP-hostile; a `std::conditional_t` empty struct, a parallel `closed_relation_data`, or `if constexpr (is_closed_facade)` so ordinary `relation_data` has no `closed_data` field).
- `bool closed` dies. `wire.cc` reads `if constexpr` / the optional member — no runtime flag.
- Empty `closed_info` on an ordinary relation is unrepresentable.

Engine validator stays the one refusal authority for hostile specs (C1). The dialect just cannot mint the flag mismatch.

## Acceptance criteria

- [ ] Gone: `rg -n 'bool closed' cpp/src/schema/spec.cc` → no matches; `rg -n 'closed_data' cpp/src/schema cpp/src/db/wire.cc` → only the closed arm's member, never beside a flag.
- [ ] Unchanged tests: every existing cpp schema/runtime test green with zero assertion edits; fingerprints identical (closed axioms still cross; only the recorded layout changes).
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`.

## Constraints

- Semantics identical. C ABI `bdb_relation_spec.closed` nullable pointer stays (essential C; sdk-008's ruling). raii `owned_relation.closed` optional is the target spelling, not a second rewrite. No Program vocabulary; no new caps (sdk-025 deletes SDK caps — coordinate, don't add). Locked names untouched.
