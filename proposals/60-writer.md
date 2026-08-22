# 60 — The writer

A writer is a replica plus the right to create log objects. One commit
path; two placements; the loser algebra (15) between them and the bucket.

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

There is no `Contended` arm: contention is the driver's to absorb via the
loser algebra — a subsumed loss reports the winner's generation, a
disjoint loss republishes silently, a conflicting loss re-judges the
recorded ops, and the host receives either the eventual `Accepted` or the
`Rejected` that a serial execution of *the submitted transaction* would
have produced. The driver never re-invokes `body` — the engine's own law
("the engine ships the outcome, never a loop — retry is host policy")
holds one layer up; a host that wants to decide differently against the
moved world retries its own closure. Bounded republish attempts at the
live tip (default 16 losses — history never counts, per 15; the recorded
basis is 80's crossover performance pin, not a hunch) surface as
`Err::Contention{braid, cause}` where `cause` is a sum matching what
actually exhausted the bound: `HotKey{statement, raw determinant
values}` when the terminal losses were conflicts (the loser owns the raw
values; a blake3 key is not an operable "examine the hot key" handle),
or `SlotRace{tip}` when sixteen fully-disjoint writers simply out-raced
us — an operational signal (declare a reservation relation, move to
resident mode; or just a hot braid needing group commit), not an
expected outcome. At *open*, a pending batch that terminates in
`Err::Contention` stays pending: the store is whole (the identity counts
it — 50), reads serve, and publication retries on the next commit or
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
everywhere (10; 50 states the general form), which is what makes
recovery one integer compare.

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

## The commit discipline (both modes; one sidecar shape)

Per braid, with the chain sidecar of 50 (`pending` is the writer's one
extra field — the same slot in both modes, not a resident specialty):

1. Run `body` against a `Batch`; encode; compute the footprint; set
   `braid_gen = chain[braid].g + 1`, `prev = chain[braid].prev`,
   `ts = max(now_ms, chain[braid].ts)` — the monotone clamp 20's apply
   refuses violations of. Write `pending = {braid, gen, bytes}` (fsync).
2. Apply locally (one `db.write`). `Rejected` → clear pending, return it —
   the network was never touched; rejections are free. Generation
   unchanged (net no-op) → clear pending, return
   `Accepted{durability: Published}` at the current generation — the
   publish law: the effects were already durable in the log via whatever
   put them there.
3. `put_create(log/{braid}/{gen})`:
   - `Created` → advance the chain, clear pending, ack
     `Accepted{durability: Published}`.
   - `Exists` → fetch, compare: byte-equal is *our* earlier ambiguous PUT
     (absorbed, proceed as Created); otherwise the loser algebra (15),
     governed by one law: **a local commit survives in place iff it maps
     to its own slot** — anything else is a fork, and forks are the
     disposable law's business, never bookkeeping's:
     - **Subsumed** → apply the winner's batch locally and let the
       engine decide the arm: generation *unchanged* (the winner's
       effects were exactly ours — the common case, two writers racing
       identical effects) → the winner's slot is accounted by our own
       earlier apply; advance the chain, clear pending,
       `Accepted{durability: Published}` at the winner's generation.
       Generation *advanced* (the winner strictly contains us — its
       residue landed on top of our commit, so one slot now covers two
       local advances) → the store forked; discard, re-open (50), and
       report `Accepted` at the winner's generation — our effects are
       in it. No new detector exists here: the wholeness identity
       (`generation ≡ Σ vector`) decides, at the instant it would
       otherwise silently break.
     - **Disjoint** (15's strict sense: zero shared keys, W intervals
       passing) → the local store holds our commit under the old base;
       the log holds the winner. Discard is not needed: apply the
       winner's batch locally — under full disjointness that apply is
       provably state-changing-accepted, and L8 is the theorem that
       winner-over-ours equals ours-over-winner byte-for-byte, which is
       where L8 is load-bearing, not decorative. Two applies, and two
       slots: the winner's and ours-republished — the identity holds.
       Re-address our batch (`gen+1`, `prev` = the winner's hash, `ts`
       re-clamped against the winner's — the winner is our new
       predecessor), rewrite pending, republish. Ops and footprint are
       untouched: L7 is what makes carrying the old verdict — and the
       batch's publish-law standing at the new base — sound.
     - **Conflict** → the local store diverged in a way L8 does not
       cover: discard the directory, re-open (50) to the winner-current
       state, **re-judge the recorded ops** (one `db.write` of the same
       ops — never a body re-run, so a split sibling can never be
       double-applied and fresh ids are never re-minted), then republish
       on Accepted / return the serial `Rejected`. Fork-discard is the
       price of a real conflict — and only of a real one.

   Discards in the subsumed and conflict arms are cheap by
   construction: checkpoints are content-addressed, so the re-open
   revalidates the locally cached `.mdb` by digest instead of
   re-downloading it (10), and only the braid tails replay.
4. `ack = local` (resident deployments): the ack may move to the end of
   step 2 — 1 ms acks, `durability: LocalPending`, RPO = publish lag,
   bounded by `max_pending` (batches or bytes; constructor knob): beyond
   it, acks stall rather than let the loss window grow silently —
   Aurora's LSN-allocation leash, ours by configuration. `LocalPending`
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
   constants): compact, upload both checkpoint objects, manifest-CAS —
   off the commit loop (10). Races resolve by 10's checkpoint order.

**Recovery at open:** `pending` present → apply it, and the verdict plus
the wholeness instrument decide everything — three arms, all forced:

