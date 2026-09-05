# Operations runbook — backup, restore, admin

Status: permanent doc over the shipped admin surface
(`@bjornpagen/bumbledb-log` root exports). Retention and recovery
meanings live in [behavioral-obligations.md](behavioral-obligations.md)
(`REP-*`, `GC-*`, `BACKUP-*`, `RESTORE-*`). All
mutating admin operations return the three-way certainty
`completed | not-started | outcome-unknown` around an operator-supplied
operation reference; `outcome-unknown` is resolved by re-running status
or the same operation with the SAME operation ID — never by assuming a
timeout meant failure. Read-only verification has typed errors.

Every procedure below names the API operation; the example admin job
(`examples/notes/scripts/migrate.ts`) shows the persist-ref-first calling
pattern that applies to all of them.

## Identities the operator mints and records

- `OperationId` — one per admin intent, minted once, persisted with the
  intent BEFORE the first attempt, reused on every retry.
- The stable creation identity for `initialize`/`create` — recorded with
  the tenant forever (a retry after uncertain creation validates it and
  completes genesis; it can never adopt an unrelated database).

## Backup

1. Choose/verify credentials: the backup identity has only its
   prefix-constrained additional permissions (never the data-writer's).
2. `checkpoint(binding, { operationId, ...work })` if a fresh coherent
   root is wanted; checkpointing streams one coherent snapshot plus a
   validated suffix under sustained load — there is no quiet-window
   requirement.
3. `pinRestorePoint(binding, { operationId, label, ...work })` — explicit
   named retention. Retention is the current recoverable root plus named
   restore points; there is NO automatic time/PITR policy and no
   clock-based expiry. Release with `releaseRestorePoint`.
4. `backup(binding, { operationId, ...work })` — independent verified
   bytes to the configured destination, NOT an active-store pointer.
5. `verifyBackup(...)` (typed E, read-only) — verification is a separate
   step; an unverified backup is not evidence.

An INDEPENDENT protected recovery root — a backup copy under credentials
the ordinary GC/writer roles cannot delete — is part of the deployment,
not an SDK feature: configure the backup destination bucket/prefix with
deny-delete policy for the data-plane roles.

## Restore

1. `restore(...)` creates a NEW WRITABLE INCARNATION from verified backup
   bytes. It never mutates the source lineage in place; prior history is
   preserved and restore provenance is recorded.
2. Application `Id128` values are preserved byte-for-byte; applied
   migration history and activation markers ride along (seeds are NOT
   re-run on restore).
3. Re-point the application's tenant binding at the restored incarnation
   explicitly (the binding registry is app-owned); old bindings keep
   refusing with a lineage mismatch rather than silently serving the
   wrong incarnation.
4. Drill (BACKUP-*/RESTORE-* gates; NotRun until executed): delete the local cache and the
   active namespace of a disposable tenant, restore from the protected
   root, verify facts/receipts/history — with data-plane credentials
   proven UNABLE to delete the protected root.

## Receipt epochs

- `rotateReceiptEpoch` opens a new admission epoch; `retireReceipts`
  permanently retires receipts through a bound. Closure/retirement are
  explicit maintenance decisions, never wall-clock expiry; a retired
  epoch's command IDs refuse execution forever
  (`ReceiptExpiredUnknown` on resolve).
- Rotate on the application's business retry horizon: a client that may
  retry a command for N days needs its epoch admitted for at least N days.

## Garbage collection

`collectGarbage(binding, { operationId, ...work })` — rooted epoch GC
only: retained roots, the protected recovery root and every reachable
dependency stay; progress is durable and resumable; failed deletion never
discards sole discovery evidence. Scratch/staging files are never
deletion authority. GC needs only its own prefix-constrained list/delete
permissions.

## Erase

`erase(...)` tombstones an authority: ordinary lookups refuse, explicit
retained roots remain honored, and the report is HONEST about residual
copies (backups, named roots) that require their own erasure decisions.
Erasure of the protected recovery root is a separate deliberate act under
the protected credentials.

## Diagnosing

- `history.inspect(work)` / `cache.inspect(work)` — bounded structured
  counters/status (redacted by default: IDs, codes and cost counters, no
  tenant facts or credentials).
- A `CloseReport` of `incomplete`/`failed` retains native Closing
  accounting; the owner's directory cannot be adopted by a successor
  until the drain completes. Inspect, drain, or restart the process —
  never delete lock files by hand while a process may be alive.
- `resolve(ref, work)` answers command uncertainty:
  `found | not-recorded-at | command-epoch-closed | receipt-expired-unknown`.
