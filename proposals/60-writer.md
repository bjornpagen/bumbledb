# 60 — The writer

A writer is a replica plus the right to create log objects. One commit
path; two placements; one loss path between them and the bucket.

```rust
pub enum Durability { Published, LocalPending }

pub enum Commit<R> {
    // `generation` is the braid generation — the slot number, exactly
    // what a `wait_for` session vector carries. Never the store-wide sum.
    Accepted { value: R, braid: BraidId, generation: u64, durability: Durability },
    Rejected(Violations),
}

pub fn commit<R>(
    &self,
    body: impl FnOnce(&mut Batch<'_>) -> Result<R>,
) -> Result<Commit<R>>;

pub fn commit_split<R>(
    &self,
    body: impl FnOnce(&mut Batch<'_>) -> Result<R>,
) -> Result<(R, Vec<BraidOutcome>)>;

pub enum BraidOutcome {
    Accepted { braid: BraidId, generation: u64, durability: Durability },
    Rejected { braid: BraidId, violations: Violations },
}
```

There is no `Contended` arm: contention is the driver's to absorb
through the one loss path — a loss whose effects the log already holds
re-judges to the engine's net no-op and reports `Accepted` at the
current generation; every other loss re-judges the recorded ops at the
re-opened tip and the host receives either the eventual `Accepted` or
the `Rejected` that a serial execution of *the submitted transaction*
would have produced. The driver never re-invokes `body` — the engine's
own law ("the engine ships the outcome, never a loop — retry is host
policy") holds one layer up; a host that wants to decide differently
against the moved world retries its own closure. The bound is
`LOSS_BOUND` = 16 consecutive loop iterations, each one race at the
then-tip; a historical loss is structurally uncountable, because a
stale writer's re-open IS its catch-up and every iteration races once
at the current head. At the bound, `Err::Contention{braid, cause}`
carries a cause sourced from the terminal re-judgment itself:
`HotKey{statement, values}` when it rejected — the engine's violation
names the statement and the cited fact carries the offending raw
values, engine-produced, never reconstructed — or `SlotRace{tip}` when
the terminal apply was accepted but out-raced, with the batch retained
as `Pending`. Both are operational signals (declare a reservation
relation, move to resident mode), not expected outcomes. At *open*, a
`Pending` batch that terminates in `Err::Contention` stays `Pending`:
the store is whole (`generation(chain)` counts it — 50), reads serve, and
publication retries on the next commit or
refresh; an applied commit is never dropped because the tip was busy. `durability` makes the ack mode part of the value: a consumer
holding `Accepted` can tell RPO≈0 from RPO=publish-lag without knowing
the constructor. `Batch` records typed inserts/deletes; `reserve` draws
from the id lease (10) and the resulting inserts carry concrete values;
id reservations never appear in the log.

**The publish law (10 owns it; here is what the writer does).** A batch
is published only if its local `db.write` advanced the generation. A net-no-op commit — the
engine's `COMMIT_NOOP` arm, detected as `generation unchanged` — is
`Accepted` at the current generation with **no object created**: the
empty commit is not a commit, and the log never contains a slot whose
replay changes nothing. This is what keeps the wholeness identity true
everywhere (10; 50 states the general form:
`generation ≡ generation(chain)`), which is what makes recovery one
integer compare.

## One braid per commit (spanning is a different verb)

`commit` requires the recorded ops to fall in one braid — the normal
case, since the schema's own dependencies put related relations together —
and refuses a spanning batch with `Err::SpanningCommit`, naming the
braids.
`commit_split` is the explicit verb for writes to relations *no statement
relates*: independent per-braid batches, committed sequentially, outcomes
as the vector of per-braid results. Partial completion is semantically
invisible to the theory (L9: no statement can observe cross-braid state) —
but not to the application, which is exactly why splitness is chosen at
the call site rather than inferred: "I thought this was atomic" is
unrepresentable. A host invariant spanning braids is by definition
undeclared — declare it and the braids merge. No cross-braid claim
protocol exists, deliberately.

## The commit discipline (both modes; one chain sum)

Per braid, with the `Chain` of 50 (`Pending` is the writer's one extra
constructor — the same arm in both modes, not a resident specialty).
Durability is `Pending → durable → Settled`, in that order: the batch
is fsynced as `Pending` before any apply; the transition to `Settled`
is written after the re-judgment resolves. A refusal never advances
the chain, because advancing *is* a new vector.

1. Run `body` against a `Batch`; encode; set
   `braid_gen = chain[braid].g + 1`, `prev = chain[braid].prev`,
   `ts = max(now_ms, chain[braid].ts)` — the monotone clamp 20's apply
   refuses violations of. Persist `Pending { vector, batch }` (fsync).
2. Apply locally (one `db.write`). `Rejected` → `Settled`, return it —
   the network was never touched; rejections are free. Generation
   unchanged (net no-op) → `Settled`, return
   `Accepted{durability: Published}` at the current generation — the
   publish law: the effects were already durable in the log via whatever
   put them there.
3. `put_create(log/{braid}/{gen})`:
   - `Created` → advance to `Settled` at the new vector, ack
     `Accepted{durability: Published}`.
   - `Exists` → fetch, compare: byte-equal is *our* earlier ambiguous PUT
     (absorbed, proceed as Created); **anything else is a loss, and
     every loss takes the one path** — discard the local directory
     (the disposable law: the local store holds a commit the log never
     assigned, and a fork is never bookkeeping's business), re-open
     through the replica to the current tip carrying the `Pending`
     batch (re-persisted before any re-judgment, so recovery stays
     crash-idempotent at every prefix), and loop back into step 1:
     re-encode at the new head, re-judge the recorded ops in one
     `db.write` — never a body re-run, so a split sibling can never be
     double-applied and fresh ids are never re-minted. The re-judgment
     is a serial execution, performed, and its verdict is the answer:
     accepted-and-state-changing → publish at the new tip;
     accepted-net-no-op (the moved base already contains our effects —
     the racing-identical-effects case, absorbed by the publish law) →
     `Accepted{durability: Published}` at the current generation with
     nothing published; rejected → the serial `Rejected`. There is no
     second path beside the one: a loser whose effects the winner
     performed re-judges to the engine's no-op commit and the publish
     law already answers it, a fully disjoint loser re-judges to the
     identical verdict and effects, and the measured latency of the
     deleted fast path was *higher* than this general path — the fsync
     floor owns both. Resolution is one fold over `Pending`, shared by
     the detached publisher, the loss-path fallback, and open-recovery.
     A batch already below the floor is *published*, not a candidate
     to re-judge.

   The loss discard is cheap by construction: checkpoints are
   content-addressed, so the re-open revalidates the locally cached
   `.mdb` by digest instead of re-downloading it (10), and only the
   braid tails replay.
4. `ack = local` (resident deployments): the ack may move to the end of
   step 2 — 1 ms acks, `durability: LocalPending`, RPO = publish lag.
   The representation is the `AckMode` sum, `Published | Local`, and it
   is the whole configuration: `Local` means exactly "the ack may
   precede the publish of the one pending batch," and the loss window
   is that batch — structurally depth-1 under the one-slot sidecar, at
   most one drain's worth (≤ the 4 MiB drain cap) — by construction,
   not by knob. `LocalPending`
   means exactly what it says: until the slot exists, the commit can
   still be lost to a crash *or rejected by a conflict loss* (a deposed
   writer's pending batch re-judged against the usurper's history) — the
   arm is the honest name for "not yet final", and there is deliberately
   no per-commit revocation channel behind it: a host that needs to
   *know* each commit's final fate gets it the only place it can truly
   live, in the return value, by using `published`. Default mode is
   `published`.
5. Checkpoint duty: after a publish that crosses 10's cadence (Σ ≥ the
   current checkpoint's Σ + K, or the log-volume bound — 10 owns both
   constants). Compaction's input type is `Settled`; a `Pending`
   checkpointer cannot compact. Compact, digest (blake3 of the
   document's full bytes, `prev` inside the hash), both uploads
   (`put_create`, write-once), and the manifest CAS run **entirely off
   the commit lock**, on a detached duty thread handed a
   proven-consistent view — commits never wait on the duty, and a duty
   that keeps failing screams legibly instead of capping. The duty's
   result is a total sum; `Refused` and `Kept` are non-success — the
   binary's exit code is a total function of it. Races resolve by 10's
   checkpoint order; a loser does not rewrite anything. The cadence
   meter subtracts the snapshot's share rather than zeroing;
   `ckpt_sum` is re-seeded on re-establish from the floor just adopted.

**Recovery at open:** the shared `open` transition matches the `Chain`.
A `Pending` arm applies the batch, and the verdict plus
`generation(chain)` decide everything — three arms, all forced:

- **Rejected** → `Settled`, publish nothing. The batch was fsynced
  *as `Pending`* before its first judgment (step 1 precedes step 2), so a
  crash in that window resurrects a batch that was never judged; its
  rejection at recovery is the ordinary step-2 rejection, delivered
  late. Nothing was acked; nothing is owed; a born-rejected batch
  reaching the log is the publish law's cardinal sin, and this arm is
  where it is structurally impossible.
- **Accepted, and `generation == generation(Settled{v})`** after the
  apply → the batch was born a net no-op (crash landed between step 2's
  no-op verdict and its `Settled` write): write `Settled`, publish
  nothing — the publish law again.
- **Otherwise** (state-changing now, or a no-op absorption of a commit
  that landed pre-crash, distinguished by
  `generation == generation(Pending{v, _})`)
  → the commit is real and unpublished: `put_create` at the pending
  slot (byte-equal `Exists` = already published; a different winner =
  the one loss path above, whose re-open IS the catch-up — one race at
  tip, no intermediate-winner tests exist to run; `Created` = the crash
  happened mid-publish), then `Settled`.

Composed with 50's catch-up-then-compare, every crash prefix of every
step above recovers through the same two mechanisms — idempotent replay
and create-or-compare — with zero mode-specific arms, and the one
instrument (`generation(chain)`) making every call. The
serverless fork that the old design could manufacture (a locally
committed batch the bucket never assigned, invisible to recovery,
silently divergent thereafter) is unrepresentable: every local commit's
bytes are in the `Pending` constructor until its slot exists.

One recorded exposure rides publish-before-ack: a crash after `Created`
but before the host's ack invites a host-level retry that mints fresh
ids and inserts near-duplicate business rows — set semantics absorbs
byte-identical retries (subsumption), but `reserve`-minted ids make the
rows distinct. Recorded, not solved: the v2 candidate is a Delta-style
`(app, seq)` idempotence key in the header; the trigger is the first
consumer that cannot make its retries byte-identical.

In serverless placements any instance may run this discipline
concurrently; `Exists` is routine and the one loss path absorbs it. In
resident placements one process holds the role by deployment choice,
and residency IS `AckMode::Local` — the deposition signal is scoped to
it: a `Local`-ack writer's first non-byte-equal `Exists` means it has
been deposed (§Adoption); its correct response is to finish the loss
like any loser, drop to `ack = published`, and surface the operational
signal naming both writer ids from the headers (20) — never a
corruption halt, because nothing is corrupt. A `published`-ack writer
treats a lost slot as the routine contention it is; no deposition
exists for it to detect. The arrangement itself is not represented in
the protocol, deliberately: slot arbitration answers "who writes", and
an advisory writer registry would be a second answer to a settled
question.

## Group commit

Per-braid queues; concurrent `commit` calls partition and queue per
braid, and the caller holding the core drains its braid's queue. The
shipped shape is recorded honestly: **drains serialize on the one core
mutex across braids** — one LMDB env serializes `db.write` across
braids inside a process regardless, so cross-braid drain concurrency
inside one process was a doc flourish, and the promise is deleted with
the measurement that unmasked it. Each drain packs up to 512 host
writes or 4 MiB into one batch
(one generation, one object; chosen constants — 4 MiB keeps a batch in
Express's sweet spot and a rejection's one-by-one fallback tolerable,
and the group-commit throughput pin re-sizes both) and resolves every
caller with the shared outcome. **The drain is one transaction, by law**: the engine judges the
final state of the composite, so a drain may accept a combination a solo
run would have rejected (one caller's delete curing another's violation —
the engine's own final-state judgment, deferred-constraint style, not a
bug) and verdicts may legitimately differ between packed and solo
execution; Lane 5 pins this with a fixture rather than letting it be
discovered. A rejection of the composite — at the initial apply *or* at a
loss's re-judgment, the same shape either way — triggers
one-by-one fallback for that drain, each caller as its own transaction
in queue order, so an innocent write never fails for a neighbor's
violation. (Recorded v2 upgrade, Aria-inspired: partition a rejected
drain by the composite rejection's violations — the cited facts name
the guilty subset — instead of one-by-one, engaging on a measured
per-braid rejection-rate moving average, Aria's own trigger, since
their data shows the fallback machinery costs more than it saves below
~0.4 % aborts and wins 3× under skew.) There is no wait-to-batch knob:
a drain batches whatever is queued and never waits — the deleted knob's
default was 0, its only nonzero behavior held the commit core through
a sleep, and a knob whose honest range is one value is a false
representation. The recorded reopen trigger for deliberate batching
delay: Turso's production batching shows a few-ms hold amortizing
many writers into one Express PUT at tenant density
(`docs/research/replication-prior-art/turso-notes.md`), and Calvin's
epoch batching paid ~5 ms average delay for its global order — the
day a deployment measures that density, the design reopens off the
commit core. Hot-braid throughput = PUT rate × batch size; braids
multiply it across processes.

## Adoption and failover (resident)

Adopt = open as replica, catch up all braids, read the id-lease counters
(the authoritative floors — no in-log floor ops exist), lease fresh
ranges, begin. No registration anywhere: the previous writer's next
`put_create` hits a non-byte-equal `Exists` and learns it was deposed —
the slot is the fence. A stale previous writer still holding old id
leases can only produce key conflicts, which the one loss path converts
into honest serial rejections. Un-published `ack = local` commits of
the deposed writer are the RPO the host chose (`durability:
LocalPending` was in its hands the whole time, one pending batch wide
by construction).

## Capacity reservations (the schema idiom, owned here)

Reservations are a schema idiom, not a subsystem: declare a reservation
relation as one more weighted child of the hot capacity statement, and
the rights are rows. Mint = an ordinary judged insert, priced against
real slack; spend = delete-reservation + insert children in one commit;
reclaim = an ordinary delete. Every one rides the discipline above, and
a contended spend re-judges like every other loss — one `db.write` at
the re-opened tip — and receives the serial capacity verdict that
prices the children against the real slack. Nothing is special-cased:
conservation is the capacity judgment, expiry is an event in the
arbitration domain, and no new objects, verbs, or obligations exist.
`Batch::reserve_capacity(statement, parent, units, expiry)` is sugar
over the ordinary insert. The deletion record rides with the idiom: the
grant-object escrow design (CAS-claimed grant objects with wall-clock
TTLs and a check-skipping fast path) was deleted as unsound — the
engine cannot see grants, so a grant-ignorant winner can consume
promised slack and the holder's unchecked publish poisons the log;
O'Neil's escrow rights, Indigo's reservations, and Homeostasis's
treaties all became side-cars with their own conservation, revocation,
and fencing machinery, and rows-judged-by-the-theory is the shape that
needs none of it. What
reservations buy is admission fairness on scarce slack; they do not and
cannot reduce *slot* contention — that is what braids, group commit, and
resident mode are for, and conflating the two was the old escrow
section's category error.

## Backups, restated

Nothing here schedules backups. Checkpoints exist for replay speed;
retention (10's `gc`) is the backup policy; PITR is the vector math.
Resident deployments may add block snapshots as belt-and-braces, not as
the story.
