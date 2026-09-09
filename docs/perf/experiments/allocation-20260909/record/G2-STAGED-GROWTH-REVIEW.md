# G2: stage occupied entries and grow unpublished tables at fixed bases

G2 is an isolated candidate from published b02a641e in `g2-source`.
P2/P3/M1/E1/G1 remain untouched and unaccepted. The source-supported lead is
`G2-STAGED-GROWTH-LEAD.md`; existing saved allocation/ownership evidence,
not a new CPU capture, motivates this experiment. Neither full-trace entry
point has been enabled. No full benchmark, commit, push, SDK/layout change,
release, tag or version bump occurred.

## Implementation and contract

Only ordinary `grow.rs` changes. One algorithm stages occupied key/packed-child
records into the existing scratch owner in dense order, reserves ctrl/bucket
growth at their original starts, and rewrites dense indices at the same
ordinals. It neither appends retired tables nor relocates whole tables.
Expanded bucket words are initialized in pieces of at most 64 KiB. Only old
control bytes are cleared; empty controls gate old bucket payloads. Controls
extend with zeroes, also cooperatively. All used scratch content is cleared
and its capacity restored on every Result path, including refusal.

All representable sizes are checked before any reservation. The map must own
all three arena tails. Both arena reservations finish before destructive
writes, and the new Map bucket count commits only after rehashing succeeds.
Failure after mutation requires the existing force_unforced rollback to
discard the whole unfinished construction. Earlier published maps precede the
three tails and are never rewritten. Successful reservations may retain extra
capacity on a later failure; this is not process-wide OOM recovery.

The old synthetic test that grew a copied *published* Map is replaced with
cancelled child construction preserving a readable root and a healthy retry.
The actual production caller is always unpublished construction. No alternate
growth implementation, owner, allocator, quota, fallback or duplicate-growth
policy change was added. force.rs is byte-identical to published source.

## Red/green and correctness

Baseline attempt 1 stopped on an extra blank line in the copied test fixture;
it is preserved. Baseline 2 completed 10:06:00 UTC (session 73399): the older
map/order/child/clone/reset control passes on unchanged production code, while
the intended retention assertion fails. For 513 distinct rows, baseline
ctrl/bucket/dense lengths are [3840, 7680, 1228], versus live lengths
[2048, 4096, 513]. This is a retention discriminator, not a timing result.

Focused attempt 1 failed compilation on test-hook module visibility. Attempt 2
passed all other checks but exposed a fixture assumption: after release/reset,
ordinary Vec::push gives the root-node vector capacity four instead of the
original vec![root]'s capacity one (48 additional payload bytes). The test now
accounts for that ordinary node capacity explicitly; scratch is unchanged.
Gate attempts 1 and 2 stopped on checked-conversion lint errors (one production
hash conversion, then five test conversions). All attempts remain preserved.

Gate 3 completed 10:15:52 UTC (session 75303), exit 0:

- Format and strict engine/benchmark all-target lint pass.
- 47 focused COLT tests pass; three intentionally ignored tests are not counted.
- 1,329 ordinary and 1,342 allocation-enabled library tests pass, with 18
  ignored in each invocation.
- Ordinary release build and 2,879 independent oracle cases pass.

The new tests cover zero/fixed/wide keys, empty/direct/multiple growth,
packed singleton/node children through the full u32 payload, dense order,
older-map resume tokens, promoted children, cloning, reset/rebind reuse,
explicit release, and checked overflow before allocation. Actual child forcing
is refused at all 35 growth checkpoints under both Cancelled and Allocation,
with cold and retained capacities: 140 failure/rollback/healthy-retry cases.
The Allocation injections test propagation, not actual allocator exhaustion;
overflow refusal is tested separately. A larger unpublished-map seam test
refuses after the first of two 64-KiB old-control clears and verifies partial
mutation, untouched published prefixes, restored scratch and safe discard.

Gate source fingerprint:
`c759483862ce7a957dcda1a39e4246b36fe47fd1e1eb5614dfa82bd725f70e04`.
Ordinary executable SHA-256:
`1d1ff51dee29fdc2e91519ecbaa0351cd13c66d2b34976a1ea335df2a9ba1184`.
Oracle stamp:
`337bfde2312fee2cf7b7b3c3eb75927097f34f8b48140511724ce1d2338d6af2`.

## Saved-corpus allocation audit — complete

`g2_saved.rs` and `g2_saved_observer.rs` derive from the frozen M1 test and
observer. The query, SQL draws, input database and execution windows are
unchanged. The only observation additions are scratch length/capacity/bytes
in every snapshot and an additional report line outside the allocation window.
All-pool retention already includes scratch; reporting it separately prevents
moving waste off the books. Baseline, G1 and G2 must use identical observers.

