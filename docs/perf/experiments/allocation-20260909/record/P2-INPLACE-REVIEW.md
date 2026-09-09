# P2 in-place routing — unaccepted refinement

Latest: gate 5 and focused attempts 4/5 are complete. NO temporary hook is
mounted. P2-INPLACE-MECHANISM-REVIEW.md contains the matched-dispatch result:
cache movement costs only a few percent; production dispatch is larger in
several narrow singleton cases. Static gate-5 code inspection confirms a
112-byte rebuild frame is paid even on a cache hit. The next isolated gate-6
refinement splits the exact hot check from the non-inlined rebuild, with a
same-address changed-layout regression. Gate 6 now passes 1,329 library tests,
294 allocation-enabled executor tests, lint/build and 2,879 oracle cases.
Static inspection confirms hits skip the separate 112-byte rebuild frame.
P2-SPLIT-REVIEW.md is the latest continuation state. It is not performance-accepted.

Comparison 3 now completes (session 48189, exit 0, 07:25:54 UTC). The split
does not establish an o4 gain versus gate 5: +0.63% p50 / -0.94% mean centers,
mixed round directions. o3 is favorable, but scan and tail controls remain
unresolved amid large same-binary variation. All raw reports and three clock
flags remain; no identical broad rerun is planned. Gate 6 stays unaccepted.
All sessions are terminal; source fingerprint was revalidated. A read-only
saved-temporal-corpus structure audit is now complete in p3-saved-structure-1/;
see OVERLAP-LEAD.md. No second production change has been bundled into P2.

The completed ordinary ABC/CBA is reviewed in P2-THREE-VARIANTS-REVIEW.md.
Both prior forms remain unaccepted. All gate-3/gate-4 binaries, patches,
independent corpora, wide tests and ordinary reports are preserved.

## Current production change relative to gate 4

fold_row's reader takes (slot, borrowed route slice), rather than capturing
the sink's route Vec outside the mutable method. It receives that slice only
at each actual group/Fold/Float/Pack read. Scalar and staged readers ignore it.
fold_batch_rows therefore leaves cached_leaf_words in place; no per-batch
mem::take/restore. The group-key read's borrow ends before probe_group; later
numeric reads borrow the route field disjointly from the accumulator fields.
No unsafe escape, new persistent owner, width special case, threshold, storage
layout or SDK change. Ordinary dedup's taken staging Vec and exact-key rules
are unchanged. Constant-group and suffix-scan bulk reductions are unchanged
from gate 4. This is a proposed ownership/code-generation improvement, not a
proven explanation of the entire prior ~11% o4 C/B gap.

A permanent regression was added to borrowed_rows.rs: checked group-count
overflow on a different full binding, no partial publication, exact route
pointer/capacity/contents retained, reset, changed key-word layout, release,
then zero-key all-outer refill with independent expected Sum/Count output.
The unused semantic-Elided binding buffer retains zero capacity throughout.
Its original draft is preserved as p2_route_recovery.rs (not a mounted module).

## Gate 5 complete

Session 98771 completed p2-gate.py 5, exit 0 at 06:59:11 UTC. Formatting,
strict all-target clippy, 1,328 library tests (18 ignored), 293 allocation-
enabled executor tests (11 ignored), release build and the frozen executable's
independent 2,879-case oracle passed. Private corpus p2-gates-5/data; stamp:
9c2ec17363f9206da54e7c412dc4941c27bd21fc609e9c4ad3d39b93bcfb192f.
Frozen binary p2-gates-5/bumbledb-bench, SHA256:
84092b1b5d1ae97b134c4ced3f7fb4d7f3e7aeb57c1a3cb4935084ef03b646e0.
Source fingerprint:
1ced1b6ce8fbd415371de00be3f5a4d4899e50feffb1968a7a28a1c6014b0e78.
The complete source fingerprint was revalidated before mounting the next test.

## Focused discriminator attempt 4 (complete; historical setup below)

p2_wide.rs and p2-wide.py now compare staged / taken-cache / in-place
readers in ABC/CBA order at nine key/group widths and chunks of 1 and 128.
Each complete logical execution resets once and processes all 128 unique
rows exactly once, so singleton chunking does not license duplicate replay.
All three arms share the new arithmetic helper; controls intentionally omit
emit_batch's known fixture dispatch. Preallocated staged storage is separate;
no per-row searches/allocations are charged to it. This is a conservative
mechanism comparison, not the frozen old binaries' complete code generation.
It retains 108 raw sample blocks, all boundary-clock flags and actual scratch
capacities, with independent expected outputs for every case.

Session 50142 completed p2-wide.py 5 4 at 07:02:25 UTC, exit 0. The temporary
test-only fold_row.rs hook was removed, and both the gated source fingerprint
and the original fold_row.rs SHA256 revalidated:
7d0ce6cd000faa905a62ffbcaeb744f569a84b3182d7a11137cf8d50d203752e.

No full trace, release, tag, npm publication, commit or push. Remaining saved
leads are still open; the latest P3 audit adds reversed/equal query-constraint
coverage and unused small-group tree-node ownership as separate hypotheses.
