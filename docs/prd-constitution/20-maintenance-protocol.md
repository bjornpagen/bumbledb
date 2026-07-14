# PRD 20 — The maintenance protocol: derived relations and conditional writes, witnessed

**Depends on:** 05 (GenerationId + FinalStateView vocabulary).
**Modules:** `docs/architecture/70-api.md` (the protocol section),
`docs/cookbook.md` + `crates/bumbledb-query/tests/cookbook.rs` (the
recipe + locks), `crates/bumbledb/src/api/db/write.rs` (doc comments;
signatures expected UNCHANGED — verified: `write`, `write_from
(snapshot)`, and the internal `write_witnessed` already encode the
witness classes in signatures).
**Authority:** brief B3+B4, approved and merged: dependency statements
prove stored derived facts SOUND relative to sources; they do not prove
COMPLETENESS relative to a query — freshness is host discipline under a
generation witness. That division of authority exists in the machinery
(`write_from`, `GenerationMoved`) but no document states the protocol
and no lock pins each conditional-write idiom against generation
movement.
**Representation move:** none in the engine (the signatures already
carry the distinction). The protocol becomes documented, classified,
and locked — the epistemic upgrade from "works" to "stated and pinned."

## Context (decided shape)

1. **The protocol, in 70-api.md** (one section, normative): snapshot →
   derive → diff → `write_from(snapshot)` → commit or
   `GenerationMoved` → re-derive and retry. The three API classes
   named per the brief: snapshot-derived generation-witnessed
   (`write_from`), final-state point-read inside the write transaction
   (reads through `WriteTx` see base+delta), unconditional (`write`).
   The division-of-authority sentence: "dependencies prove surviving
   derived facts sound; the WITNESS proves the derivation saw the
   state it claims; nothing proves completeness — recompute under a
   new witness."
2. **The cookbook recipe** (roster +1): "Derived facts, maintained" —
   a source relation, a derived rollup relation constrained sound by a
   containment, the host maintenance loop as a doc-tested function,
   and the retry path exercised.
3. **The locks:**
   - generation movement between snapshot and commit →
     `GenerationMoved`, for EACH idiom: update-where, insert-select,
     read-modify-write (three cases, one per idiom, each moving the
     generation via a concurrent write between derive and commit);
   - stale-derived rejection: after source movement, a surviving
     derived fact that violates its soundness containment is rejected
     by the final-state judgment (the dependency net working as
     designed — pinned);
   - the no-unwitnessed-path audit: enumerate every public write
     entry point (grep `pub fn` in api/db/write.rs and any siblings)
     and classify each into the three classes in the doc — an entry
     point that fits none is a policy-5 stop.
4. Explicitly OUT: automatic retries, hidden derivation semantics,
   materialized-view equality claims (D5 territory — the doc SAYS so,
   citing the refusal ledger).

## Technical direction

Docs and recipe first, locks second (they pin what the docs state).
The locks live with the existing optimistic-CC tests (find the
GenerationMoved suite and extend beside it). Zero engine diffs
expected; doc comments on `write`/`write_from` may sharpen.

## Passing criteria

- `[shape]` The protocol section exists with the three named classes
  and the division-of-authority sentence; the entry-point audit table
  in this file's Results with every public write path classified.
- `[test]` Cookbook suite green at the new roster; the three
  generation-movement locks + the stale-derived rejection lock green.
- `[shape]` Zero engine source changes beyond doc comments
  (`git diff --stat`).
- `[gate]` Full suite green; fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

This PRD is its amendments; README recipe count follows.

## Results (2026-07-13)

The no-unwitnessed-path audit used `rg -n "pub fn"` across
`crates/bumbledb/src/api/db/` and its exported API siblings. Every logical
data-write path fits the three documented classes:

| public path | classification | audit disposition |
|---|---|---|
| `Db::write_from` | snapshot-derived, generation-witnessed | the sole snapshot-derived entry; takes `&Snapshot`, compares its generation before the closure runs, returns `GenerationMoved` on mismatch |
| `Db::write` with `WriteTx::{contains,get,get_dyn}` | final-state point-read inside the write transaction | point premises are read from base + pending delta under the single-writer critical section |
| `Db::write` without a read premise | unconditional | no proposition derived from earlier state exists to witness |
| `Db::bulk_load` | unconditional | chunked calls to `Db::write`; imported input is host data, not a database-derived premise |
| `WriteTx::{insert,insert_dyn,delete,delete_dyn,alloc,alloc_at}` | inherits the opening `Db::write` / `Db::write_from` class | delta operations, not independent transaction entry points; none can bypass the opener's witness decision |

The grep also sees `api::db::plumbing`, but `lib.rs` exports it only under the
doc-hidden `__private` macro-support module and explicitly says it is not API.
`Db::{create,open,compact}` write or copy files but are lifecycle/maintenance
operations, not logical data-write transactions; none accepts a read-derived
premise. No fourth or unclassified logical write entry exists.

Recipe 27 (`Derived facts, maintained`) adds the compiled
`maintain_busy_spans` derive/diff/retry loop. Its deterministic interleaving
moves the source generation after the first derivation, observes exactly one
`GenerationMoved` retry, recomputes, and commits the new packed span. The
optimistic-CC suite now separately locks update-where, insert-select, and
snapshot read-modify-write movement (including that the stale closure never
runs), plus final-state rejection of a surviving derived fact after its source
delete. Engine signatures and implementation are unchanged.
