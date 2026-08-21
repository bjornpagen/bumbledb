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
worlds. A disagreement is a trophy.

## Lane 2 — commutativity (L8 as a running oracle)

Generate footprint-**disjoint** pairs (A, B) on one braid (the generator
uses `footprintOf` to filter): apply A;B on one store and B;A on another.
Gate: equal `catalog_digest` and equal verdicts. Then the braid corollary:
random interleavings of multi-braid histories all converge. This lane is
the executable shadow of the Lean theorem — both must exist; neither
substitutes for the other.

## Lane 3 — the conflict matrix, adversarially, cell by cell

For **every cell** of 15's four matrices, a hand fixture pair (A, B) built
on a shared base:

- **Commute cells**: both orders accepted; digests equal; the loser
  algebra's republish path (no re-judgment) taken — pinned by a counter
  on the revalidation entry.
- **CONFLICT cells**: the loser algebra re-judges and produces exactly
  the serial verdict — the double-booking fixture rejects with the FD
  violation; the dangling-reference fixture rejects the `need` loser or
  the `support−` loser per order; the capacity fixture's arithmetic
  matches `Σ` against the slack, both signs, mixed signs, floor and
  ceiling.
- The W-class quantitative boundary: sums exactly at slack (commute) and
  slack + 1 (conflict), for unit and weighted children, `parent±` rows
  included.

Every cell cites its matrix coordinates in the test name. A new statement
class cannot ship without extending this lane (the fixture table is an
exhaustive match over the footprint classes — a missing cell fails to
compile, the house roster discipline).

## Lane 4 — the crash matrices (protocol steps as data)

```rust
enum ServerlessStep { Encode, Footprint, IntentWrite, ApplyLocal, VectorBump, PutLog, Ack }
enum ResidentStep   { Encode, PendingWrite, IntentWrite, ApplyLocal, VectorBump, AckLocal, PutLog, PendingClear }
enum ReplicaStep    { IntentWrite, ApplyLocal, VectorBump }
```

Every proper prefix of each: execute, kill, recover per 50/60's forced
resolutions, assert the postcondition table — no acked commit lost, no
un-acked state observable, sidecar/vector reconciliation lands in exactly
one of its two forced cases, fork-discard leaves no residue. A forgotten
crash case is a missing enum arm.

## Lane 5 — contention and the loser algebra

N writers (2/4/8) over one `FsStore`, mixed workloads: mostly-disjoint
(booking different slots) and adversarial (hot key, hot capacity parent).
Gates: per-braid logs gap-free, each slot created once; every acked commit
appears exactly once; all replicas converge (`catalog_digest`); disjoint
losses never re-judge (counter-pinned); conflicting losses produce serial
verdicts; the ambiguous-outcome GET-verify law (40) resolves injected
response drops; bounded-retry surfaces `ErrContention` under a fixture
designed to livelock.

## Lane 6 — PITR, gc, and the vector

A 500-commit multi-braid history, checkpoints every 64 (vector-sum).
Gates: restore to every recorded vector reproduces its recorded digest;
by-time restore maps through the informational timestamps monotonically;
`gc` with window R deletes exactly the retention law's set per braid; a
restore into a gc'd gap refuses `GapDetected`.

## Lane 7 — parity goldens (Rust ⇄ TS, the protocol trio)

Checked-in corpora for the three pure functions: `encode/decodeBatch`
(every op kind, every tag, boundary values, every refusal — bad magic,
version 1, flags ≠ 0, wrong fingerprint, wrong braid relation, unsorted
footprint, kind 3), `footprintOf` (every class and mode, the W deltas,
closed-statement emptiness), `braidsOf` (multi-component schemas, mirror
statements, closed relations excluded, singleton = serial degenerate).
Byte-exact both directions; refusals carry the same typed identity.

## Lane 8 — engine-guarantee pins (30)

Intern-mint determinism; fresh-in-command collisions as ordinary
rejections; host-order independence; no-op/rejected commits create no
objects and advance nothing.

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
Rust/TS/docs — and the Lean names L6–L9 once they land, wired like L1–L5.

## Lean gates

L6–L9 exist under their stated names, build, and are cited from the
driver's revalidation and republish sites the way the engine cites
`DeltaRestriction`. Lane 2 (executable commutativity) and Lane 3 (matrix)
are CI-required alongside them; Layer-2 optimism (the republish-without-
re-judgment path) does not merge before L7.

## Performance pins (attribution-first; replaces 00's envelope)

Recorded, not asserted: per-braid commit latency (FsStore floor + gated
S3/Express smoke); disjoint-loss cost (intersection + PUT) vs conflict
cost (re-judge) vs the old discard baseline; group-commit throughput ×
braid count; cold-open vs checkpoint size with parallel braid replay;
probe cost per idle pass. The first release notes carry these numbers or
the release does not happen.
