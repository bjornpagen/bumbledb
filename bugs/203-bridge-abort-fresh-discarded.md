# Bridge ledger says aborted mint runs are discarded; Fresh.lean persists them
- id: 203
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: spec
- components: lean/Bumbledb/Bridge.lean, lean/Bumbledb/Txn/Fresh.lean, docs/architecture/10-data-model.md, crates/bumbledb/src/api/db/write.rs
- status: open (do not fix)

## Summary
The obligation-ledger premise for `never_reissue_observable` claims an aborted transaction's mint run is "discarded whole, so nothing it minted was observable." The theorem it cites, the Fresh module doc, the architecture data-model, and the Rust abort burn all say the opposite: every fate persists the high-water because `alloc` already handed the id to the host.

## Lean spec
Bridge row (`Bridge.lean:533-537`):

> "The mint is a monotone high-water mark per relation and field: any id a committed transaction made observable — generator-returned or explicitly supplied — sits below the persisted mark and is never returned again; an aborted transaction's run is discarded whole, so nothing it minted was observable."

The cited theorem (`Fresh.lean:269-280`, `Reachable.txn` at `:254-258`) states aborts are NOT exempt: "the one `txn` transition persists an aborted run's mark exactly like a committed one." Module doc (`Fresh.lean:8-12`): "EVERY transaction persists its final mark — committed, no-op, or aborted alike."

## Normative docs
`docs/architecture/10-data-model.md:313-321`: abort flushes dirty `Q` marks through a counters-only commit so issued ids are never recycled; best-effort modulo I/O.

## Rust implementation
`EscapedIdBurn` (`api/db/write.rs:29-39`, `:67-80`) burns escaped fresh high-water on every write-region termination that does not reach a successful commit. Tests: `fresh_ids_allocated_in_a_rejected_txn_are_burned`.

## Why this matters
The census-checked Bridge is the machine-listable Lean↔Rust seam. A reader of the ledger (or a future discharge) who trusts the premise sentence will implement abort-as-discard and re-issue ids the host already observed — the exact observability hole Fresh exists to close.

## Related
- 204 (docs that still say abort never touched disk)
