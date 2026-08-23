# 40 — Writer and replica repairs (house law, knob deletions, honest parts)

Everything here is independent of 10's cut except where noted; the writer
items land in the same rewrite of writer.rs that 10 forces, so one lane
owns both (90).

## Unbounded repair, legible scream (owner law restored)

The discard-and-re-pull loops cap at 8 and hard-error
(crates/bumbledb-log/src/replica.rs:439-452, writer.rs:1998, 2061). The
law: a repair loop repairs forever and screams legibly — a warning every
Nth attempt naming the repeating signature, an alarm when the signature
recurs. The caps, their counters, and the fabricated
"did not converge" infrastructure error all die. (A poisoned slot still
wedges its braid by the corruption-class refusal path — that is a refusal,
not a repair; the loop this law governs is discard-and-re-pull against a
healthy remote, which either converges or keeps honestly screaming that
the store keeps tearing.)

## Checkpoint duty off the commit lock

10/60 promise duty never stalls a hot braid; the engine's compact is
read-txn-pinned so writers keep flowing — the stall is the DRIVER's lock
shape: compact() plus the full .mdb read run under the global core lock on
the commit path (writer.rs:1906-1928). Restructure: the commit path only
*detects* cadence crossing and hands the duty a consistent view; compact,
the digest, both uploads, and the manifest CAS run entirely off the commit
lock (the detached thread the upload half already uses). The
checkpoint-order CAS race semantics are unchanged, and the orphan fix
below rides the same restructure.

## Orphaned checkpoints closed structurally

gc discovers checkpoints only via the backlink walk (gc.rs:116-138), and
the duty reads `prev` from the manifest at upload time
(writer.rs:1950-1955), so a CAS race can orphan an intermediate checkpoint
forever under the LIST ban. Fix the representation: on a checkpoint-order
CAS `Moved`, re-read the incumbent, **rebuild the checkpoint json with
`prev` = the incumbent actually being replaced**, re-upload (content
addressing makes that a new immutable object; the loser json becomes
ordinary gc fodder), and CAS again. `prev` becomes proven-by-CAS rather
than hoped; every retained checkpoint is reachable from the manifest by
construction.

## Knob deletions (a knob that cannot express its range is a false representation)

- **`linger` dies.** Default 0, zero consumers, and its only nonzero
  behavior is a bug (the sleep holds the commit core —
  writer.rs:986-999, 1063-1066). Deleting the knob deletes the bug. The
  Turso-density batching argument stays in 60 as the reopen trigger.
- **Both `max_pending` knobs die.** The pending slot is structurally
  depth-1, so `max_pending_batches` distinguishes only 0 from ≥1 and
  `max_pending_bytes` gates a single batch already capped by the 4 MiB
  drain bound (writer.rs:1272-1289). The honest representation is the
  `AckMode` sum alone: `Published` | `Local`, where `Local` means exactly
  "the ack may precede the publish of the one pending batch." 60 amends;
  the loss window is one batch, ≤ the drain cap, by construction.
- **Drain serialization recorded, not fixed.** Per-braid queues exist but
  drains serialize on one core mutex (writer.rs:645). F11 measured the
  real serial point: one LMDB env serializes db.write across braids inside
  a process regardless. Cross-braid drain concurrency inside one process
  is a doc flourish; 60 records the shipped shape and deletes the promise.
- **Deposition stays scoped to `AckMode::Local`** (writer.rs:1405-1406) —
  in the shipped representation local acks ARE residency; a published-ack
  writer treats a lost slot as routine contention. One sentence in 60.

## The ErrStore identity (the one WP-1 bug that survives 10's cut)

ts-log's `wrapStore` (errors.ts:134-138) wraps vendor errors such that the
exported `ErrStore` sentinel is never in any cause chain —
`errors.is(e, ErrStore)` is always false, and the working predicate
`isStoreFailure` (errors.ts:140-149) is unexported with zero callers.
Representation fix: the sentinel goes INTO the chain (wrap the vendor
error with `ErrStore` as ancestor), the redundant predicate dies, and a
test pins the match **by identity**. The README's advertised idiom becomes
true. (WP-1's other two TS bugs — history-counted losses and the empty
hot-key payload — are deleted structurally by 10, not fixed.)

## Canonical-fixpoint parity for the sidecar

`Checkpoint::parse` is canonical (order-strict) after f9's conviction;
`Chain` sidecar parse is order-lax (sidecar.rs:162-182 vs
manifest.rs:314-316). One wire law: every canonical document parses to a
value that re-renders byte-identically, and accepted mutants are refused.
Hold the sidecar to it, and give it the same f9 mutation gate the
checkpoint parser has.

## The small-parts sweep (each named, each with its verb)

- Rotated `store-*` LMDB dirs accumulate in replica dirs
  (ts-log/src/replica.ts:468-471): sweep dead rotations at open; the
  disposable law says cache directories do not hoard corpses.
- `waitFor` busy-polls at 20 ms: fine — but the number becomes a named
  constant with its owner and enters census lane (j).
- The stale rationale comment at
  crates/bumbledb-log/tests/f5_contention.rs:698-704 (half-outdated since
  the order-quotient digest): the file is being rewritten by 10 anyway;
  the comment dies with its fixture generation.
- f4's `crash_mid_publish` fixture asserts the end state but not the
  byte-comparison mechanism: assert the absorption arm actually ran.
- The association storm proves no delete ever won a race (vacuity): count
  and assert at least one target-delete victory.
- F1's checkpoint-hop arm is Option-guarded and can silently skip: assert
  the hop executed.
- The dual-corruption digest case (two M entries claiming one row id)
  degrades silently: make it the loud `MembershipDesync` its sibling case
  already is.
- The tenants byte budget is measured once at open
  (ts-log/src/tenants.ts:121): record "advisory, measured at open" where
  the option is declared — or make it live; the lane picks and records.
