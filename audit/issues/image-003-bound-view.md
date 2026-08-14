# image-003: `View::image` / `position_at` panic on `Unbound`

- **Severity:** low
- **Tree:** image
- **Status:** OPEN
- **Source:** audit/storage-schema.md F20
- **Depends on:** none
- **Conflicts with:** none

## The bug

`View` (`image/view.rs:212-230`) is already the right three-variant sum ("not a sentinel vector"). Then `image()` and `position_at` are total over a type that includes `Unbound` and `unreachable!` (`view.rs:241-245,272-274`). Phase is in the data; methods pretend it isn't.

## Why it's wrong

Insight 4 — Unbound is a real state of prepare-before-execute. Methods that panic on it are guards the sum already made unnecessary *if the executor held a bound view*.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree. Bound views as a type the executor holds after the first bind:

```rust
enum BoundView { All(Arc<RelationImage>), Survivors { image, positions } }
```

`Unbound` stays on the prepared object until then. Do **not** introduce a third lifetime / typestate across prepare→execute (Insight 15 — that is the expensive version; engine-037's lesson). Split the enum the executor already has in hand.

## Acceptance criteria

- [ ] Gone: `rg -n 'unreachable!\("an unbound view' crates/bumbledb/src/image/view.rs`.
- [ ] Executor/COLT paths take `BoundView` (or match without Unbound); prepare still stores `View::Unbound`.
- [ ] Unchanged tests: view memo / occurrence-dedup tests green.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Unbound-until-first-execute semantics identical. No prepare-pinned image. Recycle/clone_in stay on the prepared-owned buffers.
