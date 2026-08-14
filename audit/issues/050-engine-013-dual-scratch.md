# engine-013: two scratch layouts for one derived-image protocol; published images are `Option` holes

- **Severity:** medium
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F13
- **Depends on:** engine-001 (scratch lives on the pipeline arms); engine-010/032 build on this layout

## The bug

`crates/bumbledb/src/api/prepared/reach.rs:52-102` — two copies of "working `TransientImage` + published `Arc`":

```rust
pub(super) struct DerivedScratch {
    finished_slot: Vec<TransientImage>,            // working
    finished: Vec<Option<Arc<RelationImage>>>,     // published, None until stash
    pub(super) occ_images: Vec<Option<Arc<RelationImage>>>,
    pub(super) retired: Vec<Vec<u32>>,
}
pub(super) struct ReachScratch {
    delta: [TransientImage; 2],
    acc: [TransientImage; 2],
    acc_filled: [usize; 2],
    flip: bool,
    watermark: usize,
    round_delta: Option<Arc<RelationImage>>,       // begin() nulls; loop re-Sames
    round_acc: Option<Arc<RelationImage>>,
}
```

`begin()` (61-68, 95-101) nulls the Options; `stash_finished` (70-79) fills a hole; the round loop (398-415) writes `round_delta`/`round_acc` then immediately clones them out as locals — fields that persist as `None` between rounds for no reader.

## Why it's wrong

Dual layout, dual `None` (Insight 2 + Hoare's null): the same protocol (fill a transient, publish an `Arc`) is spelled two ways, and publication state is an `Option` hole that every reader must trust was filled in the right order rather than a type that only exists after the fill (Insight 6 — stash is a parse; its result should be the non-optional value).

## The fix

Per `audit/CONTRACT.md §C3` ("Binds/scratch"):

- One `DerivedImages` owning, per derived id, a working `TransientImage` and — after that table closes — a published `Arc<RelationImage>` (publication modeled so a published image is `Arc`, not `Option<Arc>`: e.g. the publish step returns/records the `Arc` and binds read a slice of published tables only up to the current phase; a `Vec<Arc>` grown in seal order is the natural shape — interior 0..n, then rec).
- Rec ping-pong is a `PingPong { a: TransientImage, b: TransientImage, flip: bool }` (or two named fields) of the same working/published pair. The "Size 1" comment on a `[; 2]` array dies with `ReachScratch` (absorbs engine.md F40).
- `round_delta`/`round_acc` become LOCALS in `run_reach` (they are consumed within the round), not persistent fields.
- `watermark`, `acc_filled` stay (essential semi-naive bookkeeping) — attach them to the ping-pong.

## Acceptance criteria

- [ ] Gone: `rg -n 'Vec<Option<Arc<RelationImage>>>' crates/bumbledb/src/api/prepared/reach.rs` → no matches for the published table (occ_images layout is engine-032's criterion); `rg -n 'round_delta|round_acc' crates/bumbledb/src/api/prepared/reach.rs` shows locals only (no struct fields); `rg -n 'struct ReachScratch|struct DerivedScratch' crates/bumbledb/src` → replaced by the unified layout.
- [ ] Unchanged tests: all reach/differential tests green unchanged; answers byte-identical.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb && cargo test -p bumbledb-bench`; `./scripts/check.sh`; allocation gates in `check.sh` still pass (the layout change must not add per-round allocation — reuse the transients exactly as today).

## Constraints

- Semantics and allocation discipline identical (transient reuse, buffer recycling through `spare_buffers`/retired pool unchanged).
- Lands after engine-001; engine-010/032/040 stack on it.
