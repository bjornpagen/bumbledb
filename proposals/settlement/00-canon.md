# 00 — Canon: the law and the landed representations

The successor of the retired PRD set (`00-product` … `90-rollout`) and
the retired cutover set (`representation-first-cutover/00–70`). Both
campaigns closed; this document is what survives them: the product law,
stated once, and the representations that now carry it, stated as facts
of the present tree. The build implements this document or reports the
gap; it never improvises past it.

## What it is

`bumbledb-log` makes a bumbledb store durable, replicated, backed up,
point-in-time recoverable, and **concurrently writable** by representing
the store's **history** as data in an object store: per-braid command
logs, checkpoints, one CAS manifest. The braids are the product: the
schema's statement graph decomposes into connected components that
provably never conflict (L9) — concurrency is derived from the declared
theory, machine-checked, never managed. Not a CRDT (invariants are never
weakened); not consensus (arbitration is object-store CAS per log slot —
no quorum, no term, no leader). There is no server; a resident writer is
a deployment *mode* chosen for 1 ms acks, not a component.

## The laws

1. **The log is the write-ahead truth.** A commit is acknowledged when
   its log object exists; local LMDB is a materialized view. (Resident
   `ack = local` mode trades this visibly: `durability: LocalPending`,
   loss window one pending batch by construction.)
2. **Index = applied count per braid; generation = the sum.** Each braid
   carries an independent chain; the engine's `GenerationId` equals the
   counts' sum exactly, because only state-changing batches publish (law
   6). Statements never span braids (L9).
3. **Losers keep their outcomes.** A CAS loser re-judges its recorded
   ops at the winner-current tip through the one loss path and receives
   exactly the verdict a serial execution would have produced —
   `Accepted` at the realized generation (including net-no-op via the
   publish law) or the serial `Rejected` with violations as data.
4. **Replay is deterministic.** Same checkpoint + same braid prefixes ⇒
   byte-identical catalog content, any interleaving of braid application.
5. **One way per question.** Slot arbitration: create-only PUT on the
   next log key. Tip discovery: forward probing. Checkpoint publication:
   manifest CAS. Loss resolution: byte-equal absorption, else
   discard-re-open-re-judge. Nothing else.
6. **The empty commit is not a commit.** A batch publishes only if its
   local application advanced the generation; the log never contains a
   no-op slot. Consequence: `engine generation ≡ Σ vector` on every
   honest store.
7. **Recovery is replay.** Re-application of an applied batch is a
   proven no-op (L10), so every crash window heals by replaying forward
   through the ordinary catch-up loop. The one residual instrument is
   the wholeness identity `generation ≡ generation(chain)` — one
   compare, every verdict; its failure is a discard, never a repair.
8. **Every read is a serial prefix.** A replica at any vector serves a
   real admitted state satisfying every declared statement. Freshness is
   the only staleness dimension; cross-instance read-your-writes rides
   `wait_for` with a session vector.

Honesty about the residue: commits pay one conditional PUT per braid
slot; concurrent writers on one braid serialize their claims; a lost
claim pays a cache-warm re-open plus one local re-judgment. Braids
remove every cross-braid interaction; the per-braid total order is what
makes verdicts serial and replay deterministic.

## The five deployment cases

1. **Next.js on Vercel Fluid** — replica singleton per instance;
   microsecond local reads, serverless commits, the loss path absorbs
   races.
2. **Embedded macOS (Apple Silicon)** — engine as today; the log as
   optional sync/backup in resident mode, via napi or the C ABI.
3. **Long-lived server** — resident mode: 1 ms local acks, RPO≈0, PITR,
   bucket-as-backup.
4. **Distributed per-tenant** — tenant = prefix; braids shard within a
   tenant; control-plane tenant carries shared reference data;
   cross-tenant analytics is the heap arm.
5. **Local fleet** — N writer processes, one machine, one `FsStore`
   prefix; no network in the loop; one-braid theories serialize on a
   link and the loss path absorbs the rest.

## Non-goals

No consensus or leases-as-truth (id-leases are avoidance; correctness
never depends on them). No quantitative conflict avoidance (deleted
whole by the one-path ruling; reopening is a design campaign, never a
revert). No schema migration (fingerprint mismatch refuses). No
cross-braid atomicity (spanning writes are the explicit `commit_split`
verb, never inferred). No compression (reserved flag bit).

## The representations (landed — this section replaces the retired specs)

The 141-defect bug bash proved the protocol's bugs were shadows of six
representational decisions. All six are in the tree; their invariants
are the binding contract:

