# store-003: live `Environment` encodes three modes as `_lock: Option` + `dirty_marker: Option`

- **Severity:** medium
- **Tree:** store
- **Status:** OPEN
- **Source:** audit/storage-schema.md F12
- **Depends on:** none
- **Conflicts with:** none

## The bug

Disk `StoreKind` is a sum (Durable | Ephemeral) — the open path already parses. The live handle (`storage/env.rs:238-261`) then has two independent Options: `_lock` (None = exhume) and `dirty_marker` (Some = ephemeral writer). Three modes (durable writer, ephemeral writer, exhume reader); the product admits lockless-ephemeral-writer and locked-exhume. `Drop` tests `dirty_marker.take()` to decide whether to fsync. Exhume's armed-marker conviction is a `MalformedValue` string (err-004).

## Why it's wrong

Insight 4 — two Options, four states, three valid. Insight 6 — `StoreKind` was parsed at open and discarded into holes. R17/R18 already named the three lanes.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree.

```rust
enum EnvMode {
    Durable { lock: File },
    Ephemeral { lock: File, dirty_marker: PathBuf },
    Exhume,
}
```

`Drop` matches Ephemeral. Write constructors cannot spell Exhume. Exhume cannot spell a marker. Disk `StoreKind` stays the persisted sum.

## Acceptance criteria

- [ ] Gone: `rg -n 'dirty_marker: Option' crates/bumbledb/src/storage/env.rs`; `rg -n '_lock: Option' crates/bumbledb/src/storage/env.rs`.
- [ ] `Environment::drop` matches the ephemeral arm; durable/exhume arms have no marker path.
- [ ] Unchanged tests: `storage/env/tests.rs` durable/ephemeral/exhume/orphan-marker tests green.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- R17 lock law (exhume lockless, writers locked) and R18 dirty-marker lifecycle identical. `StoreKindMismatch` / `StoreKindInvalid` names locked. Armed ephemeral marker at exhume stays a hard error (named variant per err-004, or keep the string until that lands).
