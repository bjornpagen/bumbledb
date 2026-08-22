# 80 — Conformance and gates

Determinism is a pinned oracle agreement; crashes are an iteration over
reified protocol steps; the conflict algebra is adversarially exercised
cell by cell; every law is a test. All lanes run on `FsStore` — no cloud
account in the suite; `S3Store` gets a credential-gated smoke lane
(skipped-with-reason otherwise).

## Lane 1 — replay determinism (the base oracle)

Random command sequences from the bench corpus generator, three arrivals
compared at every probed vector: **direct** apply; **replayed** through
the log; **checkpoint-hopped** (checkpoint mid-sequence, restore, replay
tail). Gate: `catalog_digest` triple-equality across ≥ 100 generated
worlds. A disagreement is a trophy. Lane candidate (recorded): a fourth
oracle from the chase literature — on ground instances, engine refusal
coincides with failure of the naive chase over the final state (every
applicable EGD step is a failure step; TGD satisfaction is trigger
absence), so a ~200-line reference chase is an independent semantics
check on the judgment itself.

## Lane 2 — commutativity (L8 as a running oracle)

Generate footprint-**disjoint** pairs (A, B) on one braid (the generator
filters with `footprint(descriptor, ops)`): apply A;B on one store and
B;A on another.
Gate: equal `catalog_digest` and equal verdicts. Then the braid corollary:
random interleavings of multi-braid histories all converge. This lane is
the executable shadow of the Lean theorem — both must exist; neither
substitutes for the other.

## Lane 3 — the conflict matrix, adversarially, cell by cell

For **every cell** of 15's four matrices, a hand fixture pair (A, B) built
on a shared base:

- **Commute cells**: both orders accepted; digests equal; the loser
  algebra resolves without re-judgment — the subsumed arm where the pair
  shares its effects (F insert×insert on one fid), the republish arm
  everywhere else — pinned by a counter on the re-judgment entry staying
  at zero.
- **CONFLICT cells**: the loser algebra re-judges and produces exactly
  the serial verdict — the double-booking fixture rejects with the FD
  violation; the dangling-reference fixture rejects the `need` loser or
  the `support−` loser per order; the capacity fixture's worst-case
  interval endpoints are matched against both bounds, floor and ceiling,
  widened and unwidened.
- The W-class quantitative boundary: worst-case interval endpoints
  exactly at slack (commute) and slack + 1 (conflict), for unit and
  weighted children, `parent±` rows included — plus the evaporation
  fixtures: a batch whose delete evaporates against the winner (its
  effective delta above its published Δ) must go CONFLICT at the bound
  and commute with headroom; the reservation spend/reclaim pair runs
  both arms; a naive point-Δ test must fail this cell (the fixture
  exists to keep the interval law honest).

Every cell cites its matrix coordinates in the test name. A new statement
class cannot ship without extending this lane (the fixture table is an
exhaustive match over the footprint classes — a missing cell fails to
compile, the house roster discipline).

## Lane 4 — the crash matrices (protocol steps as data)

```rust
enum WriterStep  { Encode, PendingWrite, ApplyLocal, AckLocal, PutLog, ChainAdvance, PendingClear }
enum ReplicaStep { ApplyLocal, ChainAdvance }
```

One writer enum, both modes (`AckLocal` only fires under `ack = local`).
Every proper prefix of each: execute, kill, recover — where recovery is
*nothing but* pending resolution + the ordinary catch-up loop + the
`generation == Σ vector + |pending|` check (50/60; there are no
forced-case tables left to consult). Postconditions: no acked commit
lost; **no rejected and no net-no-op batch ever reaches the log** — the
crash between PendingWrite and ApplyLocal resurrects an unjudged batch,
and its recovery arms (Rejected → clear; born-no-op → clear) are
exercised by dedicated fixtures; no phantom survives (a
locally-committed batch either reaches the log — its own slot, or a
subsuming winner's — or is discarded with the directory, never silently
divergent); every crash-window re-application lands in the engine's
no-op arm (`COMMIT_NOOP` observed via the trace, generation unmoved,
identity exact); recovery converges in one pass. The double-apply lane
runs here too: every batch in a generated history applied twice at every
prefix — digest unchanged, generation unchanged, vector correct (L10 as
an executable oracle). A forgotten crash case is a missing enum arm.

