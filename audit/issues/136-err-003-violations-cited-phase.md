# err-003: `Violations.cited` is empty until `attach_cited` — a phase flag in the data

- **Severity:** medium
- **Tree:** err
- **Status:** OPEN
- **Source:** audit/storage-schema.md F11
- **Depends on:** none
- **Conflicts with:** err-002 (cited-facts parallel array)

## The bug

`error.rs:1038-1045,1084-1108` — "`Empty until the commit boundary's decode pass attaches it.`" `seal` / `one` inhabit an undecorated set; `cited_facts` returns `[]` for that phase and for an out-of-range index. `attach_cited` `assert_eq!`s parallel lengths. A rejection without decoded facts is representable, then accidentally empty at the bindings layer.

The commit path *also* legitimately ships an undecorated set: `storage/commit/write.rs:207-245` — decoration is **best-effort**. `decorate_rejected` returns the sealed citations unchanged on `read_txn` / decode failure. Comment: "a sealed `CommitRejected` is never replaced by that flush error (or by a later decoration failure)." `verify_store` consumes `CommitRejected` as a probe result and rewrites it into `StoreFinding` — those sets are never decorated.

## Why it's wrong

Insight 6 — the decode pass learned the facts and stuffed them into a parallel array the type always allowed to be missing. Insight 4 — undecorated vs decorated as one product. The phase flag is real; forbidding the undecorated arm on `CommitRejected` is not.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree.

Do **not** make `Error::CommitRejected` require nonempty `cited`. That would replace a sealed rejection with a decorate-path error, which `write.rs` explicitly forbids.

```rust
enum Violations {
    Citations(Box<[Violation]>), // collectors, sweeper, decorate-failure
    Decorated { citations: Box<[Violation]>, cited: Box<[Box<[CitedFact]>]> }, // lengths equal
}

enum Error {
    CommitRejected { violations: Violations }, // either arm
    ...
}
```

- `attach_cited`'s assert becomes the `Decorated` constructor (parallel lengths by type).
- `cited_facts` on `Citations` returns `[]` (decorate-failure and sweeper). On `Decorated` it indexes the parallel array.
- Sweeper re-play stays `Citations` and may still *match* `CommitRejected` as today's probe encoding, then map into `StoreFinding`. Do not invent a second error kind for the sweeper unless the probe is rewritten in the same commit with tests unchanged.

## Acceptance criteria

- [ ] `Decorated` cannot carry empty `cited` when citations are nonempty (constructor forbids it). `CommitRejected` **can** still carry `Citations` when decoration fails.
- [ ] Gone: a single product type whose `cited` field is "not yet" vs "failed" vs "attached" with no distinction.
- [ ] Unchanged tests: commit-reject render tests still see decoded facts on the happy path; decorate-failure still yields `CommitRejected` (not `Corruption`/`Lmdb`); sweeper findings still work without decoration.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Decode-at-reject-time (pending interns) stays inside the commit boundary. Citation sort/dedup/nonempty seal unchanged.
- Never swap `CommitRejected` for a decoration failure. Public `Violations` API may grow a type rather than silently changing `cited_facts` to panic.
