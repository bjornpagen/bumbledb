# sdk-21: `query_view` empty-interiors dummy array of length 1

Severity: low
Tree: sdk (cpp)
Status: OPEN
Source: audit/sdks.md #21
Blocked-by: none
Blocks: none

## Bug

`cpp/foreign/query_view.cc:443-444,530`:
`std::array<bdb_interior, size == 0 ? 1 : size>` then
`.interiors = size==0 ? nullptr : data()` — a dummy slot we promise
not to point at.

## Fix

Cites CONTRACT C6: `std::array<T, 0>` is legal — use `N = size`,
`.data()` (or nullptr) for count 0. No `?: 1`.

## Acceptance criteria

- [ ] Grep `\? 1 :` over `cpp/foreign/query_view.cc` returns empty.
- [ ] Empty-interiors queries prepare and execute unchanged (bridge
      tests green).

## Constraints

ABI values identical (`interiors = nullptr, count = 0` stays).
