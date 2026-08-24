# 80 — Conformance and gates

Determinism is a pinned oracle agreement; crashes are an iteration over
reified protocol steps; every statement family's contention is raced
against a plain serial execution; every law is a test. All lanes run on
`FsStore` — no cloud
account in the suite; `S3Store` gets a credential-gated smoke lane
(skipped-with-reason otherwise).

## Lane 1 — replay determinism (the base oracle)

Random command sequences from the bench corpus generator, three arrivals
compared at every probed vector: **direct** apply; **replayed** through
the log; **checkpoint-hopped** (checkpoint mid-sequence, restore, replay
tail). Gate: `catalog_digest` triple-equality across ≥ 100 generated
worlds, with the hop arm asserting it actually executed — a silently
skipped hop is a failure, not a pass. A disagreement is a trophy. Lane candidate (recorded): a fourth
oracle from the chase literature — on ground instances, engine refusal
coincides with failure of the naive chase over the final state (every
applicable EGD step is a failure step; TGD satisfaction is trigger
absence), so a ~200-line reference chase is an independent semantics
check on the judgment itself.

## Lane 2 — braid convergence (L9 as a running oracle)

Random multi-braid histories under seeded interleavings all converge to
one `catalog_digest` and one generation — L9's executable shadow, and
the lane's whole content. The order-quotient digest (30) is what makes
the gate sound: two replay orders of independent commits land one
digest by construction, so convergence is byte-testable. Recorded scope
note: the generated corpus keeps its rows string-free so intern
minting — store-state-relative by the aliasing ruling — stays out of
the instrument; the digest itself is sound for string-carrying fixtures
too. This lane is
the executable shadow of the Lean theorem — both must exist; neither
substitutes for the other.

## Lane 3 — the serial-verdict lane

For each statement family, a hand-fixture pair races on a shared base
and the loser's re-judgment must produce exactly the serial verdict,
cross-checked against a plain serial execution of the same two batches
on a fresh store — the verdict IS a serial execution, performed:

- the double-booking FD rejection;
- the dangling-reference verdict per order (target-delete vs
  source-insert, each direction);
- the capacity ceiling and floor rejections;
- the reservation spend and reclaim races;
- the byte-equal absorption of an ambiguous PUT.

Small, total over the statement families, named by scenario. A new
statement class cannot ship without extending this lane.

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
winner's slot that already carries its effects — or is discarded with
the directory, never silently
divergent); every crash-window re-application lands in the engine's
no-op arm (`COMMIT_NOOP` observed via the trace, generation unmoved,
identity exact); recovery converges in one pass. The double-apply lane
runs here too: every batch in a generated history applied twice at every
prefix — digest unchanged, generation unchanged, vector correct (L10 as
an executable oracle). A forgotten crash case is a missing enum arm.

## Lane 5 — contention under the one loss path

Writer fleets of 2/4/8 over one `FsStore`, mixed workloads:
mostly-disjoint (booking different rows) and adversarial (hot key, hot
capacity parent). Every gate is structural — chain contents, digests,
outcomes — never a counter on a dead routing arm:
per-braid logs gap-free, each slot created once, every `prev`
chain hash verified; every acked commit appears exactly once; all
replicas converge (`catalog_digest`); a loss whose effects the winner
already performed re-judges to the engine's net no-op and lands
`Accepted` at the current generation with nothing published; a
disjoint-shaped loss re-judges and publishes with a fresh header at
tip+1 passing every chain check; a conflicting loss produces the serial
verdict; **the wholeness
identity `generation ≡ Σ vector + |pending|` is asserted on every store after every
fixture in this lane** — it is the invariant the loss path must
never bend; the ambiguous-outcome GET-verify law (40) resolves injected
response drops; the loss bound surfaces `Err::Contention` under a
fixture designed to livelock. One counter remains in the writer —
losses, which equals re-judgments by construction — and no fixture pins
any other.

Adversarial additions, each with its published baseline to beat:

- **The Feral storms**: the uniqueness storm at the exact Feral width
  of 64 writers on one hot determinant (their experiment leaked
  70–6,300 duplicates; ours gates **zero** duplicates, one Accepted,
  63 typed FD rejections per round), the round count scaled 100 → 16
  under the recorded wall-clock license (63 discard-and-rebuild
  re-judgments per round); target-delete + 64 concurrent source-inserts
  at the full 100 rounds (their association experiment: up to
  6,400 orphans; ours gates zero orphans, serial verdicts throughout),
  with a non-vacuity counter proving the delete actually won rounds;
  Zipfian 0.99 key skew for the hot-key curve.
- **Packed-vs-solo verdicts**: the drain-composition fixture (one
  caller's delete cures another's violation) pins the drain-is-one-
  transaction law of 60 — the composite accepts where solo rejects, and
  the fixture asserts that this is the documented outcome, not a
  surprise.
- **The stale-pending recovery**: a writer 40 slots behind the tip
  resolves its pending through re-open — which IS its catch-up — plus
  ONE race at tip; zero historical losses counted against the bound
  because a historical loss is structurally uncountable; the `SlotRace`
  and `HotKey` causes of `Err::Contention` each produced by a dedicated
  livelock fixture, the `HotKey` payload sourced from the terminal
  re-judgment's own violation; an open ending in `Err::Contention`
  keeps its pending, serves reads, and passes the wholeness identity
  with the pending term.
