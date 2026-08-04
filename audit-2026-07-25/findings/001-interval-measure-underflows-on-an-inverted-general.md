## interval_measure underflows on an inverted general-tail interval — panic in debug, garbage measure in release

bug | high | CONFIRMED | capacity-judge
outcome: fixed c9a78a67

### Summary

`interval_measure` (crates/bumbledb/src/storage/commit/judgment.rs:164-181) returns `Ok(end - start)` after only a ray guard (`end == u64::MAX`). For a GENERAL interval tail, `IntervalTail::words` (crates/bumbledb/src/schema.rs:214-229) parses any 16 bytes into raw `(start, end)` words with no ordering validation — only the fixed-width arm validates (via `decode_fixed_interval_start`). A stored fact whose general interval is inverted (start > end, end != MAX — one flipped byte) therefore reaches an unchecked u64 subtraction: debug/test builds panic ("attempt to subtract with overflow"); release builds (no `[profile]` overflow-checks override anywhere in the workspace Cargo.toml) wrap to a huge bogus measure. **Reproduced**: a temporary test (Duration-weighted capacity over a general `interval<u64>` field, valid commit, then raw-swapping the interval's start/end halves in the stored F value) panics inside `db.verify_store()` at `judgment.rs:181:8`.

### Evidence (verified)

- crates/bumbledb/src/storage/commit/judgment.rs:175-181 — `if end == u64::MAX { return Err(Error::CapacityRayMeasure {...}) } Ok(end - start)`. No inversion guard.
- crates/bumbledb/src/schema.rs:218-223 — general arm of `words`: two raw `u64::from_be_bytes` reads, `Some((start, end))` unconditionally. Fixed arm (224-227) validates.
- General intervals are legal Duration weights/bounds: crates/bumbledb/src/schema/validate.rs:867-877 (weight) and 900-913 (bound) mint `IntervalTail { width }` straight from `ValueType::Interval { width }`; `width: None` is the general 16-byte family (crates/bumbledb-theory/src/schema.rs:101-106).
- Three live corrupt-content paths into the subtraction:
  1. verify_store F pass: crates/bumbledb/src/verify_store/facts.rs:229 (`check_marks` → `expected_slot_weight` → `child_weight`, judgment.rs:147-156 and 128-138);
  2. verify_store R→F pass: crates/bumbledb/src/verify_store/reverse.rs:124;
  3. COMMIT path: `Checker::resolve_hi` judgment.rs:1199-1210 (`TargetDuration` bound read off a stored holder fact fetched at judgment.rs:1130).
- The corrupt fact is NOT filtered before reaching path 1: `decode_field` does flag inversion (`InvalidInterval` when `start >= end`, crates/bumbledb/src/encoding/decode.rs:40-46), but the F pass explicitly keeps walking after a finding (facts.rs:62-81) and then calls `check_marks` on the same fact bytes (facts.rs:176-184).
- Contract breach: verify_store's own adjacent tests are named `a_foreign_relation_capacity_edge_is_convicted_never_a_panic` and `a_wrong_width_capacity_child_is_convicted_never_a_panic` (crates/bumbledb/src/verify_store/tests.rs:1795, 1822 — convict-never-panic doctrine), and commit's doc promises panics "only on programmer-invariant violations" with typed `Corruption` for storage corruption (crates/bumbledb/src/storage/commit/write.rs:70-83).
- Both sweeper call sites already absorb a typed error correctly (facts.rs:239 `Err(_) => malformed`; reverse.rs matches on `Ok`), so a checked-sub fix slots in without new plumbing.

### Failure scenario / impact

Flip/swap bytes of a Duration-weighted child's general interval so start > end (end != MAX), then `db.verify_store()`: debug/test builds panic at judgment.rs:181 instead of reporting a finding (reproduced verbatim); release builds derive a wrapped ~2^64 expected weight — a spurious `ReverseEdgeWeightDesync`, or silent agreement with an equally-corrupt slot. The same corruption in a capacity TARGET's Duration-bound field makes the next commit touching that parent panic mid-commit in debug, or resolve a wrapped ~2^64 ceiling in release that silently ACCEPTS any group measure — a wrong commit verdict where the contract promises a typed `Corruption` error.

### Suggested fix

Replace `Ok(end - start)` with `end.checked_sub(start).ok_or(Error::Corruption(CorruptionError::MalformedValue("capacity interval inverted")))`. Both sweeper call sites already treat `Err` as a conviction, and the commit path surfaces it typed. Land with a corrupt-store test mirroring verify_store/tests.rs:1822: green Duration-weighted commit over a general interval, `replace_fact_bytes` swapping the tail's two halves, then assert findings (no panic) — the exact fixture shape used in the reproduction.