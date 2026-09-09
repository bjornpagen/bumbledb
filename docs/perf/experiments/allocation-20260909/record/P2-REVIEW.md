# P2 borrowed aggregate inputs — candidate, not performance-accepted

This records the initial linear-getter candidate. P2-COMPACT-REVIEW.md is the
latest source/gate/continuation state. Both original wide attempts are terminal;
their result disproved wide-row acceptance and motivated constant-time routing.

## Selection and scope

Standalone on published b02a641e, with the production P1 probe hunk removed.
P1's tests and all B/C artifacts remain; see P1-VARIANTS-REVIEW.md. This is
not a declaration that P1 or the saved traces have been exhausted.

The saved o4 path spends material CPU staging outer/leaf binding words before
loading only two group words for Count. The current census rules out per-row
heap allocation here. The candidate borrows a word getter for group/Fold/Float/
Pack inputs. A checked semantic or physical distinctness witness skips binding
staging; ordinary dedup still consumes the complete binding. Non-distinct
batches keep the outer copy once per batch and read folded words directly
from the existing staged slice, avoiding repeated source searches there.

Only three production files change: aggregate/fold_batch.rs, fold_row.rs,
and sink.rs. The exact seen-key contract, constant-group batch/scan reductions,
shape cache, group representation, allocation owners/capacities, planner,
executor and storage are unchanged. No new buffer, routing metadata, quota,
fallback, or unsafe access. No warm-allocation reduction or RSS claim.

Borrow audit: the getter borrows only the executor's LeafBatch/Bindings or a
local taken staging Vec, never sink-owned group pools. Group spilling does not
clear the distinctness witness or access binding_scratch. Every normal/typed
error return from fold_row returns to the caller that restores staging; the
non-distinct batch has no intervening outer early return. Key-slot metadata
is unique under the validated occurrence partition, so source_of agrees with
the old assignment map. Repeated/scrambled survivor IDs remain supported.
Text-retention declarations and source-owner lifetimes are unchanged.

The larger exact-dedup-scratch consolidation is deliberately separate: the
initial two-word routing sketch can increase retained metadata. See the
explicit length accounting and limitations in P2-IMPLEMENTATION-SKETCH.md.

## Red/green proof and test obligations

`p2-regression-baseline-1` ended EXPECTED-STRUCTURAL-FAILURE. Against unchanged
aggregate production source, the new witnessed-row test fails because staging
was touched, while the union-key and typed-error recovery controls both pass.
The exact original test and output are preserved there.

The candidate passes the first full library/allocation runs. Gate attempts 1
and 2 preserve lint failures for the long test function (138 then 104 lines),
not hidden correctness failures. The fixture now has named setup/output/reuse
helpers, without suppressing the lint. Attempt 3 additionally includes direct
distinct Float and Pack tests; production source is unchanged from attempt 1.

Coverage includes semantic and physical witnesses, no witness after reset,
scalar and empty/singleton/multirow batches, sparse scrambled survivors,
dense/hashed groups, no staging under witnesses, warm zero-request/free
windows, exact integer expectations, union keys wider than the binding with
repeated spans and reaim, zero-word union keys, typed refusal and release.
Added Float and Pack cases check exact independent expected outputs under
changing key layouts and forced partition flushes. Existing wider group,
union/DNF, numeric, Pack, spill and cancellation tests remain in the gates.

## Gates and frozen binary

Gate attempt 3 completed at 06:21:32 UTC; session 82583 exited 0. Formatting,
strict all-target clippy, 1,327 library tests (18 ignored), 292 alloc-counter
executor tests (11 ignored), ordinary release build and all 2,879 independent
oracle cases passed. Its source hash is
66f441f1763f86d13bfe7174a2fbb12555fa57de310e6b1f65c36d0ad5c2fbc2.
Frozen binary: p2-gates-3/bumbledb-bench; SHA256
1c60107fd352ca220205e096cc3a605207657b35d8f9bd5cc37b97f787deef88.
Private verified corpus: p2-gates-3/data; stamp
afc2bac45ce4eaed23a951ac611fd5756c48202c4208ce296e74d3bf543fe10b.
Nothing is accepted merely because the gates passed.

## Initial continuation state (historical; superseded above)

Session 34694 completed the ordinary ABBA and exited 0 at 06:28:56 UTC.
P2-COMPARISON-REVIEW.md reviews every family and its remaining concerns.
Source fingerprint was independently rechecked before mounting the diagnostic.

Session 28966 now runs p2-wide.py 1 under the measurement lock. Its release
test build began at 06:31:25 UTC as PID 20901. A TEMPORARY test-only absolute
path hook in aggregate/fold_row.rs mounts p2_wide.rs. It must be removed after
the driver finishes; do not leave it in a commit. Before the hook, the file
SHA256 was 2dcb38826fa120559b85c9c9723431bd6b62b3abbac56a57d39a7e453d8b7d32
and the complete source fingerprint was the gated 66f441... value above.
Revalidate the live session/process; do not restart on an observation timeout
or edit tracked source during its frozen build/timing. The driver records the
mounted source, builds without running, freezes/hashes the exact test binary,
then runs only the wide discriminator. No profiler or full trace is invoked.

Wide-batch performance remains an explicit acceptance gate. The direct batch
getter calls source_of, a linear search over K key words, for W required group
and argument reads: O(K*W), whereas staging is O(K+W) plus amortized outer
copying. Narrow leaf wins cannot license a general improvement claim. A
store-free wide witnessed grouping/folding discriminator should quantify this
before acceptance; do not introduce routing metadata or magic thresholds to
hide it without measurement. A dense cached slot-to-source map could bound
lookup work, but its ownership, borrow scope, cold capacity and non-distinct
outer-copy costs must be accounted separately. This is an open lead, not an
accepted design or a request for a new trace.

No commit, push, release, tag, npm publication, or new full trace in this work.

## First ordinary pair — provisional, not a disposition

A0 read windows are all unflagged by the clock guard. The first candidate
scenario pass has a promising o4 signal, but j4 loses again. Do not accept
from this pair; B1/A1 and the candidate read controls are still required.
The large apparent o5 mean/tail improvement is partly A0's 1.478 ms pause.
No isolated point estimate establishes a performance regression or win.

| Query | A0 p50 ms | B0 p50 ms | B0/A0 p50 | B0/A0 mean |
|---|---:|---:|---:|---:|
| j1_filmography | 0.000459 | 0.000417 | 0.9085 | 0.9803 |
| j2_costars | 0.001084 | 0.001084 | 1.0000 | 0.9983 |
| j3_keyword_kind | 0.001791 | 0.001584 | 0.8844 | 0.8295 |
| j4_five_way | 0.719542 | 0.916208 | 1.2733 | 1.1040 |
| j5_country_rollup | 4.319791 | 4.058542 | 0.9395 | 0.9459 |
| j6_keyword_neighborhood | 0.028291 | 0.029250 | 1.0339 | 0.9865 |
| o1_revenue_by_region | 0.480000 | 0.482750 | 1.0057 | 1.0007 |
| o2_category_window | 0.362375 | 0.387208 | 1.0685 | 1.0060 |
| o3_promo_split | 0.329208 | 0.331375 | 1.0066 | 0.9913 |
| o4_segment_category | 24.362083 | 22.718667 | 0.9325 | 0.9397 |
| o5_store_extremes | 0.500375 | 0.485917 | 0.9711 | 0.8814 |
| o6_brand_drill | 0.002000 | 0.001917 | 0.9585 | 0.9610 |
