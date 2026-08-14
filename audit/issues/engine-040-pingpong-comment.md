# engine-040: `ReachScratch`'s "Size 1" comment vs `[TransientImage; 2]` ping-pong

- **Severity:** low
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F40
- **Depends on:** engine-013 (the PingPong layout is its deliverable; this is the comment/naming residue)

## The bug

`crates/bumbledb/src/api/prepared/reach.rs:82-87`:

```rust
/// Rec ping-pong: delta vs accumulated of the one SCC. Size 1.
#[derive(Default)]
pub(super) struct ReachScratch {
    delta: [TransientImage; 2],
    acc: [TransientImage; 2],
```

"Size 1" (one SCC — the k-SCC leftover) collides with the width-2 arrays (ping-pong). One SCC means one *pair* of buffers, not a length-1 array of pairs; the comment describes neither.

## Why it's wrong

The comment is a fossil of the k-SCC design colliding with the current layout (Insight 1); a reader must reverse-engineer which axis "1" and "2" each refer to.

## The fix

Rides engine-013: the layout becomes `PingPong { a: TransientImage, b: TransientImage, flip: bool }` (or named `delta_working`/`acc_working` pairs), and the doc says what is true: "the one rec's delta/accumulated working buffers, double-buffered across rounds; `flip` selects the round's writer." No "Size 1", no SCC counting.

## Acceptance criteria

- [ ] Gone: `rg -n 'Size 1' crates/bumbledb/src/api/prepared/reach.rs` → no matches; no `[TransientImage; 2]` arrays whose meaning is carried by a comment (engine-013's layout grep).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Lands inside engine-013's change (one fixer).
