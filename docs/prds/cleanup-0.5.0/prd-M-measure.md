# PRD-M — The Measure phase: the twins ruled, the ephemeral re-earned

Wave 3 · Repo: bumbledb · depends on: U1 (the re-earn measures the lazy
NOSYNC-only ephemeral), U2/U6 landed (a settled tree) · **idle machine only;
owner go** · executes rulings 6, 7, 8 and U1's pending-measurement marks

The Measure phase alone owns timing (house law). Before any run: check
`ps`/`uptime` for foreign heavy processes; the machine is the M2 Max; runs go
through `scripts/measure.sh` (the mkdir lock); every banked number commits
its `report.json` AND `report.md` (the healed pin doctrine — the numbers
derive from what is committed).

## Part 1 — the measure-or-merge twins (rulings 6–8)

Protocol, identical for all three: build an isolated A/B (the fast path
ON vs routed through the generic machinery), measure on the idle machine,
then EXACTLY ONE of:

> **Amendment (recorded deviation, 2026-07-19):** the A/B scaffolding was
> prepared EARLY by the U2 commit (`b0ddb330`), against prd-U2's own
> scope wall — `Executor::disable_leaf_elision` (`cfg(test)` only, no
> runtime mode), `api/prepared/tests/measure_twins.rs`, and the ignored
> determinant twin in `storage/keys.rs`. The fast paths are unmodified;
> M still owns the measurement and the verdict, and every switch dies
> with the verdict as this protocol states. The deviation is also
> recorded in prd-U2's not-in-scope list.

- **Law**: a real win → record the number and the reverses-if clause at the
  site (the scan-pushdown sibling is the template) and in
  `40-execution.md` where the family's numbers live; the branch is
  KEEP-AS-LAW thereafter.
- **Merge**: no real win → delete the fast path, route through the generic
  machinery, correctness re-refereed by the differential oracle; re-run U5's
  checklist for the merged site (semantics-identical is claimed, so prove
  the conformance battery green).

"Real win" is judged against the house's existing margins in
`40-execution.md`'s decision records (the crucible ADOPT precedent, ~9%, is
the scale of the smallest banked win) — state the threshold used BEFORE
measuring, in the run's report.md. No number, no verdict: if the machine
never goes idle, the marks stay pending and the branches stay untouched.

1. **The leaf-elision complex** (ruling 6): `exec/run/leaf_precompute.rs`
   single-subatom condition + `run.rs` `leaf_single` buffer +
   `run_leaf_pinned` (`exec/run/leaf.rs`) — one measurement covers the
   complex; the A/B toggles the `single` classification off (test-scoped
   switch, the -off idiom; the switch itself dies with the verdict, never
   ships).
2. **The all-words finalize fast path** (ruling 7):
   `api/prepared/finalize.rs` `fill_word_answers` vs
   `fill_resolved_answers`, both sinks. The falsifier
   (`tests/fixpoint_finalize_hunt.rs`) already guards equivalence — the
   measurement is the only missing fact.
3. **The permuted-identity determinant** (ruling 8): `storage/keys.rs`
   `determinant_image` vs `permuted_determinant_image` with the identity
   permutation — measure on the hot commit path (writebench), since the
   trade is a per-fact indirection there.

## Part 2 — the ephemeral re-earn (U1's debt)

U1 changed what the ephemeral kind IS (32 GiB lazy map, NOSYNC-only, no
WRITEMAP): every banked ephemeral number is stale by construction. Re-earn:

4. **The ephemeral bench twins**: re-run the `--ephemeral` scenarios
   (`bumbledb-bench`, the `ephemeral_twin` load path) and re-true the
   R4-family numbers wherever the docs cite them.
5. **The device-tax number**: the 1.0–1.1× ramdisk figure
   (`50-storage.md` § the ephemeral kind) was measured under WRITEMAP through
   the R6 lane — re-measure through the post-U1 ramdisk lane (whose sizing
   U1 re-derived) or retire the figure with a retraction if the lane no
   longer exists in comparable form.
6. **The verdict sentence**: `50-storage.md`'s ephemeral section states WHY
   the kind exists; with WRITEMAP gone the recorded rationale must be
   re-argued from the new numbers. If NOSYNC-only shows no material win over
   durable, that is a FINDING for the owner (the kind's identity machinery is
   law regardless — the finding is about the marketing sentence, not the
   kind), never a silent doc edit.

## Passing criteria

- Each of items 1–3: a committed run dir (report.json + report.md), a
  pre-stated threshold, and exactly one verdict executed (law recorded at
  site + docs, or merge landed with oracle + conformance green). No branch
  left in the "measured but unruled" state.
- Items 4–6: every doc site U1 marked pending now carries a post-U1
  number citing its run dir, or a recorded retraction. The mark as
  actually planted is spelled `PENDING-RE-EARN` (50-storage.md,
  00-product.md, 70-api.md, architecture/README.md, README.md,
  ramdisk_phase_r.rs, TODO.md), so the close-out check is: `grep -rn
  "PENDING-RE-EARN" docs/architecture/ README.md crates/ TODO.md` is
  empty at phase close (the path set deliberately excludes this packet's
  own prose, which names the mark in order to grep for it).
- No test/margin weakened to make a number land; the alloc gate and
  check.sh green after any merge.
- All numbers measured on the idle M2 Max through measure.sh; the landing
  bar governs every claim.
