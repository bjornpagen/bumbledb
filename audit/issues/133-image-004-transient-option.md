# image-004: `TransientImage.image: Option` — empty pool as Option plus expect

- **Severity:** low
- **Tree:** image
- **Status:** FIXED(228ef446)
- **Source:** audit/storage-schema.md F21
- **Depends on:** none
- **Conflicts with:** none

## The bug

`image/build.rs:435-549` — a retained-capacity pool: empty at construction, filled after first refill. `fill` `.expect("filled above")` after a branch that just filled. Empty vs occupied is a product the expects reconstruct.

## Why it's wrong

Insight 4 — Option plus expect is empty-vs-full. Insight 8 — a sentinel zero-row image would make first fill the same path as later fills.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree.

```rust
enum TransientImage {
    Empty { capacity: usize },
    Occupied { image: Arc<RelationImage>, capacity: usize },
}
```

Or always allocate a zero-row sealed image at `new` (sentinel — Insight 8). Cheap either way.

## Acceptance criteria

- [ ] Gone: `rg -n 'expect\("filled above"\)' crates/bumbledb/src/image/build.rs`.
- [ ] Empty-vs-occupied is a sum or a sentinel image; refill/append reuse discipline unchanged (Exact vs Doubling).
- [ ] Unchanged tests: reach/transient-image tests green (no assertion edits).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Transient images stay uncounted, never cached, never pinned. In-place `Arc::get_mut` reuse when views have dropped is identical.
