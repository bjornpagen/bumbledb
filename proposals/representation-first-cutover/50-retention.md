# 50 — The gc floor is a write-path invariant

> **Decision.** The published checkpoint vector is the one **floor**, and
> it is consulted on every path that *creates or deletes* a slot, not
> only on the replica read path. The sweep is a resumable bottom segment.
> Retention ages by a trusted publish clock, never a writer-claimed
> timestamp. Every scratch directory, temp file, and thread has an owner
> that reclaims it.

## The current representation

The floor exists, but it is enforced in exactly one place — the replica
deciding what it may read — and nowhere on the paths that *write*. So the
write paths forge and strand:

- **Slot resurrection.** A stale writer whose `put_create` returns
  `exists`, then whose follow-up `get` returns `null` (the occupant was
  gc-swept below a newly advanced floor), loops straight back into
  `put_create` and *re-creates the retired slot* with different bytes —
  acked-published data loss and a permanently poisoned bootstrap. The
  critical (finding [7]); the same forge reached from the exists arm and
  its TS twins (findings [70] [116] [127]). The writer never asked the
  floor "is this slot below you?" before creating.
- **Stranded sweep.** The log sweep walks *downward* from the head and
  `break`s on the first missing object, so an interrupted sweep leaves a
  hole and every slot below it is stranded permanently — the sweep can
  never resume past the hole (findings [13] [15]). The checkpoint-sweep
  `break`s at the first missing json and can orphan a `.mdb` forever
  (finding [16]).
- **Untrusted clock.** Retention ages by `batch.header.timestamp`, which
  the *writer* controls, so a lying publisher can destroy its own audit
  window (finding [18]).
- **Frozen floor.** `adoptManifest` commits the new etag *before* the
  checkpoint fetch, so a failed fetch freezes the gc floor forever and
  turns swept holes into fake tips (findings [40] [67]).
- **Zombie re-judgment.** `resolveColdPending` re-judges an
  already-published batch when gc collected its slot — a zombie
  republication — because the pending path does not consult the floor to
  learn the batch is already below it (finding [46]).
- **Cadence drift.** The duty resets `log_bytes` to zero unconditionally,
  discarding bytes published during the duty window (finding [98]);
  `ckpt_sum` is never refreshed on `re_establish`, so a post-loss writer
  fires spurious full checkpoint duties (finding [99]).
- **Swallowed refusals.** The duty binary swallows `Published::Refused`
  and `Gc::Refused`, reporting success while the store refuses — the
  scream clause the spec promises is a silent cap (findings [14] [100]);
  the duty tests assert slack arms and never observe the publish through
  the binary, so these are invisible (finding [80]).
- **Leaked lifecycles.** Scratch dirs `{dir}.ckpt{seq}` / `{dir}.duty-ckpt`
  from a crashed process are never swept (findings [103] [126] [134]);
  `duty_busy` leaks `true` on a panic, disabling checkpoints forever
  (finding [134]); the `JoinHandle` vector grows without bound over a
  writer's life (finding [102]); stale sidecar temp files accumulate one
  per crashed pid (findings [75] [86] [111]).

## The target representation

### 1. The floor is an invariant every write path asserts

The floor — the published checkpoint vector — is not advice the reader
consults; it is a **precondition every slot create and every slot delete
asserts against**. A `put_create` at a slot below the floor is refused as
a fault (`the slot is retired`) before it touches the store, so
resurrection (findings [7] [70] [116] [127]) is a refused write, not a
loop that forges history. The floor is read once per transition and
threaded as a value, so it cannot be stale-per-path.

### 2. The sweep is a resumable bottom segment

Deletion is a contiguous bottom segment per braid, tracked by a durable
`swept-below` marker. The sweep walks **upward from the marker toward the
floor**, so an interruption advances the marker to wherever it stopped
and the next sweep *resumes* from there — a hole below the floor is
impossible because the deleted region is always `[0, marker)` and always
contiguous (findings [13] [15] [16]). The checkpoint sweep walks the
immutable Merkle backlink ([40](40-checkpoint-chain.md)) and deletes the
`.json` and `.mdb` as one unit, so no orphan `.mdb` survives (finding
[16]).

### 3. Retention ages by a trusted clock

The age of a slot or checkpoint is measured from a **publish timestamp
the checkpointer stamps**, not from the writer-claimed batch header, so a
lying publisher cannot move its own retention window (finding [18]). The
window is a property of when the object entered the reachable history,
which the trusted checkpointer observes.

### 4. Adopt is atomic; the pending consults the floor

Adopting a manifest pointer and adopting its checkpoint is **one
transition**: the new etag is not committed until the checkpoint it names
is in hand, so a failed fetch leaves the old floor, never a frozen one
(findings [40] [67]). Pending resolution ([30](30-pending-chain.md))
takes the floor as an input and recognizes a batch already below the
floor as *published*, not as a candidate to re-judge (finding [46]).

### 5. The cadence meter subtracts, and refuses are non-success

The duty meter subtracts the snapshot's share rather than zeroing, so
bytes published during the window are not lost (finding [98]);
`ckpt_sum` is re-seeded on `re_establish` from the floor it just adopted
(finding [99]); and the duty's result is a **total sum** whose `Refused`
and `Kept` arms are non-success — the binary's exit code is a total
function of `Ran`, so a swallowed refusal (findings [14] [100]) is a
type error, and the duty test observes the publish and the exit code
through the binary (finding [80]).

### 6. Every resource has an owner

Scratch dirs are leases with expiry, swept at open by any successor
(findings [103] [126] [134]); `duty_busy` is released on unwind (finding
[134]); finished thread handles are reaped in steady state, not only at
`quiesce` (finding [102]); sidecar and store temps live in the reserved
namespace ([20](20-store-contract.md)) and are swept at open (findings
[75] [86] [111]). The rule: a process that crashes leaves no object no
successor will reclaim.

## The invariant

> **A slot below the floor cannot be created and a hole below the floor
> cannot exist; retention time is the trusted history's clock, not the
> writer's; and every scratch object has a successor that reclaims it.**
> The floor is a precondition of writing, the swept region is a
> contiguous prefix, and a refusal is never a success.

Dissolves: [7] [13] [14] [15] [16] [18] [40] [46] [67] [70] [75] [80]
[86] [98] [99] [100] [102] [103] [111] [116] [126] [127] [134]. The
immutable backlink the checkpoint sweep walks is [40](40-checkpoint-chain.md);
the duty's outcome sum and the pending fold are [10](10-protocol-machine.md)
and [30](30-pending-chain.md); the reserved temp namespace is
[20](20-store-contract.md).
