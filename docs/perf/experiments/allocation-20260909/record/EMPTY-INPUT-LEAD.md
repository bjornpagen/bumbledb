# Known-empty positive inputs: avoid constructing unrelated join tables

The hypothesis is now isolated in `e1-source`; see `E1-EMPTY-INPUT-REVIEW.md`
for discriminators, implementation, failed evidence and verification status. It
comes from the saved triangle corpus's exact ownership audit, not a new CPU
capture, synthetic RSS model or guess from total result row count.

## Evidence

M1-CONSTRUCTION-REVIEW.md and m1-saved-{baseline-1,duplicate-2}/ record exact
SQL results and all active/parked COLTs for four S/seed-1 triangle parameters.
Every baseline/candidate allocation and ownership row agrees. In the cold
empty range [500,500), occurrence 0 has an empty filtered view but occurrence
1 still forces all 100,000 source rows into 43,236 keys / 16,384 groups.
The full occurrence-1 published table is 2,401,168 bytes; its three arena
capacities total 4,423,680 bytes; its COLT pool capacities total 6,393,280 bytes.
The whole cold query requests 16,414,780 bytes in 95 events. These are different
scopes. Never call all of them removable, RSS or peak live database memory.

The saved timing panel includes this empty draw in a pooled four-parameter
distribution. It does not say how many nanoseconds this individual draw owns.
Use per-draw untimed ownership first and per-draw timing for any new acceptance;
do not infer from the panel's p50 or repeat the same pooled loop for a win.

## Source and smallest candidate

run_join binds all occurrence views, then selects their parameter prefixes,
then executes. Colt::select returns Some(root) for zero selection levels even
when the bound view is empty. That is a cursor contract, not a claim that rows
exist. The executor's node/cover ordering can therefore enumerate or force a
different occurrence before a later positive input eliminates all candidates.

Investigate a rule-local empty-positive-view early exit at the common bound/
selection seam. It should use already-known row absence and require no hashing,
forcing, new allocator policy, planner rewrite, or data-dependent mode switch.
Prefer a common check after binding over duplicating it in every fresh/memo/
dedup/derived bind arm for the initial experiment. Preserve the existing
selection contract; do not casually redefine Some(root) in Colt::select for
all consumers. Price any additional per-occurrence branch on nonempty paths.

## Required discriminators before implementation/acceptance

- Reproduce baseline: an empty positive filtered view plus a large independent
  positive input still constructs the latter. Assert logical SQL/naive output
  AND which COLTs/images were actually touched; result equality alone cannot
  detect the wasted work. Include an ordinary empty relation, not only equal
  parameter bounds, to avoid a workload-specific contradiction peephole.
- An empty negated occurrence does NOT empty the positive join. Discharged
  occurrences must keep their existing rules. Zero-arity positive atoms must
  preserve Boolean existence; atom-free and statically true rules still run.
- Keep this return local to one conjunctive rule: DNF alternatives, other
  prepared rules, derived/interior pipelines and recursive fixed points must
  continue. Empty Count/Sum and grouped-empty finalization must stay correct:
  the existing BumbleDB contract emits zero rows for both, not SQL's synthetic
  empty global aggregate group.
- Test cold empty, warm empty, empty-to-nonempty and nonempty-to-empty parameter
  rotations, parked/memoized hits, same-shape dedup, changed source epoch,
  post-write reuse, release_memory and failure/retry. A short exit must not
  make later unbound/stale occurrence state appear current on a future run.
- Poll/flush cancellation consistently before a successful early return; a
  cancelled operation must not be converted into successful empty output.
  Existing current/prior WorkContext and stale token guarantees remain.
- Snapshot requested bytes, published/retired lengths and retained capacity
  separately; keep shared images and caller output ownership out of COLT sums.
  Expect unchanged nonempty controls. No quotas, fallback architecture, SDK or
  persisted-layout changes. No new full trace.

Keep this experiment separate from M1's duplicate-boundary patch and P2/P3.
The saved evidence is not exhausted, and general empty-input elimination is
a more promising next discriminator than adding a large final-table copy with
no demonstrated retained-capacity or actual clone benefit on this corpus.