The first G2 observer build failed on module visibility and is preserved in
`g2-saved-g2-1`. Attempt 2 completed 10:17:20 UTC (session 83794), baseline
completed 10:18:05 (69070), and matched G1 completed 10:18:52 (80302).
These are counting-allocator diagnostics, not ordinary elapsed-time comparisons.

`g2-saved-review-1/` completed 10:19:30. It checks all 20 exact SQL windows
and all 78 active/parked owners in each of the three variants, including all
50 changed snapshots. New baseline and G1 allocation/arena/all-pool records
are byte-for-byte equivalent as parsed values to their original frozen audits:
adding scratch observation did not change an execution's allocations.
All map shapes, widths, counts and published-table bytes are unchanged.
All G2 retired arena contents disappear. Every other owner, after subtracting
the three arenas AND scratch, is exactly equal across all variants. All clones
are zero. No active or parked owner is omitted from the totals.

| Saved `[1,6)` cold window | Published | G1 | G2 |
| --- | ---: | ---: | ---: |
| Allocation requests | 166 | 162 | 165 |
| Requested bytes | 16,597,148 | 15,442,076 | 14,402,764 |
| Freed bytes | 4,806,726 | 4,507,718 | 4,161,590 |
| Three-arena capacity bytes | 4,497,408 | 3,641,344 | 2,527,232 |
| All COLT-pool retained bytes, including scratch | 6,876,432 | 6,020,368 | 5,327,184 |
| Four cached draws: all COLT-pool retained bytes | 8,249,384 | 7,319,592 | 6,629,544 |

Each nonempty cold draw saves one request, 2,194,384 requested bytes and
1,549,248 retained bytes versus published code. The first two draws retain
22.53% less; the third retains 22.52% less because an unrelated owner adds
4,096 bytes in all three variants. Requested saving minus the reduction in
freed bytes equals the retained saving: 2,194,384 - 645,136 = 1,549,248.
Four cached draws save 1,619,840 retained bytes (19.64%) versus published.

The saved large root matches the prior **four-owner** model exactly:

| Large root only | Published | G1 | G2 |
| --- | ---: | ---: | ---: |
| Three-arena capacity bytes | 4,423,680 | 3,604,480 | 2,490,368 |
| Scratch capacity bytes | 64 | 64 | 419,424 |
| Combined retained bytes | 4,423,744 | 3,604,544 | 2,909,792 |

Do not hide the larger scratch or describe this as a universal owner/request
win over G1. The small occurrence-0 index has identical arena capacities to
G1 (36,864 bytes), but scratch rises from 64 to 1,632 bytes. It therefore
retains 1,568 MORE bytes than G1. Each extra nonempty rotating draw requests
2,784 more bytes and two more allocations than G1, while still improving
on published code. Whole cold-query G2 requests are three above G1; whole
retention is 693,184 bytes below G1. With three cached nonempty views, the
retained advantage over G1 is 690,048 bytes. These tradeoffs are preserved.

All same-draw warm/repeat and second-rotation windows allocate and free zero
bytes. The fresh empty draw saves 2,123,440 requested and 1,513,952 retained
bytes versus published, but does NOT save a request and still builds the
unnecessary root: G2 does not include E1. Rotating empty-draw allocations are
identical across variants. Do not sum isolated E1/G2 savings.

These are requested layouts and payload capacities, not RSS, mapped storage,
allocator usable size, peak query ownership, elapsed time or Pi qualification.
Debug diagnostic wall times are not ordinary timing evidence.

## Restored source and next step

All observer mounts have been removed. `g2-closeout-1/` completes 10:19:33
(session 77984) with the exact gate-3 fingerprint, published force.rs,
format/diff checks passing, and unchanged P2/P3/M1/E1/G1 identities.
All sessions are terminal. No commit, push, release or full trace occurred.

G2 is correctness-validated with measured memory savings, but remains isolated
and not performance-accepted. Ordinary matched per-draw timing is now COMPLETE;
`G2-PER-DRAW-TIMING.md` is the timing authority. Panel 1 completes 10:27:02 UTC
(session 66246), review 10:27:08, all 128 distributions and 96 clock brackets
checked. There are 25 flags, no retries or discarded samples. Nonempty cold
medians improve -1.29%, -0.58%, -5.30%, but warmed tails and unflagged point
controls are mixed/adverse. The apparent empty warmed gain is confounded by
slow flagged baseline A1; it is not a speed claim. Do not repeat this panel.

Temporary Cargo mounts were removed before timing; g2-closeout-2 verifies
exact gate-3 source and unchanged P2/P3/M1/E1/G1 identities. Existing evidence
is still unexhausted; full tracing remains disabled. Next audit the remaining
current-source mechanisms against saved traces and allocation owner records.
