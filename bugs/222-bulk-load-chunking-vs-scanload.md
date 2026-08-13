# Lean scanLoad is one judgment; bulk_load is a sequence of 4096-fact commits
- id: 222
- severity: low
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: unspecified
- components: lean/Bumbledb/Txn.lean, docs/architecture/50-storage.md, docs/cookbook.md, crates/bumbledb/src/api/db.rs, crates/bumbledb/src/api/db/write.rs
- status: open (do not fix)

## Summary
Lean `scanLoad` judges the transformed instance as a single final state (`etl_lands_valid`). Rust `bulk_load` / `bulk_load_dyn` commits 4096-fact chunks as ordinary transactions; a failing chunk leaves prior chunks committed. Lean records chunking as mechanism; hosts that treat `scanLoad` as the API contract will see prefix commits and containment failures that the one-shot model cannot produce.

## Lean spec
```686:695:lean/Bumbledb/Txn.lean
noncomputable def scanLoad {T : Theory} (s : State T) (T' : Theory)
    (t : Fact → Option Fact) : Result (State T') (Set Statement) :=
  judge T' (transform t s.inst)
```

Narrowing (`Txn.lean:109-114`): "`bulk_load`'s 4096-fact chunking is mechanism: a chunked load is a SEQUENCE of ordinary commits, each judged … which is exactly why recipe 28's first law — load containment targets first — is host-facing."

## Normative docs
`50-storage.md:495-497`: chunked into multiple transactions, 4096 facts each; failing chunk aborts whole, prior chunks stay committed. Cookbook recipe 28: load containment targets first.

## Rust implementation
`api/db.rs` `BULK_CHUNK = 4096`. `write.rs:382-384` `bulk_chunks`: each chunk a write transaction; committed count on failure.

## Why this matters
A migration that Lean `scanLoad` would reject atomically can leave a prefix of the new store committed and valid. Recipe 28 is the host obligation that makes this safe; it is not implied by `etl_lands_valid` alone. Wrong load order is a real `CommitRejected` the one-shot spec never sees.

## Related
- recipe 28 (`docs/cookbook.md`)
