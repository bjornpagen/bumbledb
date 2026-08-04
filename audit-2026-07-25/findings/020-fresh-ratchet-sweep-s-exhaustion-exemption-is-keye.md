## Fresh-ratchet sweep's exhaustion exemption keys on max_fresh alone, masking regressed Q next-values whenever any row holds u64::MAX

bug | medium | CONFIRMED | cross-branching-new
outcome: fixed d3356737

### Summary

`ratchet_broken(stored, max_fresh)` in `crates/bumbledb/src/verify_store/fresh.rs:24-26` returns false whenever `max_fresh == u64::MAX`, regardless of the stored `Q` next-value. But the module's own doc (fresh.rs:9-11) states the one legal exemption as `next == value == u64::MAX`, and the commit path makes `stored == MAX` the only legal state once a MAX-valued fresh row exists. The over-broad exemption means a single legal `u64::MAX` fresh row silently disables the ratchet-law check for its entire `(relation, field)` — a corrupted or regressed `Q` entry (or a fully deleted one, read as 0) passes `Db::verify_store` clean, and `alloc()` on that store re-issues ids other rows hold, violating the Lean-pinned never-reissue law.

### Evidence

- `crates/bumbledb/src/verify_store/fresh.rs:24-26` — the predicate:
  ```rust
  fn ratchet_broken(stored: u64, max_fresh: u64) -> bool {
      max_fresh != u64::MAX && stored <= max_fresh
  }
  ```
  `stored` is never inspected when `max_fresh == u64::MAX`.
- `crates/bumbledb/src/verify_store/fresh.rs:9-11` — the documented exemption requires both: "an explicit `u64::MAX` fresh value leaves the sequence exhausted with `next == value == u64::MAX` — never convicted."
- `crates/bumbledb/src/storage/delta/insert.rs:101-103` — an explicit MAX insert is legal and saturates the mark: "`saturating_add`: an explicit u64::MAX is legal to insert; the sequence is then exhausted for the generator (alloc errors)." → `mark.next = mark.next.max(value.saturating_add(1))`. So after any commit containing a MAX-valued fresh row, the stored `Q` next is exactly `u64::MAX`.
- `crates/bumbledb/src/storage/delta/alloc.rs:29-34` — `alloc` refuses only when `next == u64::MAX`; any regressed next (< MAX) mints live.
- `crates/bumbledb/src/verify_store/fresh.rs:64-73` (Q-entry pass) and `:78-90` (absent-entry pass, `ratchet_broken(0, max_fresh)`) both route through the broken predicate.
- No sibling pass covers it: `crates/bumbledb/src/verify_store/counters.rs:49-55` treats any S high-water on a fresh-keyed relation as malformed, and `:79-81` states a fresh-keyed relation "OWES no S high-water … its mint is Q, judged by the Q pass's ratchet law" — the Q pass is the sole judge.
- `lean/Bumbledb/Txn/Fresh.lean:283` — `never_reissue_observable`, the law the sweep (finding 033) verifies at rest.
- Existing tests (`crates/bumbledb/src/verify_store/tests.rs:788`, `:812`) only exercise `max_fresh = 1`; the exemption arm is untested.

### Failure scenario / impact

A host legally inserts a fact with fresh value `u64::MAX` (insert.rs:101 sanctions this). The store is later corrupted — a torn `Q` page restored from an old snapshot, or a hostile edit — so the field's stored next reads 3 while rows with fresh values 5 and MAX exist. `Db::verify_store` reports the store clean (the MAX row trips the exemption for the whole field), then `alloc()` mints 3, 4, 5 — re-issuing id 5 the host already holds, violating `never_reissue_observable`. The same masking applies if the `Q` entry is deleted outright (absent reads as 0; `read_fresh_next` returns 0 and alloc re-mints from the bottom). This is precisely the corruption class the finding-033 sweep exists to convict at rest.

### Suggested fix

Tighten the predicate so the exemption is what the doc says it is — the exhausted state `stored == u64::MAX` (which implies `max_fresh == MAX` whenever it fires via `stored <= max_fresh`):

```rust
fn ratchet_broken(stored: u64, max_fresh: u64) -> bool {
    stored <= max_fresh && stored != u64::MAX
}
```

Update the `FreshNextValueLow` doc at `crates/bumbledb/src/verify_store.rs:230-234` (currently "the legal exhausted sequence (`max_fresh == u64::MAX`) is exempt") to state the exemption as `stored == u64::MAX`, and add a sweep test with committed rows `{5, u64::MAX}` against a raw-regressed stored next of 3 expecting `FreshNextValueLow { stored: 3, max_fresh: u64::MAX }`, plus the absent-entry variant.