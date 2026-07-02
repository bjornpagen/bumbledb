# PRD 08 — Commit: FK Validation, Counters, Tx-Id, Commit

Authority: `docs/architecture/40-storage.md` (commit steps 3–5, tx-id rules),
`10-data-model.md` (Restrict = no dangling references in any committed state).

## Purpose

Commit phases 3–5: final-state FK validation, counter flush, tx-id advance, LMDB
commit.

## Technical direction

- Extends `storage::commit::apply` into `commit(delta, env) -> Result<CommitReport>`:
  3. **FK validation** against the txn's own uncommitted state (LMDB write txns read
     their writes): (a) forward — for every inserted fact and each of its FKs, probe
     `U | target_rel | target_constraint | key`; miss → `ForeignKeyViolation`
     (constraint id + offending fact); (b) Restrict — for every unique key deleted in
     phase 1 and **not re-established** by phase 2 (track via PRD 07's bookkeeping),
     prefix-scan `R | rel | constraint | key`; any surviving entry →
     `ForeignKeyViolation` naming the referencing (source_rel, source_row).
  4. **Counters**: flush per-relation row counts (`S`), row_id high-waters, serial
     next-values (`Q`), dictionary next-id; **storage tx id advances iff the delta
     changed state** (any applied insert or delete; an all-no-op delta commits without
     bumping and PRD 11's cache stays valid — implement as: skip the whole apply when
     the net delta is empty).
  5. LMDB commit (fsync implied by env flags). Any error anywhere → abort, nothing
     persisted.
- `CommitReport { changed: bool, new_generation: u64 }` — PRD 11 subscribes to this
  for cache eviction; PRD 28 wires it.

## Non-goals

Cache eviction (PRD 11). The public write closure (PRD 28).

## Passing criteria

- Unit tests: insert referencing a target inserted in the same delta commits (order
  irrelevance); insert referencing a missing target → FK violation, aborted, base
  intact; delete of a referenced target alone → Restrict violation naming the
  referencer; delete of target + all referencers in one delta commits; delete+reinsert
  of a referenced unique key in one delta commits (Restrict sees the re-established
  key); tx id advances exactly once per state-changing commit and not for all-no-op
  deltas; counters after reopen match a recount of F entries (reopen inside the test —
  this is a unit contract of counter flushing, not an e2e suite); serial values
  allocated in an aborted txn are re-issued by the next txn (committed sequence
  untouched).
- Global commands green.
