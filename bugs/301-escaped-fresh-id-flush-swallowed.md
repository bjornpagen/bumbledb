# Escaped fresh-ID flush failures are silently discarded
- id: 301
- severity: high
- confidence: confirmed
- area: persistence
- components: crates/bumbledb/src/storage/commit/write.rs, crates/bumbledb/src/api/db/write.rs
- status: open (do not fix)

## Summary

Once `alloc()` returns a fresh id to the host, that id must never be reissued (`never_reissue_observable`). On every abort path the engine burns the escaped `Q` high-water with `flush_escaped_fresh_ids`, then **throws the `Result` away**. If that counters-only LMDB commit fails (ENOSPC, transient `CommitSync` after retries, etc.), the on-disk `Q` mark stays below the issued id and the next write transaction can mint the same value again while the host still holds it.

## Evidence

Abort-after-`commit()` owns the flush but discards failure:

```191:195:crates/bumbledb/src/storage/commit/write.rs
    if outcome.is_err() {
        let _ = flush_escaped_fresh_ids(env, &delta);
    }
```

The write-closure drop guard does the same, including on panic:

```78:83:crates/bumbledb/src/api/db/write.rs
        // Best-effort, panic-safe: the result is discarded — the abort's
        // own error (or unwind) dominates, and a discarded flush failure
        // never turns an unwind into a double-panic abort. The silently
        // no-oped disk failure is the recorded narrowing
        let _ = flush_escaped_fresh_ids(self.env, &delta);
```

The empty-delta success path uses `flush_escaped_fresh_ids(...)?` and therefore does **not** have this hole. The invariant is only dropped on abort/unwind.

`flush_escaped_fresh_ids` itself is a real durability boundary (`commit_bounded` + `mdb_txn_commit`); it can return `Error::CommitSync` / `Error::Lmdb`.

## Why this is a bug

The never-reissue law is an observable identity contract, not a best-effort hint: a host that stored an `alloc()` result in another system (or even in the same process) must not later see that id attached to a different entity. Swallowing the flush error makes the next `alloc()` a silent reuse after a disk/sync failure the caller never saw.

The comments record this as an accepted Lean narrowing. It is still a broken invariant under I/O failure: the process continues as if the id were burned.

## How to trigger / repro sketch

1. Schema with a `fresh` u64 field.
2. `Db::write(|tx| { let id = tx.alloc::<Id>()?; Err(...) })` (or panic after `alloc`).
3. Force the counters-only commit inside `flush_escaped_fresh_ids` to fail (full volume, injected `CommitSync`, etc.).
4. The outer `write` returns the closure error / unwinds; the flush error is gone.
5. A later `write` + `alloc()` can reissue `id`.

Contrast: the same `alloc()` in a successful empty-delta commit surfaces flush failure via `?`.

## Related

- `lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`
- `flush_escaped_fresh_ids` in `storage/commit/write.rs`
- Empty-delta path at `write.rs:100-110` (correct `?`)

## Verification (2026-08-12)

Confirmed. `WriteDelta::alloc` returns the current `Q` mark to the caller and advances it in memory (`crates/bumbledb/src/storage/delta/alloc.rs:26-36`). Those dirty marks (`dirty_fresh_marks`, `next > base`) are the only thing `flush_escaped_fresh_ids` writes (`write.rs:302-321`): a real `commit_bounded` LMDB write txn + `txn.commit()`, which can still return `CommitSync` after three retries or `Lmdb`/`Io` on map-full / ENOSPC.

Two abort owners discard that `Result`:

- After `commit()`'s `outcome` closure: `write.rs:193-195` `let _ = flush_escaped_fresh_ids(env, &delta);` — comment says a flush failure must not mask the caller's error.
- `EscapedIdBurn::drop` when the write closure returns `Err` or panics before `commit()`: `api/db/write.rs:78-83` `let _ = flush_escaped_fresh_ids(...)`.

The empty-delta success path at `write.rs:109` uses `flush_escaped_fresh_ids(...)?` and therefore surfaces the same I/O class. The next write's `read_fresh_next` (`alloc.rs:76-83`) reads the on-disk `Q` (missing = 0), so a discarded failed burn lets `alloc()` mint the same u64 the previous closure already handed out.

Lean/architecture record this as a sanctioned narrowing (`Fresh.lean` § narrowings recorded, `10-data-model.md:318-321`). It remains a runtime identity hole: the host never sees the burn failure, and `never_reissue_observable` does not hold after that I/O. Severity stays **high**.
