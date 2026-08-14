# sdk-021: `query_view` pads empty interiors with a dummy length-1 array

- **Severity:** low
- **Tree:** sdk (cpp foreign)
- **Status:** FIXED(54f6fb9f)
- **Source:** audit/sdks.md #21
- **Depends on:** none (two-line fix; parallel-safe)

## The bug

`cpp/foreign/query_view.cc:443-444, 530`:

```cpp
std::array<bdb_interior, Query.interiors.size() == 0 ? 1 : Query.interiors.size()>
// ...
.interiors = size == 0 ? nullptr : data(),
```

Empty is spelled as "one dummy slot we promise not to point at."

## Why it's wrong

Dijkstra's half-open interval (Insight 7): the empty range is not a special case needing a phantom element — `std::array<T, 0>` is legal and `.data()` on it is fine to not dereference. The `?: 1` manufactures storage whose only job is to exist unread.

## The fix

`std::array<bdb_interior, Query.interiors.size()>` — zero-length when empty; `.interiors = Query.interiors.size() == 0 ? nullptr : arr.data()` (or `arr.data()` unconditionally if the ABI tolerates non-null with count 0 — match the header's documented convention; `bumbledb_c.h` pointer/count pairs elsewhere say count is the authority).

## Acceptance criteria

- [ ] Gone: `rg -n '\? 1 :' cpp/foreign/query_view.cc` → no matches.
- [ ] Unchanged tests: cpp `ctest` + bridge `cargo test` green with zero assertion edits.
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`; `cd cpp/bridge && PATH="$HOME/.cargo/bin:$PATH" cargo test`.

## Constraints

- ABI contract unchanged (count 0 already means "no interiors"). Pure representation cleanup.
