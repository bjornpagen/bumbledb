# Allocation research checkpoint — September 9, 2026

Research stopped at the user's request. These are preserved experiments,
not a combined implementation or a performance-accepted release. The user
subsequently requested merging the current P2 branch and this archive into
main; the other experimental alternatives remain on their separate branches.
Version 1.1.0 is unchanged and no release is being made. Do not infer
authorization to resume work from historical plans and next steps in `record/`.

## Source snapshots

Each snapshot is independently based on published commit
`b02a641e087364ec09c161a97c67c87cf626e6b2`. Q2 includes Q1's changes; Q3
includes Q2's changes. Do not merge all these branches together blindly.

| Experiment | Branch | Source commit |
|---|---|---|
| P2: borrowed aggregate inputs and regression coverage | `codex/autoresearch-allocations` | `b394b0cf` |
| E1: empty-input construction | `codex/autoresearch-e1` | `d50ad8b6` |
| M1: construction allocation | `codex/autoresearch-m1` | `36799f5d` |
| P3: interval overlap representation | `codex/autoresearch-p3` | `d15d0b8e` |
| G1: construction-tail growth | `codex/autoresearch-g1` | `ce64e880` |
| G2: staged growth | `codex/autoresearch-g2` | `0728e15c` |
| Q1: demand-grown query maps | `codex/autoresearch-q1` | `7c2de89a` |
| Q2: dense WordMap storage | `codex/autoresearch-q2` | `459f45b7` |
| Q3: single-source dense publication | `codex/autoresearch-q3` | `7720ebfc` |

## Verification and limitations

Before committing, the existing Q3 fingerprint verifier successfully
validated Q3, Q2, Q1, and all six earlier identities against the saved
records. All nine source diffs passed whitespace checks. The primary
checkout also passed a fresh workspace formatting check. No new build,
benchmark, timing panel, or trace was run for this checkpoint.

P2's `p2-gates-6/STATE.json` records passing formatting, strict lint,
ordinary library tests, allocation-enabled execution tests, optimized build,
and independent query verification. Its fingerprint is
`3bc43f3f04a09fa8983abeaf183eb54152fc8159a4a5b26fd73cbceb9f26958c`.

Q3's `q3-gates-2/STATE.json` records passing map/sink tests, strict lint,
1,329 ordinary and 1,342 allocation-enabled library tests, an optimized
build, and 2,879 independent oracle cases. Its fingerprint is
`6bc03e29d2e426a50995b5945efe2d1761966d816601f06a046edb183d39ff3b`.
The earlier failed gate is preserved as well. These are local results;
they do not imply a fresh cross-platform CI pass.

Q1/Q2 show substantial reductions in some retained allocations, but also
warm-query regressions. Q3 removes redundant map bookkeeping; it has not
had a matched allocation/timing comparison. Static code-size reductions
are not evidence of runtime speedups. None of these results establishes
RSS, Raspberry Pi suitability, or a universal performance improvement.

## Evidence archive

`record/` preserves the original reviews, diagnostic drivers, phase-state
records, source patches, and available phase source snapshots verbatim.
Read `Q3-DENSE-PUBLICATION.md`, `Q2-ORDINARY-TIMING-REVIEW.md`, and the
individual experiment reviews for acceptance limitations and losing controls.

The complete local evidence remains under
`bench-out/autoresearch-20260908.pMXtzv/`. Approximately 43 GB of build
products, databases, raw traces, timings, and other outputs were not added
to Git. This compact archive is not a self-contained reproduction bundle.

Drivers are historical, machine-specific artifacts, not supported scripts
to execute from this archive. Their original fingerprint checks expect
uncommitted diffs against the published baseline. Snapshot commits change
HEAD, so those checks must not be run unchanged against the now-committed
worktrees. Reconstruct the relevant baseline-plus-diff in a disposable
checkout if resuming verification; never reset these saved branches to
make an old assertion pass. Another full trace remains prohibited until
the saved evidence has been exhausted.
