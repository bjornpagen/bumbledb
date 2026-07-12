# PRD 06 — The elision sub-measurement isolated

**Depends on:** baseline (Phase A recommended first but not required).
**Modules:** `crates/bumbledb/src/api/prepared/either_sink.rs` (mechanism
name: the `force_disjoint_off` toggle), `api/prepared/build.rs` (sink
construction), `api/prepared/introspect.rs`,
`crates/bumbledb-bench/src/calendar/families.rs` (`rsvp_union` /
`rsvp_union_off`), `crates/bumbledb-bench/src/driver/read_family.rs`
(report columns).
**Authority:** `docs/architecture/60-validation.md` (sub-measurement
protocol); the refutation policy (README measurement discipline: the
record keeps the numbers AND the failure mechanism); the 2026-07-12
anomaly: proof-on 1,393 µs vs forced-off 939 µs (−32%), identical row
counts, **different est/actual digests (3.00 vs 1.00)** — evidence the
two variants may not share a plan, i.e. the sub-measurement may not be
measuring the seen-set.
**Representation move:** a sub-measurement that cannot prove its two arms
differ in exactly one mechanism is not a measurement. This PRD makes the
isolation a *typed property with a unit test*, so the number the human
then produces is adjudicable.

## Context (decided shape)

Two hypotheses the isolated run must separate:
1. **The seen-set is a work-skipping device, not just dedup**: the sink's
   duplicate detection triggers the D2 origin cancellation; eliding it
   removes pruning, and the executor enumerates bindings that only
   produce duplicates. If true, the elision's premise ("dedup is pure
   cost") is refuted and PRD 07 reverts it.
2. **The two prepares diverge upstream of the sink** (plan shape or
   estimates), so −32% measures two different plans. If true, the
   sub-measurement is repaired here and the number re-earned before any
   ruling.

The decided isolation contract: `force_disjoint_off` must produce a
prepared query whose EVERY plan-derived artifact is byte-identical to the
proof-on prepare — same `ValidatedPlan`s, same estimates, same view
memos' shapes, same executor scratch shape — differing ONLY in the sink's
seen-set configuration (spanning seen-set present vs elided). If the
current toggle achieves this, the PRD proves it; if it does not (e.g. the
witness feeds anything upstream), the PRD moves the toggle to the one
legal place: sink construction, after planning is complete.

## Technical direction

1. Read the toggle's current wiring: where `force_disjoint_off` enters
   (bench harness → prepare path) and everything downstream that reads
   `disjoint_rules`/`union_elided` before the sink is built. Enumerate
   in a code comment at the toggle: the complete list of decisions the
   witness influences (expected: sink dedup only; the audit's
   digest-difference says verify, not assume).
2. If anything besides the sink reads it during prepare: restructure so
   the toggle applies at sink construction alone — prepare runs
   identically, then the sink is built with the seen-set forced on. No
   transitional flag: the old wiring is deleted.
3. `[test]` in `api/prepared/tests/` (or the bench differential tests if
   the toggle is bench-side): prepare the same multi-rule disjoint-arm
   fixture query twice, toggle on/off; assert the two `PreparedQuery`s'
   plans are equal (`ValidatedPlan: PartialEq` exists; compare per rule),
   assert estimates equal, assert `union_elided` differs, assert the
   introspection stats agree on everything except the elision flag and
   (post-execution) the absorbed counter.
4. Make the anomaly diagnosable from the report: the family report rows
   for `rsvp_union`/`rsvp_union_off` gain the executor's
   emitted/absorbed counters (already counted per rule — surface them),
   so the human's next bench run shows directly whether the seen-set
   absorbs within-rule duplicates (hypothesis 1's smoking gun: absorbed
   > 0 with the proof off) and whether cancellation fires.
5. Do NOT change the elision itself, the witness computation, or any
   sink semantics — this PRD is measurement plumbing plus its proof.

## Passing criteria

- `[test]` The plan-identity test of direction 3, green.
- `[shape]` The toggle is consumed at exactly one site (sink
  construction); `grep -n "force_disjoint_off" crates` shows the bench
  entry, the one consumer, and tests — nothing else.
- `[shape]` The report for the two families carries emitted/absorbed
  columns (assert via the report-render unit tests / goldens in the
  bench crate — update them).
- `[shape]` A comment at the toggle enumerates what the witness gates,
  with the test cited.
- `[gate]` Workspace gates green at campaign close.

**Handoff (measurement register):** after this PRD lands, one locked bench
run of the calendar family adjudicates PRD 07's branch. For this complete
unattended campaign the owner delegated the run and its mechanical ruling
to the executor: loss selects R; win selects E.

## Doc amendments (rule 5)

`60-validation.md` sub-measurement paragraph: the isolation contract is
now typed and tested (one sentence, citing the test by name).
