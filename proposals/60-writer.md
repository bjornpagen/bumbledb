# 60 — The writer

A writer is a replica plus the right to create log objects. One commit
path; two placements; the loser algebra (15) between them and the bucket.

```rust
pub enum Commit<R> {
    Accepted { value: R, braid: BraidId, generation: u64 },
    Rejected(Violations),
}

pub fn commit<R>(
    &self,
    body: impl FnOnce(&mut Batch<'_>) -> Result<R>,
) -> Result<Commit<R>>;
```

There is no `Contended` arm anymore: contention is the driver's to absorb
via the loser algebra — a disjoint loss republishes silently; a conflicting
loss re-judges, and the host receives either the eventual `Accepted` or
the `Rejected` that serial execution would have produced. Bounded retries
(default 16 losses) then surface as an `Err::Contention` with the braid
and last winner — an operational signal (add escrow, examine the hot key),
not an expected outcome. `Batch` records typed inserts/deletes; `reserve`
draws from the id lease (10) and the resulting inserts carry concrete
values; reservations never appear in the log.

## Auto-split (spanning batches)

`commit` partitions the recorded ops by braid. One braid — the normal
case, since the schema's own dependencies put related relations together —
commits as below. Multiple braids — writes to relations *no statement
relates* — commit as independent per-braid batches, sequentially, and the
outcome is the vector of per-braid outcomes (`Commit::Split(Vec<…>)` in
the full signature). Partial completion is semantically invisible to the
theory (L9: no statement can observe cross-braid state); a host invariant
spanning braids is by definition undeclared — declare it and the braids
merge. No cross-braid claim protocol exists, deliberately.

## Serverless mode (any instance may write)

Publish-before-ack; local state disposable; per braid:

1. Build the batch; compute the footprint (`footprint(descriptor, ops)`);
   set `braid_gen = vector[braid] + 1`.
2. Apply locally (sidecar discipline, one `db.write`). `Rejected` →
   return it — the network was never touched; rejections are free.
3. `put_create(log/{braid}/{braid_gen})`.
   - `Created` → ack `Accepted`.
   - `Exists` → fetch the winner, run the loser algebra:
     a. **Disjoint** (no CONFLICT cell): apply the winner's batch locally
        (sidecar discipline), `braid_gen += 1`, recompute nothing,
        republish. The already-committed local application of *our* batch
        is not rolled back — but note the subtlety: our batch committed
        locally *before* the winner's; the replayed order everywhere else
        is winner-first. L8 (commutativity under disjointness) is exactly
        the theorem that makes the two orders byte-equal, so the local
        store remains a correct materialization. This is where L8 is
        load-bearing, not decorative.
     b. **Conflict**: the local store diverged in a way L8 does not cover
        → discard the directory, re-open (50), re-run the host body
        against fresh state, return its verdict (accepted after
        republish, or the honest rejection). Fork-discard is the price of
        a real conflict — and only of a real one; the disjoint common
        case keeps everything.
4. Checkpoint duty: after creating a batch that puts the vector sum ≥
   `manifest.checkpoint.sum + 256` (or 16 MiB of log since), compact,
   upload, manifest-CAS. Races benign.

## Resident mode (one process owns writes by arrangement)

Local-latency acks; the log is the WAL; per braid, the sidecar gains a
`pending` slot (the batch bytes, fsynced) alongside the vector:

1. Encode; write `pending = {braid, gen, bytes}` (fsync).
2. Apply locally. Rejected → clear pending, return `Rejected`.
3. `ack = local` → ack now (1 ms, RPO = publish lag) or `ack = published`
   → ack after 4. Constructor parameter; default `published`.
4. `put_create(log/{braid}/{gen})`. `Exists` here means the single-writer
   arrangement was violated — corruption-class, naming both writers.
5. Clear `pending`.

Recovery at open: `pending` present and `vector[braid] == gen` → the
commit landed; republish, clear. `pending` present and
`vector[braid] == gen − 1` → pre-commit crash; discard pending. Two
forced resolutions, composed with the vector-intent rules of 50.

## Group commit

One commit loop per braid; concurrent `commit` calls partition and queue
per braid. Each drain packs up to 512 host writes or 4 MiB into one batch
(one generation, one object) and resolves every caller with the shared
outcome; a rejection triggers one-by-one fallback for that drain so an
innocent write never fails for a neighbor's violation. No linger in v1.
Hot-braid throughput = PUT rate × batch size; braids multiply it.

## Adoption and failover (resident)

Adopt = open as replica, catch up all braids, read the id-lease counters
(the authoritative floors — no in-log floor ops exist), lease fresh
ranges, set `manifest.writer` (advisory), begin. A stale previous writer
still holding old leases can only produce K-conflicts, which the loser
algebra converts into honest rejections.

## Escrow (v2, per 15)

Wired here when a measured hot capacity parent exists: hold a grant →
treat `Δ ≤ granted` at that key as conflict-free in step 3a. Avoidance
only; the algebra remains the guard.

## Backups, restated

Nothing here schedules backups. Checkpoints exist for replay speed;
retention (10's `gc`) is the backup policy; PITR is the vector math.
Resident deployments may add block snapshots as belt-and-braces, not as
the story.
