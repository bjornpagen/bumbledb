# 10 — One loss path

The centerpiece. The loser algebra ships three arms — subsume, republish
without re-judgment, discard-and-re-judge — plus the pairwise intersection
that routes between them and the footprint wire section that feeds the
intersection. All of it collapses to **one path**, and everything that
existed only to route around that path dies, wire to theorem.

## The ruling and its proof

**On a lost slot: fetch the winner and byte-compare. Byte-equal means the
object is ours (an ambiguous PUT absorbed — 40's GET-verify law, unchanged).
Anything else: discard the local directory, re-open through the replica to
the current tip, re-judge the recorded ops in one db.write, then publish on
accepted-and-state-changing, report Accepted at the current generation on
accepted-net-no-op (the publish law), or return the serial Rejected.**

Three facts make the deleted arms pure weight:

1. **Subsumption is the general arm plus the publish law.** A loser whose
   effects the winner already performed re-judges to an empty net delta —
   the engine's no-op commit — and the publish law already answers
   `Accepted` at the current generation with nothing published. The
   dedicated arm, its winner-strictly-contains fork detection, and the
   engine-decided survive-vs-fork machinery reproduce by hand what the
   general arm does by construction.
2. **Republish-without-re-judgment is provably outcome-identical.** L7's
   acceptance form says exactly that a fully key-disjoint loser re-judged
   at the moved base lands the same verdict and the same effects. An
   optimization whose license is "the slow path returns the identical
   answer" optimizes nothing but latency —
3. **— and the latency win does not exist.** F11, measured on the shipped
   store: disjoint-loss republish **67.2 ms** end-to-end p50; conflict-loss
   discard-and-re-judge **64.3 ms**. The fsync floor owns both; the
   checkpoint-revalidating re-open is cache-warm. The fast path measured
   *slower*. A fast path that is not fast and changes no outcome is two
   ways to do one thing.

## The cascade: what dies (Rust and TS twin alike)

The one-path rule orphans the intersection; the orphaned intersection
orphans the W arithmetic; the orphaned arithmetic orphans the wire delta;
and with routing gone, the footprint section's only remaining reader is the
recomputation that checks the footprint section — harness-as-consumer,
banned (00). The cut runs to the bottom:

1. **The subsume and republish arms** — `crates/bumbledb-log/src/writer.rs`
   loser region: the F-subset test, the winner-apply-in-place, the
   generation-moved fork detector, the header re-address machinery
   (gen+1, prev-rewrite, ts re-clamp, pending rewrite), and the
   `counts_live` live-vs-history loss accounting (writer.rs:1430-1447).
   Under one path a stale writer's re-open IS its catch-up: each loop
   iteration races once at the then-tip, so a historical loss is
   **structurally uncountable** — the TS history-counting bug
   (ts-log/src/writer.ts:366) is deleted, not fixed. The 16-loss bound
   survives as a plain iteration count on the one loop;
   `Err::Contention` and its causes survive (payload re-sourced below).
2. **`src/intersect.rs` and `ts-log/src/intersect.ts`, whole.** Subsumed /
   Disjoint / Conflict, strict key-disjointness, the W interval test,
   `capacity_cell`, `capacityCommutes`, `BaseMeasure`,
   `CapacityMeasureMissing`, and both live call sites that passed the
   empty measure map (writer.rs:1416-1422, 1555-1561).
3. **The W arithmetic in the footprint layer**: evaporation intervals
   (`capacity_profiles`), per-key signed delta merging, the
   `DeltaOverflow` refusal.
4. **The footprint wire section, whole** (20's `fp_count` + entries): the
   emission rules, `fkey`/`fid` hashing, the per-class suffixes, the
   sorted-dedup refusals, `FootprintMismatch`, apply's
   recompute-and-refuse step, `footprint(descriptor, ops)` and
   `footprintOf` in both languages, their corpus goldens, parity sections,
   alloc windows, and emission tests. The batch becomes header + ops. The
   version stays **2**: no release ever shipped this format; it never
   existed outside this tree.
5. **Three of the four writer counters** (subsumptions, republishes,
   disjoint_verdicts). One remains: losses — which now equals
   re-judgments by construction.
6. **The lying-winner defense class**, because there is no carried claim
   left to lie in and no routing decision to steer. A hostile batch can
   still only be what its ops decode to; the chain discipline and the
   apply battery (version, flags, fingerprint, ChainMismatch three ways,
   op-relation-in-braid) survive untouched — they judge the header and
   ops, not the deleted section.

## What survives, and why it is untouched

- **Braids** — the operational concurrency, derived from the descriptor,
  proven never to conflict (L9). The thesis's load-bearing half; nothing
  here reads footprints.
- **The chain discipline, publish law, pending recovery, checkpoints,
  PITR, gc, leases, tenants, the replica lifecycle** — one-path changes
  loss handling only. Pending recovery's three arms even simplify: the
  "catch up running loser tests against intermediate winners" clause dies
  (re-open IS catch-up; one race at tip).
- **Group commit** — one commit path with batching, real consumer
  (resident-mode packing); its drain-is-one-transaction law and one-by-one
  fallback stand. (Its knobs are 40-writer-repairs.md's business.)
- **`commit` vs `commit_split`** — essential, not accidental: splitness is
  chosen at the call site so "I thought this was atomic" stays
  unrepresentable.
- **The serial-verdict promise** (00 law 4's heart): a loser receives
  exactly the verdict a serial execution of the submitted transaction
  would have produced. One path makes this trivially true — the verdict
  IS a serial execution, performed.

## Contention causes, re-sourced

The hot-key payload no longer comes from a shared intersection key — it
comes from the re-judgment's own rejection: `Violations` names the
statement and carries the offending facts' raw values, engine-produced and
richer than the hash-derived determinants the old path reconstructed. The
cause sum survives unchanged in shape: `HotKey{statement, determinants}`
built from the terminal re-judged rejection; `SlotRace{tip}` when the
terminal losses were accepted-but-outraced. This also deletes the TS bug
where a pure-capacity contention reported `statement:-1, determinants:[]`
(ts-log/src/writer.ts:398) — the payload's one source is now the engine's
violation, which always names itself.

## The reopen trigger (recorded once, here)

If a real multi-writer deployment on a network store ever measures loss
resolution — discard, re-open, re-judge — as the dominant term in commit
latency under contention, the quantitative algebra reopens from this
deletion record: the theory lived at proposals/15-conflict-algebra.md and
lean/Bumbledb/Txn/Footprint.lean as of the pre-pass history, with L7's
acceptance form and its countermodels proven. Reopening is a design
campaign, not a revert — the wire, the emission rules, and the arithmetic
would be rebuilt against the then-measured workload. Until that
measurement exists, the algebra is git history.
