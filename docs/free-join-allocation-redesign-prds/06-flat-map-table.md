# PRD 06: Flat Map Table

## Purpose

Replace `HashMap<EncodedTuple, Rc<RefCell<ColtNode>>>` with an arena-owned flat map table.

## Required Design

Forced map state must use compact arena ranges:

```rust
struct MapTable {
    buckets: OffsetRange,
    entries: OffsetRange,
}
struct MapEntry {
    hash: u64,
    key: KeyOwned,
    child: ColtNodeId,
    next: u32,
}
```

Exact collision strategy may differ. It must be deterministic for exact result semantics or tests must sort public outputs only.

## Required Work

- Add map table storage to `ColtArena`.
- Add insertion and lookup by `KeyRef`.
- Store child node IDs, not heap node pointers.
- Use one or two contiguous allocations per forced map, not one allocation per map entry.

## Passing Criteria

- Grep under COLT arena path shows no `HashMap<EncodedTuple`.
- A forced map with many duplicate input offsets creates entries proportional to distinct keys.
- Lookup by borrowed key allocates zero heap objects for inline-width keys.
- Map construction allocates less than the current HashMap path in a focused fixture.
- Global gates pass.
