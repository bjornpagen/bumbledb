# PRD 10: Remove Legacy COLT Structures

## Purpose

Delete the old heap-shaped COLT implementation after arena COLT is behaviorally equivalent.

## Required Deletions

- Delete `Rc<RefCell<ColtNode>>` from production COLT code.
- Delete `HashMap<EncodedTuple, Rc<...>>` from production COLT code.
- Delete per-child `Vec<usize>` storage from production COLT code.
- Delete compatibility wrappers that preserve old COLT behavior.
- Delete tests that rely on internal map iteration order.

## Passing Criteria

- `rg "Rc<RefCell|HashMap<EncodedTuple|Vec<usize>" crates/bumbledb-lmdb/src/colt*` returns no production hot-path matches. Test fixtures may use `Vec` only for expected data setup.
- All COLT tests pass against the arena implementation.
- All executor tests pass.
- q09 and broad exact SQLite comparisons pass.
- No public API changes.
- Global gates pass.
