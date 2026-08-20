# 39 — New lanes: the heap arm gets measured like the store arm

- **Status:** OPEN (final pass).
- **Severity:** performance coverage — the heap arm shipped without its
  ladder.

## Principle

The store arm's perf truths are pinned by lanes; the heap arm has focused
tests but no measured characteristics. "Aggressively cleaning up the
performance characteristics" starts with measuring the surface that never
had numbers.

## The lanes

1. **Frozen vs LMDB point reads**: warm `get`/`contains`/`scan` on the same
   canonical corpus, heap arm vs store arm — the binary-search-vs-B-tree
   truth, recorded.
2. **Admission throughput**: facts/second and the five phase quantities
   (`A`, `I`, `R`, `F`, `J`) across corpus prefixes — the proposal's
   telemetry, promoted into a lane.
3. **The Primer scaling gate** (proposal's Allocation and performance
   gates): the full normalization corpus from the sibling `primer-spec`
   repo through load → complete admit → keyed reads → representative joins
   → `fromInstance` publish, across at least four prefixes, with no
   unexplained superlinear growth. This is the release gate the proposal
   already names; it has never been run.
4. The bare-metal ramdisk row rides the release checklist (closed issue
   18's suggested one-liner) beside this lane.

## Acceptance

- The three lanes exist, run under the bench driver, and their first pins
  are recorded in the bench docs.
- The Primer gate's verdict (green or the named superlinear term) is
  recorded; if the corpus cannot be obtained, the gate is marked blocked
  with the ask — never silently skipped.