- **The lying checkpoint**: a fixture checkpoint whose `.mdb` contains
  one extra row at the correct generation with honestly copied heads —
  refused at fresh open by the `catalog` claim (10), and refused by a
  replay-reaching store's comparison. (The lying *winner* fixture died
  with its subject: the wire carries no claim left to lie in, and a
  hostile batch can only be what its ops decode to.)

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

## Lane 7 — parity goldens (Rust ⇄ TS, the mirrored pair)

Checked-in corpora for the pure functions, regenerated as header + ops:
`encode/decodeBatch`
(every op kind, every tag, boundary values, every refusal — bad magic,
version 1, flags ≠ 0, wrong fingerprint, wrong braid relation, kind 3,
`ChainMismatch` in all three causes (prev, slot,
timestamp)) and `braidsOf`
(multi-component schemas, mirror statements, closed relations excluded,
singleton = serial degenerate, serial-at-statements as typed data).
Byte-exact both directions; refusals
carry the same typed identity.

## Lane 8 — engine-guarantee pins (30)

Intern-mint determinism; fresh-in-command collisions as ordinary
rejections; host-order independence; no-op/rejected commits create no
objects and advance nothing (the publish law's engine half:
`COMMIT_NOOP` ⇒ no generation advance ⇒ nothing to publish — pinned
against the trace names, since the whole recovery design stands on it).

## Lane 9 — fuzz

The batch decoder (offset-free sequential — prove it): arbitrary bytes
and golden mutations; no panic, no overflow, every rejection typed, the
trailing-bytes refusal landing at the exact end of the accepted prefix.
The same harness shape runs over the manifest, checkpoint, and
chain-sidecar parsers, where an accepted mutant must be a **canonical
fixpoint**: parse-then-render reproduces the exact input bytes.

## Born lanes (the two the store unification demanded)

- **Cross-language interop**: one `FsStore` prefix, both drivers — Rust
  writes / TS reads byte-for-byte, TS writes / Rust reads, and a mixed
  fleet of real Node child processes and Rust threads races one prefix
  with create-only exclusivity asserted (exactly one Created per slot,
  every CAS linearized, etags agreeing on every object). The lane that
  makes "one protocol" a fact instead of a sentence.
- **Multi-process TS**: real child processes over one `fsStore` prefix
  — disjoint content ⇒ every ack exactly once in a gap-free chain; a
  shared determinant ⇒ one winner and N−1 typed FD rejections; a child
  killed mid-commit ⇒ the fleet converges and the restarted process
  resolves its pending through the one recovery path. The single most
  load-bearing property for deployment case 5, previously untested
  outside one Node process.

## Law gates (census tier)

Zero-dyn extends to `bumbledb-log`'s own code at zero exemptions; TS
temporal gate (every
exported async awaits a store verb); codec alloc windows
(output buffers only); comment hygiene sweeps the driver sources both
languages; `spec-census.sh` carries the protocol tokens
(manifest fields, op kinds, error identities) across
Rust/TS/docs and the Lean names L9 and L10, wired like
L1–L5. One more census tier, learned from this doc set's own review:
**every law and constant has one owning file and one defining site per
language** (the publish law and cadence in 10, the drain caps and loss
bound in 60, the
lease width in 10, the waitFor poll cadence beside its owner); the
census greps for second full statements and second spellings and
fails on them — other files cite, never restate.

## Lean gates

L9 and L10 exist under their stated names in
`lean/Bumbledb/Txn/Braids.lean`, build in the obligation ledger at 104
rows, and are cited from the driver's braid derivation and recovery
sites the way the engine
cites `DeltaRestriction`. Lane 2 (braid convergence) and Lane 3
(serial verdicts) are CI-required alongside them; the recovery
design (no intent field, no forced cases) does not merge before L10,
and cross-braid service claims do not merge before L9.

## Performance pins (attribution-first; replaces 00's envelope)

Recorded, not asserted: per-braid commit latency (FsStore floor + gated
S3/Express smoke); loss cost — one measured pin already ruled here: the
deleted disjoint fast path's publish measured 67.2 ms end-to-end p50
against 64.3 ms for discard-and-re-judge, the fsync floor owning both,
which is the measurement that licensed the one-path deletion;
group-commit throughput ×
braid count; cold-open vs checkpoint size with parallel braid replay;
probe cost per idle pass. Three more, stolen from the genre's own
figures: the **contention curve** (throughput and re-judge rate vs
hot-key skew, Zipfian 0→0.999 — Aria's Fig. 11 shape; their 39 %-of-
uniform at 0.999 is the number reservations exist
to beat); the **loss ratio** (share of commits resolving as clean
publish / re-judged Accepted / serial reject — the one number RedBlue
and
Homeostasis both lead with); and the **crossover point** (contention at
which loss-rate × re-judge cost exceeds a resident writer's group-commit
throughput on that braid — the recorded basis for the 16-loss bound and
for recommending resident mode). The first release notes carry these
numbers or the release does not happen. The stated baseline to embarrass:
Delta Lake's own published ceiling of "several transactions per second"
per table on standard object storage; per-braid group commit on Express
must beat it by the braid count times the batch size, measured.
