# 60 — The writer

One commit path, two placements. A writer is a replica plus the right to
create log objects. The host-facing verb is:

```rust
pub enum Commit<R> {
    Accepted { value: R, generation: GenerationId },
    Rejected(Violations),
    Contended { winner: GenerationId },   // serverless mode only: lost the slot; state advanced
}

pub fn commit<R>(
    &self,
    body: impl FnOnce(&mut Batch<'_>) -> Result<R>,
) -> Result<Commit<R>>;
```

`Batch` records ops (typed inserts/deletes lower to the codec's raw rows;
`reserve` mints fresh ids locally and the resulting inserts carry them as
plain values — the log never contains a reservation). The body runs
against the replica's current state for reads-before-write. `Contended`
mirrors `ConditionalWrite::Moved`: an expected answer, data not error;
hosts loop on it exactly like a moved witness.

## Serverless mode (case 1 — any instance may write)

The ordering law is **publish before ack; local state is disposable**:

1. Build the batch; encode with `base = k` (local generation).
2. Apply locally via one `db.write`. `Rejected` → return
   `Commit::Rejected` — nothing touched the network; rejection is free.
3. Accepted → `put_create(log/{k+1}, batch)`.
   - `Created` → ack `Accepted { generation: k+1 }`. (Crash between 2 and
     3 loses only an un-acked local fork, which dies with `/tmp`.)
   - `Exists` → this instance's local store is now a fork: **discard the
     local store**, re-open from checkpoint + log (50), and return
     `Contended { winner }`. The host retries `commit` against the new
     state; the re-judgment may legitimately change the verdict — that is
     the correct semantics of a retried write.
4. Every K commits (or when this writer created `log/g` with
   `g − manifest.checkpoint.g ≥ K`): compact, upload `ckpt/{g}.mdb`,
   manifest CAS. Checkpoint publication races are benign: manifest CAS
   keeps the newest; a losing checkpoint object is garbage for `gc`.

Fork-discard is the price of coordinator-free writes; at SaaS booking
rates contention is rare, and the discard is a re-pull of a small store.
Deployments that measure real contention add the v2 lease object — a
recorded nicety, not v1 surface.

## Resident mode (cases 2/3 — one process owns writes by arrangement)

Local-latency writes with the log as the WAL. The one-slot sidecar closes
the crash window:

1. Build + encode the batch (`base = k`).
2. Write the batch bytes to `dir/pending.batch` (single file, fsynced) —
   the one-deep local WAL.
3. Apply locally (`db.write`). Rejected → delete sidecar, return
   `Rejected`.
4. Ack the host now if configured for `ack = local` (1 ms writes,
   RPO = publish lag) or after step 5 for `ack = published` (RPO = 0).
   The mode is a constructor parameter; the default is `published`.
5. `put_create(log/{k+1})` — in resident mode this never loses (no other
   writer by arrangement); an `Exists` here means the arrangement was
   violated and is a corruption-class error naming both writers.
6. Delete the sidecar.

Crash recovery at open: if `pending.batch` exists —
`local generation == log tip + 1` → republish it (step 5) and continue;
`local == tip` → the crash was pre-commit; discard the sidecar. Both
resolutions are forced by the index=generation law; there is no judgment
call.

## Group commit

The writer runs one commit loop; concurrent host `commit` calls queue.
The loop drains the queue into **one** batch (bounded: 512 host writes or
4 MiB encoded, whichever first), applies as one `db.write` (one generation,
one log object), and resolves every queued caller with the shared outcome.
A rejection rejects the whole batch (the engine's write is atomic); the
loop then falls back to one-by-one application for that drain so an
innocent host write is never rejected for a neighbor's violation —
degraded throughput under rejection is the recorded trade. No linger timer
in v1: batch whatever is queued, never wait.

## Failover and adoption (resident mode)

Adopting a store (new box, restored PITR, promoted replica): open as a
replica, catch up to tip, then publish one **FloorBump batch** advancing
every fresh field's floor to `local floor + gap` (default gap 1 000 000).
This is the never-reissue safety across writer identities: ids reserved
but never committed by the dead writer can never be re-minted by the new
one. The adoption batch is an ordinary log object; replicas replay it like
anything else. Then write `manifest.writer` (advisory) and begin.

## Backups, restated

Resident mode inherits everything from the log: no `compact`-cron for
backup purposes (checkpoints are for replay speed, not safety), no
snapshot schedule, no RPO math beyond the ack mode. The serverful backup
runbook from the pre-log era survives only as the belt-and-braces block
snapshot, optional.
