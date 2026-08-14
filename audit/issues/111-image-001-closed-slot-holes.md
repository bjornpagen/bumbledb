# image-001: `ImageCache.closed_slots: Box<[Option<u32>]>` — Option-padded array

- **Severity:** high
- **Tree:** image
- **Status:** OPEN
- **Source:** audit/storage-schema.md F4
- **Depends on:** schema-001
- **Conflicts with:** none (cache layout; lands after the relation sum)

## The bug

`image/cache.rs:104-127` and `cache/new.rs:17-28`:

```rust
closed_slots: Box<[Option<u32>]>,  // None = ordinary; Some(slot) → closed[slot]
closed: Box<[OnceLock<Arc<RelationImage>>]>,
```

`get_or_build` tests `closed_slot(rel).is_some()` then `get_or_synthesize` `.expect("caller probed closed_slot")`. Three encodings of "this id is closed": schema Option (schema-001), the hole in `closed_slots`, the length of `closed`. A foreign id also answers `None` — ordinary and unknown share a hole.

## Why it's wrong

Insight 4 / engine F6 analog — Option-padded array as a phase/kind flag. The cache re-parses a kind schema-001 already owes the witness.

## The fix

Per schema-001: the Closed arm owns its image slot, or a dense closed-only array sized at cache construction, indexed by a `ClosedSlot` minted only for closed relations. No Option hole. `get_or_build` on an ordinary relation cannot spell synthesize.

## Acceptance criteria

- [ ] Gone: `rg -n 'closed_slots: Box<\[Option' crates/bumbledb/src/image/cache.rs`.
- [ ] Gone: `rg -n 'expect\("caller probed closed_slot"\)' crates/bumbledb/src/image/cache`.
- [ ] Closed images still live outside the generation map; never evicted; one `OnceLock` per closed relation.
- [ ] Unchanged tests: `image/cache/tests.rs` closed-image forever-resident tests green.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Lands after schema-001. Lineage law for ordinary relations unchanged. Synthesis remains pure (cannot fail).
