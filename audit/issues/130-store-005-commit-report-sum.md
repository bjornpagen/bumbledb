# store-005: `CommitReport { changed: bool, new_generation }` — a mode bit beside the clock

- **Severity:** low
- **Tree:** store
- **Status:** FIXED(26311c8c)
- **Source:** audit/storage-schema.md F23
- **Depends on:** none
- **Conflicts with:** none

## The bug

`storage/commit.rs:74-77`:

```rust
pub struct CommitReport {
    pub changed: bool,
    pub new_generation: GenerationId,
}
```

A counters-only/no-op commit is `changed: false` with a generation that did not advance (the image cache keys on this). The bool restates whether the clock moved.

## Why it's wrong

Insight 4 — two fields, a redundant mode bit. If `changed` is a function of the two generations the caller already has, it is a dual coordinate.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree.

```rust
enum CommitReport {
    Noop { generation: GenerationId },
    Changed { new_generation: GenerationId },
}
```

Cache-advance matches `Changed`. Confirm against the subscriber before landing: if some caller needs "generation after this commit" uniformly, keep a `generation()` accessor on both arms.

## Acceptance criteria

- [ ] `CommitReport` is a sum, or a documented proof that `changed` is not recoverable from the generation pair (then this issue flips to WONTFIX with that proof in the file).
- [ ] Image-cache advance still skips no-op commits; `GenerationMoved` still ignores counters-only commits.
- [ ] Unchanged tests: commit/noop/cache-advance tests green.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- No-op still persists dirty fresh marks. Observable `changed` meaning identical for hosts that match today.