- **Rejected** → clear pending, publish nothing. The batch was fsynced
  *before* its first judgment (step 1 precedes step 2), so a crash in
  that window resurrects a batch that was never judged; its rejection at
  recovery is the ordinary step-2 rejection, delivered late. Nothing was
  acked; nothing is owed; a born-rejected batch reaching the log is the
  publish law's cardinal sin, and this arm is where it is structurally
  impossible.
- **Accepted, and `generation == Σ vector`** after the apply → the batch
  was born a net no-op (crash landed between step 2's no-op verdict and
  its pending-clear): clear pending, publish nothing — the publish law
  again.
- **Otherwise** (state-changing now, or a no-op absorption of a commit
  that landed pre-crash, distinguished by `generation == Σ vector + 1`)
  → the commit is real and unpublished: catch up to the tip (running the
  loser tests against any intermediate winners, per 15), `put_create`
  (byte-equal `Exists` = already published; a different winner = the
  loser algebra above; `Created` = the crash happened mid-publish), then
  clear.

Composed with 50's catch-up-then-compare, every crash prefix of every
step above recovers through the same two mechanisms — idempotent replay
and create-or-compare — with zero mode-specific arms, and the one
instrument (`generation` vs `Σ vector`) making every call. The
serverless fork that the old design could manufacture (a locally
committed batch the bucket never assigned, invisible to recovery,
silently divergent thereafter) is unrepresentable: every local commit's
bytes are in `pending` until its slot exists.

One recorded exposure rides publish-before-ack: a crash after `Created`
but before the host's ack invites a host-level retry that mints fresh
ids and inserts near-duplicate business rows — set semantics absorbs
byte-identical retries (subsumption), but `reserve`-minted ids make the
rows distinct. Recorded, not solved: the v2 candidate is a Delta-style
`(app, seq)` idempotence key in the header; the trigger is the first
consumer that cannot make its retries byte-identical.

In serverless placements any instance may run this discipline
concurrently; `Exists` is routine and the loser algebra absorbs it. In
resident placements one process holds the role by deployment choice, so
a non-byte-equal `Exists` is *unexpected* — but the protocol's answer is
the same loser algebra, because the discipline is sound under
concurrency whether or not anyone expected it. What the event tells the
resident writer is that it has been deposed (§Adoption): its correct
response is to finish the loss like any loser, drop to `ack =
published`, and surface an operational signal naming both writer ids
from the headers (20) — never a corruption halt, because nothing is
corrupt. The arrangement itself is not represented in the protocol,
deliberately: slot arbitration answers "who writes", and an advisory
writer registry would be a second answer to a settled question.

## Group commit

One commit loop per braid; concurrent `commit` calls partition and queue
per braid. Each drain packs up to 512 host writes or 4 MiB into one batch
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
conflict-loss re-judgment, the same shape either way — triggers
one-by-one fallback for that drain, each caller as its own transaction
in queue order, so an innocent write never fails for a neighbor's
violation. (Recorded v2 upgrade, Aria-inspired:
partition a rejected drain by footprint instead of one-by-one — the
conflict keys name the guilty subset — with two laws stolen from their
measurements: the partition is computed from the *complete* footprints of
every caller in the drain, never short-circuited at the first conflict,
or replicas of the same drain would partition differently; and the
upgrade engages on a measured per-braid rejection-rate moving average,
Aria's own trigger, since their data shows the fallback machinery costs
more than it saves below ~0.4 % aborts and wins 3× under skew.) Linger is
a knob, default **0** — batch whatever is queued, never wait — with the
recorded basis for turning it up: Turso's production batching shows a
deliberate few-ms linger amortizing many writers into one Express PUT at
tenant density (`docs/research/replication-prior-art/turso-notes.md`),
and Calvin's epoch batching paid ~5 ms average delay for its global
order. Hot-braid throughput = PUT rate × batch size; braids multiply it.

## Adoption and failover (resident)

Adopt = open as replica, catch up all braids, read the id-lease counters
(the authoritative floors — no in-log floor ops exist), lease fresh
ranges, begin. No registration anywhere: the previous writer's next
`put_create` hits a non-byte-equal `Exists` and learns it was deposed —
the slot is the fence. A stale previous writer still holding old id
leases can only produce K-conflicts, which the loser algebra converts
into honest rejections. Un-published `ack = local` commits of the deposed
writer are the RPO the host chose (`durability: LocalPending` was in its
hands the whole time, bounded by `max_pending`).

## Capacity reservations (wiring only — 15 owns the idiom)

Reservations are a schema idiom, not a subsystem: a reservation is a row
in a declared weighted child relation of the hot capacity statement, and
15 §Capacity reservations owns the whole story — the deletion of the
unsound grant-object escrow, the mint/spend/reclaim algebra, and why the
spend's fast path is the W interval test itself. This file wires it:
`Batch::reserve_capacity(statement, parent, units, expiry)` is sugar
over an ordinary insert; spends and reclaims are ordinary commits
through the discipline above; nothing here is special-cased. What
reservations buy is admission fairness on scarce slack; they do not and
cannot reduce *slot* contention — that is what braids, group commit, and
resident mode are for, and conflating the two was the old escrow
section's category error.

## Backups, restated

Nothing here schedules backups. Checkpoints exist for replay speed;
retention (10's `gc`) is the backup policy; PITR is the vector math.
Resident deployments may add block snapshots as belt-and-braces, not as
the story.