## Lane 5 — contention and the loser algebra

N writers (2/4/8) over one `FsStore`, mixed workloads: mostly-disjoint
(booking different slots) and adversarial (hot key, hot capacity parent).
Gates: per-braid logs gap-free, each slot created once, every `prev`
chain hash verified; every acked commit appears exactly once; all
replicas converge (`catalog_digest`); subsumed losses publish nothing,
report the winner's generation, and hit both engine-decided arms
(identical effects → in-place survive via `COMMIT_NOOP`; strict superset
→ fork-discard) under dedicated fixtures; disjoint losses never re-judge
(counter-pinned) and republish with re-addressed headers that pass every
chain check; conflicting losses produce serial verdicts; **the wholeness
identity `generation ≡ Σ vector + |pending|` is asserted on every store after every
fixture in this lane** — it is the invariant the loser algebra must
never bend; the ambiguous-outcome GET-verify law (40) resolves injected
response drops; bounded-retry surfaces `Err::Contention` under a fixture
designed to livelock.

Adversarial additions, each with its published baseline to beat:

- **The lying winner**: a fixture writer publishes a batch whose
  footprint section understates its ops; the loser must catch it by
  recomputation before intersecting (15) — the fixture fails if any loser
  trusts a carried section.
- **The Feral storms**, at their exact parameters: 64 writers × 100
  rounds inserting one hot determinant (their uniqueness experiment:
  70–6,300 duplicates leaked; ours gates **zero** duplicates, one
  Accepted, 63 typed FD rejections per round); target-delete + 64
  concurrent source-inserts × 100 (their association experiment: up to
  6,400 orphans; ours gates zero orphans, serial verdicts throughout);
  Zipfian 0.99 key skew for the hot-key curve.
- **Packed-vs-solo verdicts**: the drain-composition fixture (one
  caller's delete cures another's violation) pins the drain-is-one-
  transaction law of 60 — the composite accepts where solo rejects, and
  the fixture asserts that this is the documented outcome, not a
  surprise.
- **The evaporating republish** (the adversary trace that forced 15's
  strict-disjointness): a loser sharing one commute-cell F key with the
  winner, its other ops base-redundant — must route to the conflict arm,
  re-judge to a net no-op, publish nothing, and report Accepted at the
  current generation; a point-set implementation that republishes it
  must fail this fixture on the no-op-slot refusal (20) and the identity
  gate.
- **The stale-pending recovery**: a writer 40 slots behind the tip
  resolves its pending through catch-up replay plus one tip attempt —
  zero historical losses counted against the bound; the `SlotRace` and
  `HotKey` causes of `Err::Contention` each produced by a dedicated
  livelock fixture; an open ending in `Err::Contention` keeps its
  pending, serves reads, and passes the wholeness identity with the
  pending term.
- **The lying checkpoint**: a fixture checkpoint whose `.mdb` contains
  one extra row at the correct generation with honestly copied heads —
  refused at fresh open by the `catalog` claim (10), and refused by a
  replay-reaching store's comparison.

## Lane 6 — PITR, gc, and the vector

