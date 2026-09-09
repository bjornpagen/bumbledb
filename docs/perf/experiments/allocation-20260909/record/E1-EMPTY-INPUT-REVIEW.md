# E1: known-empty positive inputs

Isolated on published v1.1.0 in `e1-source`; P2/P3/M1 are untouched. This
experiment follows `EMPTY-INPUT-LEAD.md` and the existing exact saved-corpus
ownership audit. No new full trace, benchmark suite, release, commit or push.

## Discriminators and failed attempts

The original small triangle chooses an empty-first, terminal-scan plan. Its
passing no-force test was not a regression reproduction. Attempts 1/2 preserve
that negative evidence. Attempt 3 failed compilation (`Term::Lit` typo), and
attempt 4 failed validation (`NoCover` on a node with no new variable). Neither
is a database regression or a successful construction discriminator.

Attempt 5 seals a legal entry-first plan using the ordinary plan validator,
then constructs its matching executor/COLTs and calls the real `run_join`.
No prepared plan is substituted without rebuilding its dependent artifacts.
It reproduces unnecessary construction for both an empty filtered input and
an empty relation. The baseline's root capacities are:

- Filter-empty: `[Some(8), Some(1024), Some(1024)]`.
- Relation-empty: `[Some(512), Some(1024), Some(1024), None]`.

Both outputs are correctly empty; the failing assertion checks unnecessary
construction, not logical answer corruption. `e1-baseline-regression-5/`
records the expected raw test failure and confirmed discriminator status.

Focused run 1 (candidate) exposed two errors in the new tests. Focused run 2
(baseline, production edits removed) reproduces both: the planner subsumes a
restricted rule under its unrestricted duplicate, and BumbleDB emits **zero
rows** for empty global aggregates. The existing permanent
`global_aggregate_over_empty_input_yields_zero_rows` test specifies the latter.
Do not replace that semantics with SQL's synthetic Count/Sum group. The union
fixture now has a genuinely distinct live relation; empty-fold expectations
match the existing contract.

Focused run 3 is the corrected baseline suite: five pass, two intentional
failures. In addition to construction, cancellation injected on the final
epoch lookup of fully warmed bindings becomes `Ok(())` on a selection miss.
This is an internal join-seam gap; the outer public execution boundary already
has its own final checkpoint. No user-visible cancellation-corruption claim.

## Candidate and scope

`Colt::is_empty` checks the bound view's existing length; Unbound still panics
as an invariant error. At the common post-bind selection seam, a participating
positive empty occurrence returns from this rule before join execution. Empty
negation and discharged roles keep their meanings. `Colt::select` retains its
root-cursor contract. Both the new empty exit and the old selection-miss exit
flush pending work before succeeding.

All occurrence binding still precedes the check. This does not avoid earlier
image construction, filtering buffers or same-shape cloning in that binding
loop, or selections reached before a later empty occurrence. It introduces no
owner, representation mode, allocation policy, public
SDK change or persisted-layout change. Subsequent nonempty runs select all
required prefixes again; stale cursors from skipped execution are not used.

The seven tests cover the forced late-empty plan, natural empty-first controls,
parameter/epoch/release reuse, empty positive/negated and zero-arity relations,
live alternative rules, empty/nonempty aggregate reuse, and exact cancellation
at the bind/selection seam followed by healthy empty and nonempty execution.
Existing full tests additionally cover interiors, recursion, discharged/folded
roles, memoized current/prior work, and fallback execution.

## Checks and next evidence

`e1-gates-1/` completed at 09:17:18 UTC, session 96873, exit 0. All steps
and raw outcomes were reviewed: seven focused tests; format; strict
engine/benchmark all-target clippy; 1,328 ordinary library tests and 1,341
allocation-enabled library tests (18 ignored in each); ordinary release build;
and 2,879 independent query-oracle cases. Oracle stamp:
`b194ee7d9c4e5ec93c099c4d3292b4e319937ed34c20cbe936272e04a95171ce`.

