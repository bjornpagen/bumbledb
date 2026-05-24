# PRD 02: COLT Arena Data Model

## Purpose

Introduce the arena-backed COLT data model without cutting the runtime over yet.

## Required Design

Add a new private module for arena COLT state. Exact names may differ, but the model must include these concepts:

```rust
struct ColtArena { ... }
struct ColtSourceId(u32);
struct ColtNodeId(u32);
struct ColtMapId(u32);
struct OffsetRange { start: u32, len: u32 }

enum NodeData {
    Range { start: u32, len: u32 },
    Singleton { offset: u32 },
    Offsets(OffsetRange),
    Map(ColtMapId),
}
```

## Hard Constraints

- No `Rc` in the new arena model.
- No `RefCell` in the new arena model.
- No per-node `Vec<usize>` in the new arena model.
- No `HashMap<EncodedTuple, Rc<...>>` in the new arena model.
- Arena indexes must be compact integer IDs.
- Offset storage must be append-only or range-addressed.

## Required Tests

- Create range, singleton, offset-range, and map placeholder nodes.
- Verify node IDs remain stable after later insertions.
- Verify offset ranges read back exact offsets.
- Verify empty ranges and singleton ranges behave distinctly.

## Passing Criteria

- New arena module compiles but is not yet used by production query execution.
- Tests cover all `NodeData` variants.
- Grep under the new arena module shows no `Rc<`, `RefCell`, or `HashMap<EncodedTuple`.
- Global gates pass.
