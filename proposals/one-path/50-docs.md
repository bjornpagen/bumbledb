# 50 — The amendment matrix (written once, after the code shapes are final)

The numbered docs are the product's spec; this pass's code changes make
several of them false. Amendments land as ONE lane at the end so every doc
is written once, in its own voice, describing the final shape — never an
intermediate one. Where the audit found doc-vs-code disagreement and the
code was right, the law changes openly.

## proposals/15-conflict-algebra.md — DIES WHOLE

With the matrices' operational consumer deleted (10), the W arithmetic
deleted, the footprint deleted from the wire, and the loser algebra reduced
to one sentence, 15 has no surviving subject. Content disposition, mapped:

- The braid derivation details it restated → already owned by 10-protocol
  (one-owner law; the restatement dies unmourned).
- The reservations idiom (§Capacity reservations) → moves to 60-writer.md
  beside `reserve_capacity`, restated for one-path: a reservation is an
  ordinary row in a declared weighted child relation; mint, spend, and
  reclaim are ordinary commits; a contended spend re-judges like every
  other loss and receives the serial capacity verdict. The escrow deletion
  record moves with it.
- The empty-determinant degeneracy (serial-at-statements as typed data) →
  moves to 10-protocol §Braids, whose derivation already returns it.
- The fkey construction, the four matrices, the interval algebra, the
  loser algebra, the statement-class boundary, L6–L8's statements → the
  deletion record in one-path/10 and git history. The matrices remain
  TRUE as theory; the product no longer computes them, and prose about
  computations the system does not perform is the exact class the doc
  funeral killed.
- The Feral grounding (uniqueness/orphan leaks vs our serial verdicts) →
  survives, re-homed into 00's honesty paragraph: the answer to Feral was
  never the fast path; it was typed serial verdicts, which one-path
  delivers through the only path there is.

The README table row for 15 dies with it; the set returns to its original
00–90 shape.

## proposals/00-product.md

- **The thesis paragraph rewrites.** The product is: the braided object
  log — durability, replication, backup, PITR, sharding, and
  multi-writer concurrency derived from the schema's statement graph
  (braids, L9), with serial verdicts under contention and replay-forward
  recovery (L10). The footprint-as-published-certificate sentence dies;
  "concurrency is extracted from the theory" now names braids alone,
  which is what it always operationally named.
- **Law 3 (footprints carried and checked) dies.** Laws renumber or gap —
  the doc's own style decides; nothing may cite the dead law.
- **Law 4 rewrites** to the one-path promise: a CAS loser's outcome equals
  a serial execution of the submitted transaction — Accepted at the
  realized generation (including the net-no-op case via the publish law)
  or the serial Rejected with violations as data. "Losers keep their
  work" dies as a sentence; losers keep their *outcomes*.
- The non-goals gain one line: quantitative conflict avoidance (the
  interval algebra) deleted by the one-path ruling; reopen trigger in
  one-path/10 (measured network-store contention where loss resolution
  dominates).

## proposals/10-protocol.md

- §Log objects: the loser algebra reference becomes the one-path rule
  (byte-equal absorption, else discard/re-open/re-judge); the "batch
  carries its footprint section; replicas recompute and refuse" sentence
  dies.
- §Braids gains the serial-at-statements paragraph from 15.
- The failure-semantics table: the `FootprintMismatch` row dies; the CAS
  `Exists` row rewrites to the one-path rule; everything else stands.

## proposals/20-command-codec.md

- The batch layout drops `fp_count` + footprint entries; the batch is
  header + ops. Version stays 2 (never released; recorded in the doc).
- The footprint-section subsection, the per-class suffix grammar, the
  derivable-section tripwire argument, and the W delta prose die.
- The determinism laws renumber: raw-values-only, canonical commit order,
  intern-mint, fresh-in-command, cross-braid irrelevance, replay
  idempotence all stand; "footprint recompute equality" dies.
- §Apply loses step 2 (recompute) and its refusal.

## proposals/30-engine-seams.md

- The digest paragraph rewrites to the shipped order-quotient algorithm
  (row ids rendered through M-namespace fact identities, canonical entries
  sorted, dictionary raw behind) — commit 90bd6004's contract, stated as
  the spec it now is. The surface law is reaffirmed: one #[doc(hidden)]
  function IS the engine seam.
- One added line blesses the ts/crate `blake3_hash` napi export
  (`internalBlake3`): engine-linked hashing lent across the FFI for the TS
  driver's store etags — consumer named, seam list complete.
- The footprint-is-not-a-seam paragraph simplifies: there is no footprint;
  the sentence becomes the record that the engine never learned
  replication exists, which remains exactly true.

## proposals/40-object-store.md

- The FsStore item rewrites to the unified protocol of one-path/30:
  O_EXCL temp + link(2) create-only; computed blake3 etags, never stored;
  the pid-lockfile CAS with dead-owner breaking under the one-machine law;
  fsync discipline restated. The flock and .etag sentences die.
- The vendor row for local fs gains the interop sentence: one on-disk
  protocol, two conforming implementations, raced in conformance.

## proposals/50-replica.md

- The self-disagreement resolves the way the code did: read legality
  follows PROVENANCE (checkpoint-seeded and bootstrapped stores are whole
  by construction; only a pre-existing dir is in the unproven open phase).
  The failure-table row rewrites to match; the poisoned-slot
  infinite-discard hazard is the recorded reason.
- The apply discipline loses the footprint recompute step.

## proposals/60-writer.md

- The commit discipline's Exists arm rewrites to the one-path rule; the
  subsume/disjoint/conflict subsections die; the pending-recovery arms
  simplify (no intermediate-winner loser tests).
- The bound: 16 consecutive loop iterations, `Err::Contention` causes
  re-sourced from the re-judged violations (10).
- Knobs: `linger` and `max_pending` die; `AckMode` is the representation;
  the drain serialization is recorded as shipped; deposition's Local-ack
  scoping is recorded. The Turso-density trigger survives as the group
  commit section's reopen line.
- Gains the reservations idiom from 15, restated for one-path.

## proposals/70-typescript.md + ts-log/README.md

- The mirrored-pure-trio section becomes the mirrored pair: codec and
  braids (footprintOf dies). Parity language updates.
- The README's group-commit remedy line dies (TS ships no group commit;
  requirement questioned, trigger recorded); the ErrContention runbook
  rewrites for the violation-sourced payload.
- `openTenants`' real options ({dir, theory}) and the advisory
  once-measured byte budget: recorded honestly where declared.
- The error identity roster: `ErrFootprintMismatch` dies; the rest stand.

## proposals/80-conformance.md

Rewritten to the pass's lane reality (one-path/60 owns the details): lane
2 reshapes, lane 3 shrinks to the serial-verdict lane, the lying-winner
fixture dies with its subject, lane 7's corpus loses the footprint
sections, the two new lanes (interop, TS multi-process) enter, and the
Lean-gates section re-points to L9/L10 at ledger 104.

## proposals/90-rollout.md

Receipts re-issued at the end of the pass (one-path/90 owns the shape).
