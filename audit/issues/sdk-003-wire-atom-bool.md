# sdk-003: C++ `wire_atom` stores both ids plus a bool instead of `AtomSource`

- **Severity:** high
- **Tree:** sdk (cpp dialect)
- **Status:** OPEN
- **Source:** audit/sdks.md #3
- **Depends on:** none (parallel-safe within cpp; textual overlap with sdk-001/002 in lower.cc/query_view.cc)

## The bug

Engine `AtomSource = Edb(RelationId) | Interior(InteriorId)` — one tag, one payload. C++ `wire_atom` (`cpp/src/query/ir.cc:259-270`) stores both payloads plus a flag:

```cpp
struct wire_atom {
    std::uint32_t relation;
    bool interior;
    std::uint32_t interior_id;
    ...
};
```

Lowering (`cpp/src/query/lower.cc:161-163`) writes `out.interior = true; out.interior_id = id` and leaves `relation` as 0. `query_view` (`cpp/foreign/query_view.cc:292-311`) re-derives the C ABI tag from the flag while stuffing BOTH fields into `bdb_atom`. `interior == true` with a stale `relation`, or `interior == false` with a stale `interior_id`, are representable states.

## Why it's wrong

The C ABI already has `bdb_atom_source_kind` — a real tag. The dialect invented a WORSE encoding upstream of it and translates at the boundary (Insight 4: a bool plus two payloads spells states the domain doesn't have; Insight 2: the flag encoding is the pre-cut shape). Dialect law (`cpp/AGENTS.md` §8) makes `std::variant` the blessed closed sum; a manual discriminator + payload struct is on the forbidden list verbatim.

## The fix

Per `audit/CONTRACT.md §C6` (C++): `wire_atom`'s source is one sum with one payload. Options in preference order: (a) a `std::variant<edb_source, interior_source>` if NTTP-trivial on the pinned toolchain; (b) the C ABI's own `bdb_atom_source_kind` tag + a single `std::uint32_t id` field — one tag, one payload, no stale twin. Lowering writes the tagged value once; `query_view` copies tag+id through without re-derivation.

## Acceptance criteria

- [ ] Gone: `rg -n 'bool interior;' cpp/src` → no matches; no struct in `cpp/src` carries both `relation` and `interior_id` as sibling fields (`rg -n 'interior_id' cpp/src/query/ir.cc` shows the single-payload shape only).
- [ ] Unchanged tests: cpp `ctest` green with zero assertion edits; `bdb_atom` bytes emitted for existing queries identical.
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`; `cd cpp/bridge && PATH="$HOME/.cargo/bin:$PATH" cargo test`.

## Constraints

- The C ABI `bdb_atom` layout (both fields + kind) is ESSENTIAL C and does not change — this issue is the dialect-side encoding only (sdk-008 owns bridge-side parsing). No Program vocabulary.