1. **The protocol is one machine.** One transition table both drivers
   execute: `ReplicaState`/`WriterState` are `Mounted | Unmounted`
   sums; wedging is per-braid; `Reseed` is an arm; one stepper carries
   heartbeat, wholeness, pass counting, and the disposed check, shared
   by `refresh`, `waitFor` (= refresh + predicate), catch-up
   (round-robin by construction), and open. The id-lease is one algebra
   (`OverWidth | Exhausted | Drawn`; the body runs exactly once).
   Deposition derives from the fixed-layout slot header, never a body
   decode. The scream trips on a *set* of repair signatures. The writer
   births the store; a replica refuses `ManifestMissing`. *A behavioral
   divergence between the drivers is a conformance failure, not a design
   choice — neither driver defines arms.*
2. **The store is one contract.** Five verbs; outcomes are total sums
   with `Ambiguous` resolved by the GET-verify law (S3 409 is
   `Ambiguous`, never proved); success means fsynced-and-visible, object
   and parent dir, every impl; keys are one parsed grammar with the
   `~`-reserved temp/lease namespace disjoint by construction and swept
   at open; the mutation lock is a fenced CAS lease
   (`{holder, token, expires}`) broken only by expiry through the
   store's own CAS — liveness is `Alive | Dead | Unknown` and `Unknown`
   never breaks; every write carries its fencing token; a replica
   directory has one leased owner and a refcounted handle whose
   `Disposed` arm has no verbs. *A stored object cannot exist without
   its body, cannot be acked without being durable, and cannot be locked
   by a probe of a foreign process.*
3. **Pending is a chain constructor.** `Chain = Settled{vector} |
   Pending{vector, batch}`; `generation()` is a total function of the
   value (`Vector.sum()`, or `sum + 1` under `Pending`); `Vector` owns
   the algebra — `sum` (the one Overflow site), `dominates`, `order`,
   `at`/`advance` — so wholeness, wait_for, checkpoint order, and the
   floor are calls, not loops; compaction's input type is `Settled`;
   resolution is one fold returning remaining segments as data; the
   sidecar is a binary v:3 document; the sidecar read is `Absent |
   Fault | Corrupt | Read` with `Absent` = NotFound only; durability
   order is `Pending → durable → Settled`. *There is no addend a
   reader can forget, because there is no addend.*
4. **The checkpoint chain is immutable and content-addressed.** The
   manifest, checkpoint document, and sidecar are binary v:3 records
   at the keys `manifest`, `ckpt/{digest}`, and `chain` (the `.mdb`
   sibling keeps its suffix). `prev` is inside the content hash; a
   document is written once with `put_create` (`Exists` is
   byte-identity); `blake3(bytes)` has no canonicalization clause,
   because one encoder produces one byte string; the manifest points
   at the head of the Merkle list; a Kept or refused loser deletes
   its own digest pair; crash candidates are named in the
   `~lease/ckpt-scratch` document any successor sweeps at open (see
   RULINGS). The catalog claim is audited at the one seed transition.
   Compact→publish is one transition; the detached duty binary and
   the resident cadence are two entries into it. *The spine cannot
   be rewritten and reachability is decidable.*
5. **The gc floor is a write-path invariant.** A below-floor slot create
   is refused `SlotRetired` before it touches the store; the sweep is a
   resumable contiguous bottom segment `[0, marker)` walked upward;
   retention ages by the publish instant stamped at the winning CAS;
   adopt commits the etag only after the checkpoint is in hand; a
   pending the floor covers is published, not re-judged; the duty's exit
   code is a total function of its outcome and a refusal screams; every
   scratch object has a successor that reclaims it. *A slot below the
   floor cannot be created and a hole below the floor cannot exist.*
6. **Parse, don't validate, at the codec.** One grammar; every
   protocol object — batch, manifest, checkpoint, sidecar — is a
   sentence of it, opened by version byte 3; numbers are exact
   (`u64`/`bigint`, checked sums — overflow is a refusal of
   `Vector.sum()`, not a wrap); digests are `[u8; 32]`; a row vector
   cannot claim more rows than its bytes back; a string cell is
   `WellFormedUtf8` from a fatal encoder; fixed intervals are
   half-open (the ceiling is not a value); `schema_file`, the duty
   argv, and the Lambda request are parsed grammars; refusals are
   named identities (`Malformed`, `Version`, `UnknownBraid`,
   `Overflow`) spelled identically by both drivers and pinned by the
   conformance inventory. Machines write binary; humans write text:
   protocol objects are the one binary grammar, and the theory file
   and `duty inspect` output are the text half. *One codec cannot
   decode one byte string to two values.*

Conformance executes the artifacts: both drivers walk the one v:3
inventory (`crates/bumbledb-log/conformance/v3/`), the crash matrices
execute the same step tables, the parity lane asserts identical named
outcomes on identical bytes, and the store smoke lane ties outcomes to
persisted bytes and cleans its bucket.

**The remaining delta between this canon and the tree is the lockstep
receipt** ([50-proof-as-gate.md](../lockstep/50-proof-as-gate.md)).
When that receipt lands, this document describes the tree with no
remainder.
