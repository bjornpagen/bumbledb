# `CommitRejected` can be replaced by a later `read_txn` / decode error
- id: 304
- severity: medium
- confidence: confirmed
- area: correctness
- components: crates/bumbledb/src/storage/commit/write.rs
- status: open (do not fix)

## Summary

After phases 1–2 (or 3) collect a complete violation set, `commit` maps `Error::CommitRejected` through a **new** read transaction and `decode_cited_facts`. Those steps use `?`. If `env.read_txn()` fails (`ReadersFull`, LMDB) or a cited fact cannot decode (`Corruption`, dangling intern), the original `CommitRejected` is dropped and the host sees the secondary error with **no violation set**. The write was already aborted; the caller loses the only information that would tell them *why*.

## Evidence

```193:209:crates/bumbledb/src/storage/commit/write.rs
    if outcome.is_err() {
        let _ = flush_escaped_fresh_ids(env, &delta);
    }
    let report = match outcome {
        Err(Error::CommitRejected { violations }) => {
            let view = env.read_txn()?;
            return Err(Error::CommitRejected {
                violations: decode_cited_facts(violations, schema, &view, &delta)?,
            });
        }
        other => other?,
    };
```

`decode_cited_facts` (`write.rs:228-290`) can also `?` on dictionary resolve / fact decode. Either `?` exits with a non-`CommitRejected` error.

This runs under the writer mutex but **not** inside the aborted LMDB write txn; concurrent readers can still exhaust the reader table between abort and this decoration.

## Why this is a bug

The public contract of a constraint failure is `CommitRejected` carrying the complete sealed set (`70-api.md`, `Violations`). Substituting `ReadersFull` or `Corruption` makes a legal, fully-judged rejection look like infrastructure failure. Hosts that branch on `CommitRejected` to present statement citations will take the wrong branch; retry-on-transient logic may retry a commit that can never succeed.

The facts that caused the rejection are still in the in-memory delta at this point (`pending_raw` is why decode happens here). Losing them because a *second* snapshot could not be opened is not required by the decode design — the error should remain `CommitRejected`, with decode failure attached or with undecoded citations, not a wholesale kind change.

## How to trigger / repro sketch

1. Fill the LMDB reader table (`MAX_READERS`) with live snapshots on other threads.
2. Run a write that must fail a key or containment (duplicate unique key is enough).
3. Phase 2/3 produces `CommitRejected`.
4. `env.read_txn()` for citation decode hits `ReadersFull`.
5. Caller receives `ReadersFull` (or `Lmdb`), not `CommitRejected`, and `violations()` is empty.

A corrupt intern id in a cited novel `str` field similarly turns the rejection into `Corruption(DanglingInternId)`.

## Related

- `decode_cited_facts` in the same file
- `Violations::attach_cited` (assumes decode succeeded in parallel)
- Finding 301 (abort path already best-effort on a different flush)

## Verification (2026-08-12)

Confirmed. After the abort-path Q burn, `commit` (`write.rs:204-209`) matches `Error::CommitRejected` and runs `env.read_txn()?` then `decode_cited_facts(...)?`. Either `?` drops the sealed `Violations` and returns the secondary error. `decode_cited_facts` (`write.rs:273-287`) `?`s `encoding::decode_values` and `dict::resolve`, which is `Corruption(DanglingInternId)` on a miss (`dict.rs:147-150`). This is a new snapshot under the writer mutex, not the aborted write txn; concurrent readers can fill the 1024-slot table (`MAX_READERS`, `env.rs:202-203`). `70-api.md` and C++ `Error::is_transient()` (`cpp/src/error.cc:183-187`) treat `ReadersFull` as retryable and `CommitRejected` as permanent, so a host retry loop can storm a write that can never succeed until a slot frees — then it would see the real rejection. Severity stays **medium**: the illegal write still does not persist; the lost citation set and kind swap are the defect.
