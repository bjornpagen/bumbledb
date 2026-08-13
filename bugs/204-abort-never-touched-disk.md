# Docs claim abort never touched disk; abort burn writes Q marks
- id: 204
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: docs
- components: README.md, docs/architecture/70-api.md, crates/bumbledb/src/lib.rs, docs/architecture/10-data-model.md, crates/bumbledb/src/api/db.rs, crates/bumbledb/src/api/db/write.rs, lean/Bumbledb/Txn/Fresh.lean
- status: open (do not fix)

## Summary
The product README, crate docs, and the embedding-surface architecture doc state that an abort never touched disk / never wrote anything / never touched LMDB. The data-model doc and the write path persist escaped fresh high-water via a counters-only LMDB commit on abort. Lean Fresh requires that persistence. The "never wrote" claim is true of *facts* and *generation*, false of the `Q` namespace.

## Lean spec
`Txn/Fresh.lean:8-12`, `:73-80`, `:81-93`: every transaction persists its final mark, aborts included, via `flush_escaped_fresh_ids`; the abort burn is unconditional modulo I/O failure.

## Normative docs
False on the abort-disk claim:

```51:51:README.md
// judged at commit against the final state — an abort never touched disk.
```

```616:619:docs/architecture/70-api.md
- **The transaction is a delta** (`50-storage.md`): operations are in-memory set
  arithmetic; … nothing touches LMDB until
  commit, and an abort never wrote anything.
```

```849:849:docs/architecture/70-api.md
since the transaction is a delta, an aborted transaction never touched LMDB at all.
```

True on the same axis in `10-data-model.md:313-321`: "The escaped high-water flushes through a counters-only commit that writes exactly the dirty `Q` marks."

## Rust implementation
`api/db.rs:7-11` already qualifies: "an abort … never wrote a fact: the one thing every abort persists is the escaped fresh high-water." `EscapedIdBurn` (`write.rs:67-80`) issues that counters-only write. Crate root `lib.rs:18` still repeats the README "never touched disk" sentence.

## Why this matters
Crash/recovery and lock reasoning that assume abort is a pure in-memory drop will miss a durable `Q` write (and its fsync on durable stores). The never-reissue law is exactly that write. 70-api's "never touched LMDB at all" is a false durability claim.

## Verification (2026-08-12)
Re-read README, `70-api.md`, crate docs, Fresh, and the abort burn. **Confirmed.** `wrong-side: docs`. The “never wrote” claim is true of facts/generation and false of `Q`.

**Lean** (`lean/Bumbledb/Txn/Fresh.lean:8-12`, `:73-80`, `:81-93`): every transaction persists its final mark, aborts included, via `flush_escaped_fresh_ids`; abort burn unconditional modulo I/O.

**Docs:** False: `README.md:50-51` (“an abort never touched disk”); `docs/architecture/70-api.md:616-619` (“an abort never wrote anything”); `:849` (“never touched LMDB at all”); crate root `crates/bumbledb/src/lib.rs:16-18`. True: `10-data-model.md:313-321` (counters-only `Q` flush).

**Rust** (`crates/bumbledb/src/api/db.rs:7-11`; `write.rs:67-80`): abort never wrote a *fact*; `EscapedIdBurn` persists escaped fresh high-water.

## Related
- 203 (Bridge prose still describes abort-as-discard)
