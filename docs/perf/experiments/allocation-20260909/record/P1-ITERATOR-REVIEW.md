# P1 iterator refinement — gated, not performance-accepted

Latest disposition: `P1-VARIANTS-REVIEW.md`. The complete ABC/CBA comparison
finished at 06:13:03 UTC. Both forms are deferred, not accepted. The
experimental production probe hunk was restored to baseline; its differential
regression test remains. No full trace was run. P2 is now a separate experiment.

All processes for the original broad comparison, focused ABBA, route observer,
iterator gate build and code-generation inspection are terminal. No benchmark,
build or profiler remains running. No full CPU trace was started.

## Why this is a separate candidate

The first P1 form's heavy-join gains repeat, but the ordinary control signals
were not cleared by focused ABBA. `P1-REVIEW.md` preserves the results and the
important actual-route proof: displaced probe never enters the changed carried
branch. Do not attribute its slowdown to repeated carried work, or claim this
new source form fixed its timing merely because tests pass.

The original indexed nested loop generated an additional survivor bounds check
and conditional-select bookkeeping. The new form consumes an enumerated slice
iterator across cursor runs. It still prepares one borrowed Probe per adjacent
equal tagged cursor, preserves whole-batch prefetch and successful/refused
prefix order, and adds no buffer or production metadata. It may recompute the
first cursor address at a run boundary; inspect and measure rather than assuming
that an iterator is automatically faster.

## Gates and frozen evidence

`p1-iterator-1/STATE.json` is
`GATES-COMPLETE-CODEGEN-AND-PERFORMANCE-REVIEW-REQUIRED`; session 64148 exited 0
at 05:54:13 UTC. Passed: formatting; 1,322 library tests (18 ignored);
287 allocation-enabled executor tests (11 ignored); all-target strict clippy;
ordinary release build. The saved-corpus test-only absolute-path hook is gone.

Frozen executable: `p1-iterator-1/bumbledb-bench`.
SHA256: `7dcf7fcf9b722757fb638236d68082eb209efcaa2648d9749eb199142a6a2331`.
Source fingerprint: `34b5672db672dd0f3da07212971286a8548f87f6245f7849632b915817318fbb`.

The original candidate remains frozen at
`p1-evaluation-2/candidate-bumbledb-bench`, SHA256
`ac4f45aba133369710b2d5a1ac458503fe1d60c7ba5e3f1f12236fff609fa622`.
Its 2,879-case oracle receipts and ordinary measurements do **not** certify the
new iterator executable. Verify the new binary in its own corpus before reads;
do not share or rewrite binary-bound stamps. Scenario commands independently
gate their selected queries.

The old `p1-resume.py check` intentionally requires the original source
fingerprint and now fails on this legitimate source change. It passed after
the completed ABBA and before editing. Do not "repair" old receipts to match
the new source. Use the new saved patch/fingerprint for this candidate.

## Code-generation result, not a speedup claim

`p1-codegen-iterator-1/` contains the frozen assembly and diffs. Width-one
containing function instruction counts:

| Mode | published baseline | indexed P1 | iterator P1 |
|---|---:|---:|---:|
| presence | 342 | 364 | 359 |
| children | 328 | 350 | 352 |

Per-key callbacks remain 244/265 instructions for presence/children; the normal
lookup instructions and local branches match baseline. The only normalized
callback differences are error-string addresses. The child-mode frame remains
320 bytes versus baseline 304. Root-loop shape remains eight instructions per
key versus baseline nine. The indexed survivor bounds check/csel is absent,
but iterator state introduces other moves/loads: this is **not** an unambiguous
machine-code-size or latency win. It does not establish a fix for root timing.

## Next bounded step

Choose a focused ordinary discriminator between the frozen indexed and iterator
forms on the primary rings/join workloads, with baseline controls and the
correct oracle gates. `scenarios --only` selects **scenario names**, not query
names (e.g. `rings,joins`); keep the registered rotating parameter stream.
Do not launch another identical full-roster timing loop just to hope away the
noisy host or accept solely from fewer static instructions. If this form fails,
preserve it and refine/defer P1; do not ship an unaccepted result.

After P1 disposition, the unimplemented P2/P3 and M1/M2 leads remain open in
the saved-trace ledger. Their evidence is not exhausted. No release, tag,
publication, commit or push occurred in this continuation.