Frozen ordinary source:
`e44e04132d30992ab094d61c93d620207175e1390006046e53733c16fb52cd55`.
Frozen ordinary executable:
`465f5db5471fb5c5e997153c9f264a61fbeb45ed71880515ed1dd7d62cc22c47`.

## Actual saved-corpus allocation and ownership

`e1-saved-1/` completed at 09:18:13 UTC, session 54064, exit 0. The driver
verifies the byte-identical saved database, identical external observer/test/
schema and exact SQLite windows from `m1-saved-baseline-1/`. Original inputs
remain unchanged. `e1-saved-review-1/review.log` reviews all 20 execution windows
and 78 active/parked owner pairs, including every one of 11 changed snapshots.
The reviewer finished successfully at 09:18:31 UTC.

Fresh cold empty `[500,500)` query:

| Measurement | Published baseline | E1 |
| --- | ---: | ---: |
| Allocation requests | 95 | 18 |
| Requested bytes | 16,414,780 | 5,300,324 |
| Free requests | 4 | 4 |
| Freed bytes | 4,718,502 | 182 |
| Live published table bytes | 2,401,304 | 0 |
| Three map-arena lengths, bytes | 4,229,756 | 0 |
| Three map-arena capacities, bytes | 4,423,816 | 0 |
| All COLT-pool retained bytes | 6,795,880 | 400,048 |
| Retired map-arena content, bytes | 1,828,452 | 0 |

This removes 11,114,456 requested bytes (67.71%) and 6,395,832 retained
COLT-pool bytes (94.11%) **in this cold empty query**, not across the database.
Occurrence 1 keeps its 100,000-row bound image but no longer builds the
43,236-key root table: its pool retention falls from 6,393,280 to 16 bytes.
Occurrence 0 retains the existing 400,000-byte filter-position capacity plus
16 root bytes. These are not RSS, peak live ownership, allocator usable size,
the mmap extent, shared image bytes or Pi qualification.

Every nonempty execution's exact allocation/free/byte/clone tuple is unchanged.
The eight warm/repeat and four second-rotation executions still make zero
requests and free zero bytes. All clones remain zero. All logical answers match
SQLite: five accounts for each nonempty draw, zero for the empty draw.

The first rotating empty draw saves only four requests / 2,568 bytes, because
prior nonempty draws already legitimately built the large root. Retained bytes
then differ by just those 2,568 bytes, including when the empty occurrence is
parked during later nonempty runs. Do not generalize the fresh-empty 94.11%
reduction to that already-warmed mixed workload.

## Handoff and next action

All temporary observer hooks were removed with a targeted patch. The exact
ordinary source fingerprint above matches gate 1 again; format and diff checks
pass. P2/P3/M1 fingerprints revalidated exactly against the prior checkpoint.
All sessions are terminal. No commits, pushes, release actions, full benchmark
or new full CPU trace occurred.

E1 now has proved saved-workload allocation savings and complete correctness
gates. The subsequent ordinary per-draw ABBA comparison completed at 09:28:21
UTC (session 63397); its raw-sample review completed at 09:28:39.
`E1-PER-DRAW-TIMING.md` is the timing authority: cold empty medians fall from
8.56–9.10 ms to 4.18–4.20 ms, while warmed empty executions fall from about
1.49 ms to below 1 microsecond. Nonempty triangle centers are close; adverse
point controls and eight flagged clock brackets remain in the evidence.
This is scoped empty-query speed evidence, **not universal speed acceptance**.
No identical panel rerun is justified just to improve those controls.
"Memoized" means warmed query structures, not an answer-row cache; see the
explicit terminology correction in the timing document.
Source images and filter buffers remain allocated; later empty occurrences can
also follow earlier selections. These are remaining scope questions, not a
license for a larger rewrite or a claim that saved traces are exhausted.