A 500-commit multi-braid history, checkpoints every 64 (vector-sum).
Gates: restore to every recorded vector reproduces its recorded digest;
every checkpoint's `catalog` claim verifies at open and against a
replay-reaching store; the checkpoint backlink chain walks from the
manifest to every retained checkpoint (no LIST anywhere in the suite —
grep-enforced); by-time
restore maps through the publish-clamped timestamps, and a fixture batch
with a non-monotone timestamp is *refused at apply*, not mapped around;
`gc` with window R deletes exactly the retention law's set per braid; a
404 at-or-below the current checkpoint's vector refuses `GapDetected`
while the same 404 above it reads as the tip — both directions pinned; a
hibernated-replica fixture (vector far behind a gc'd horizon) must
refuse rather than serve stale reads as fresh.

## Lane 7 — parity goldens (Rust ⇄ TS, the protocol trio)

Checked-in corpora for the three pure functions: `encode/decodeBatch`
(every op kind, every tag, boundary values, every refusal — bad magic,
version 1, flags ≠ 0, wrong fingerprint, wrong braid relation, unsorted
footprint, kind 3, `ChainMismatch` in all three causes (prev, slot,
timestamp), a delta on a non-W entry [must be unparseable, not refused]),
`footprintOf` (every class and mode via the per-class suffixes, the W
deltas and their per-key merging, closed-statement emptiness), `braidsOf`
(multi-component schemas, mirror statements, closed relations excluded,
singleton = serial degenerate). Byte-exact both directions; refusals
carry the same typed identity.

## Lane 8 — engine-guarantee pins (30)

Intern-mint determinism; fresh-in-command collisions as ordinary
rejections; host-order independence; no-op/rejected commits create no
objects and advance nothing (the publish law's engine half:
`COMMIT_NOOP` ⇒ no generation advance ⇒ nothing to publish — pinned
against the trace names, since the whole recovery design stands on it).

## Lane 9 — fuzz

The batch decoder (offset-free sequential — prove it): arbitrary bytes
and golden mutations; no panic, no overflow, every rejection typed. Same
harness over the manifest parser and the footprint recomputation
comparator.

## Law gates (census tier)

Zero-dyn extends to `bumbledb-log`'s own code; TS temporal gate (every
exported async awaits a store verb); codec/footprint alloc windows
(output buffers only); `spec-census.sh` gains the protocol tokens
(manifest fields, op kinds, footprint classes, error identities) across
Rust/TS/docs — and the Lean names L6–L10 once they land, wired like
L1–L5. One more census tier, learned from this doc set's own review:
**every law and constant has one owning file** (the publish law and
cadence in 10, the algebra and its constants in 15, the drain in 60, the
lease width in 10); the census greps for second full statements and
fails on them — other files cite, never restate.

## Lean gates

L6–L10 exist under their stated names, build, and are cited from the
driver's revalidation, republish, and recovery sites the way the engine
cites `DeltaRestriction`. Lane 2 (executable commutativity) and Lane 3
(matrix) are CI-required alongside them; the optimism path
(republish-without-re-judgment) does not merge before L7; the recovery
design (no intent field, no forced cases) does not merge before L10.

## Performance pins (attribution-first; replaces 00's envelope)

Recorded, not asserted: per-braid commit latency (FsStore floor + gated
S3/Express smoke); disjoint-loss cost (intersection + PUT) vs conflict
cost (re-judge) vs the old discard baseline; group-commit throughput ×
braid count; cold-open vs checkpoint size with parallel braid replay;
probe cost per idle pass. Three more, stolen from the genre's own
figures: the **contention curve** (throughput and re-judge rate vs
hot-key skew, Zipfian 0→0.999 — Aria's Fig. 11 shape; their 39 %-of-
uniform at 0.999 is the number the W arithmetic and reservations exist
to beat); the **conflict ratio** (share of commits resolving as
subsumed / republish / re-judge / reject — the one number RedBlue and
Homeostasis both lead with); and the **crossover point** (contention at
which loss-rate × re-judge cost exceeds a resident writer's group-commit
throughput on that braid — the recorded basis for the 16-loss bound and
for recommending resident mode). The first release notes carry these
numbers or the release does not happen. The stated baseline to embarrass:
Delta Lake's own published ceiling of "several transactions per second"
per table on standard object storage; per-braid group commit on Express
must beat it by the braid count times the batch size, measured.
