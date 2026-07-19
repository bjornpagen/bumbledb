# PRD-I2 — The delete-bearing cold-read bench lane

Repo: bumbledb (`crates/bumbledb-bench/`) · depends on: — (lands before Wave M
so I1's measurement has both witnesses) · Report-class, never gated · gates:
`scripts/check.sh` (bench crate builds + its tests) — no timed run required to
land the lane itself.

## Objective

Make the delete-induced rebuild cost **measurable at all**. Today it is
invisible by construction: every timed write family is insert-only (registry
`bumbledb-bench/src/families/write.rs:7-85`; the only `tx.delete` in the whole
bench crate is the differential oracle; `sweep.rs:368` even annotates "these
commits delete nothing"), and the one read-after-write lane —
`cold_containment_walk` (`writebench.rs:174-208`, harness `harness/cold.rs:6-65`)
— pays an **insert** touch commit before its timed read. No bench measures a
rebuild after a delete-bearing commit. So the compact-vs-mask fork of the
delete design has no number, I1's boundary has no negative witness, and the
gated mask PRD's reopen trigger (§00-README ruling 2) has nothing to fire on.

## Work

1. **The lane.** A sibling family beside `cold_containment_walk` — working name
   `cold_containment_walk_delete` — identical in every respect except the touch
   commit: instead of one Org insert, it performs the cookbook's canonical
   revision idiom, **delete(old) + insert(new)** of one existing fact in a
   relation the timed walk reads (recipe-20 shape; primer's attemptText-swap
   shape). Same harness discipline as `harness/cold.rs`: every sample pays the
   touch commit (generation bump) before the timed read, so the timed number
   carries the delete-induced rebuild. The revised fact must genuinely change
   bytes (a same-bytes delete+insert cancels in the delta,
   `storage/delta/insert.rs:9-14`, and would commit nothing).
2. **Registered Report-class** (`Kind::Report`) — recorded, never gated,
   exactly like the write families and the existing cold lane. No ALL-WIN
   membership, no README claim; the row exists to be read.
3. **The pins** (tests beside the existing cold ≥ warm pin at
   `writebench.rs:301+`):
   - delete-cold ≥ warm (same shape as today's pin);
   - the touch commit's shape is asserted delete-bearing (the delta genuinely
     carries one Delete disposition for the walked relation) — this is the
     lane's reason to exist; a drift to insert-only would silently measure the
     wrong fork.
4. **The I1 interaction, stated in the family's doc comment:** under I1 this
   lane must NOT improve (the walked relation is dirty every sample — the
   append arm never fires for it), while `cold_containment_walk` should
   collapse. The pair is the discriminator's end-to-end witness: append lane
   fast, delete lane unmoved. Wave M reads both rows; if the delete lane moves
   under I1's twin, the discriminator is wrong and the landing stops.
5. **SQLite twin**: the same delete+insert touch on the comparison side, so the
   row's ratio means something — mirroring how `cold.rs` twins the touch today.

## Recorded baseline (Wave M, 2026-07-19)

The lane's first honest numbers — the new lane's baseline, no A/B (there is no
"before" for a new lane). Conditions: Apple M2 Max, idle machine (verified),
release driver, three full `bench` invocations each under `scripts/measure.sh`
(fresh scratch corpus per run), scale S seed 1, durable stores, engine rev
5f6c746a, verify stamp `1f665e0c…` fresh (2862 oracle cases green, this
engine); COLD protocol (2 warmups, 16 samples); min-of-3 of the report p50s:

- `cold_containment_walk_delete` — ours p50s [3540.6, 3554.4, 3566.3] µs,
  **min 3540.6 µs** (that rep's proxy block clean, 3.26/3.26 GHz); SQLite twin
  p50s [59.2, 56.8, 66.3] µs, min 56.8 µs. The delete-induced rebuild costs
  ~3.5 ms per cold read at scale S — the compact-vs-mask fork's price tag,
  measurable at last. Report-class: recorded, not gated.
- `cold_containment_walk` (the append sibling, same runs, post-I1) — ours p50s
  [1416.5, 1356.4, 1357.6] µs, min 1356.4 µs; SQLite twin min 56.7 µs. The
  pair shows the discriminator end-to-end: append lane ~1.4 ms, delete lane
  ~3.5 ms — consistent with the controlled in-process A/B
  (`cold_lineage_twin`: walk OFF/ON 2.54×, delete lane 0.99).
- Clock proxy: rep 1 both blocks clean; rep 2's walk block and rep 3's both
  blocks stamped CONTAMINATED (post-fsync DVFS) — recorded per the report's
  own dirty-list; the min-of-3 delete figure comes from a clean block.

The I1-interaction contract held: the delete lane did NOT improve under I1's
twin (0.99), while the append lane collapsed — the landing proceeds.

## Explicitly out of scope

- No mask/tombstone implementation, no compaction policy — the fork stays
  gated; this lane is its price tag, not its build.
- No new gate, no geomean membership, no README sentence beyond the report row.
- No timed numbers asserted at landing: the lane's first honest numbers arrive
  in Wave M under `scripts/measure.sh` (idle machine, owner go), recorded with
  tier and machine conditions.

## Passing criteria

- The family registered, runs under the bench driver at every scale the cold
  lane supports, produces a report row.
- Both pins green (delete-cold ≥ warm; touch-commit shape asserted
  delete-bearing).
- Doc comment carries the I1-interaction contract (append lane moves, this lane
  does not).
- `scripts/check.sh` exit 0. No existing family's registration, gating, or
  recorded numbers touched.

## Size

**S.** One family + one harness variant + two pins + a doc comment. No split.
