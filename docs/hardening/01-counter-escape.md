# PRD 01 — Counter escape discipline: no-op serial flush, mint-free deletes

Findings fixed (docs/audit/): **api-schema MEDIUM** "Serial ids minted in a
net-no-op committed write escape and are re-issued"; **api-schema LOW**
"Deleting a fact with a never-interned string permanently interns it" (and the
matching storage NOTE tracing the same behavior).

## Purpose

Two escape hatches in the counter discipline. A *successful* write that nets
to no state change drops its serial marks, so ids the closure already returned
to the host get re-issued later — contradicting 10-data-model's never-reissue
guarantee and delta.rs's own "none of them are observable" comment (false as
written: the closure's return value is an observation). And delete-side
encoding mints dictionary entries for values that provably match no fact. Fix
both at the representation: what escaped must persist; what cannot match must
not mint.

## Technical direction

- **Flush serial marks on no-op commits — without touching the generation.**
  `storage/commit.rs:214-237`: the empty-delta and `!applied.changed` paths
  currently abort the txn wholesale. New rule: if the delta carries dirty
  serial marks (`Q` high-waters), write exactly those keys and commit the LMDB
  txn; do **not** bump the storage tx id, do not evict images, do not flush
  pending interns or the dict next-id. Soundness, spelled out (this is the
  resolution of the audit's two reports pulling opposite directions):
  - The generation identifies *query-visible state* (`F/M/U/R`); `Q` marks are
    write-path bookkeeping no query reads. A counters-only commit therefore
    leaves every image, memo, and cache key exactly valid — the
    tx-id-advances-iff-data-changed rule (which the storage NOTE rightly
    defends) is untouched because the tx id is our own `_meta` value, not
    LMDB's.
  - Pending interns stay dropped: intern ids never escape (hosts see values,
    not words), and re-issuing an unflushed provisional id is the established
    abort semantics. Only the serial marks escaped; only they flush.
  - True aborts (closure `Err`/panic) still drop everything — the
    never-advance-on-abort clause holds; this PRD changes *successful* no-ops
    only.
  - Update the delta.rs comment (currently false) and 10-data-model's wording
    to state the refined rule: "a successful commit persists every serial
    value it returned, even when no facts changed."
- **Mint-free delete-side encoding.** `api/db.rs:549-557` encodes deletes
  through the write context so insert-then-delete cancels byte-exactly — but
  that only requires the *pending map*. Give `WriteDelta` a non-minting
  resolve for the delete path (`delta.rs:91-114` grows a sibling):
  pending-map hit → use the provisional id (cancellation works); committed-
  dict hit → use the committed id; both miss → **the fact cannot exist in
  base or delta** (its bytes would embed an id that was never minted), so the
  delete is a proven no-op — return that signal without minting. `delete`
  short-circuits to `Ok(false)`. The dictionary stops growing on typo'd
  deletes; the documented leak class shrinks to what 10-data-model already
  accepts (deleted *real* facts leak their interns).

## Non-goals

Dictionary GC (still a non-goal by decision); changing abort semantics;
changing the tx-id rule (explicitly preserved — see soundness above);
persisting intern ids that never escaped.

## Passing criteria

- The audit's scenario as a test: `let a = db.write(|tx| tx.alloc::<HolderId>())`
  (no insert) then `let b = db.write(|tx| { let id = tx.alloc()?; tx.insert(...)?; Ok(id) })`
  — **`b` is strictly greater than `a`**, across both the empty-delta and the
  insert-then-delete-nets-to-nothing (`changed: false`) paths.
- Generation stability: a serial-only no-op commit does **not** change
  `Db::generation()` and does not invalidate the image cache or the view memo
  (trace-lane test: the next execution memo-hits — extend the existing
  generation-bump trace test with the no-op case asserting `view_memo_hit`).
- Abort semantics unchanged: the existing
  `serials_allocated_in_an_aborted_txn_are_reissued` test still passes
  verbatim.
- Mint-free delete: deleting a fact whose string was never interned no-ops
  AND leaves the dictionary unchanged (assert via a subsequent
  `dict`-observable path: a query for that literal still misses; plus a
  direct storage-level test that `_dict` entry count is unchanged); the
  insert-then-delete-same-fact cancellation test still passes verbatim.
- Crash-safety: kill-during-commit coverage (tests/crash.rs) extended with the
  counters-only commit shape — reopen shows either the old or new `Q` marks,
  never a torn state (it is one LMDB txn; the test pins it).
- delta.rs comment and 10-data-model amendment landed. `scripts/check.sh`
  green.
