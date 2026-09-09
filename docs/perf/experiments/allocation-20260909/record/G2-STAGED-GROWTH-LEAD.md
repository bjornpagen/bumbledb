# G2 lead: stage occupied entries, grow the unpublished table in place

Update: G2 is now implemented and correctness-gated in its isolated worktree.
`G2-STAGED-GROWTH-REVIEW.md` is the current authority: the matched saved audit
includes scratch explicitly and confirms the four-owner model. Ordinary
timing is still pending; the candidate is not performance-accepted. The
design/model below is preserved as the pre-implementation hypothesis.

G1 ordinary timing is complete in `G1-PER-DRAW-TIMING.md`. Its real retained
memory saving remains valid, but the fixed per-draw panel does not demonstrate
a consistent speed win. Preserve the adverse controls/tails and do not repeat
an identical panel hoping to accept it. This is the next representation
question from the SAME saved construction evidence, not a new CPU trace.

## Source audit and one growth implementation

Current `Colt::scratch` is a Vec<u64> used only by grow.rs for one key at a
time. new.rs initializes it, reset retains it, and release_memory drops it.
No probe, iteration, selection or caller observes its length/content. It is
not cloned by clone_bound_from. This existing owner can instead stage dense
key/child records; a new owner, mode, fallback or allocator is unnecessary.

Every production grow_map call is in force.rs::ingest_one while force_fresh
holds a local unpublished Map. It owns the three arena tails. Other published
maps precede them; child promotion allocates unforced nodes/chunks, not maps.
The only direct call outside ingestion is the synthetic test in admit.rs,
which grows a copied published Map. That artificial test contract must not
force production to maintain two growth implementations. If this experiment
is chosen, document grow_map's actual construction-only contract and replace
the test with equivalent protection of earlier published maps and real force
rollback. Do not silently weaken error protection or count obsolete tests.

## Candidate sequence, not yet implemented

1. Check the three tail extents and representable growth/staging sizes. Take
   the existing scratch vector and stage each occupied key plus its packed
   child word in original dense order. Reserve fallibly and checkpoint while
   gathering; no earlier published map can change.
2. Reserve the expanded control and bucket tails at their EXISTING bases.
   Complete reservations before destructive mutation. Dense already contains
   exactly the current distinct-key count; its entries can be rewritten.
3. Extend the tails and clear only this map's controls, cooperatively. Old
   bucket words need not be cleared: empty controls make them unreadable.
   Do not copy the full table or zero irrelevant bucket payloads again.
4. Iterate scratch.chunks_exact(arity+1). Hash the key slice, find its new slot
   with the existing bucket probing rule, write key/packed child and update
   the existing dense index at the SAME ordinal. No sorting or permutation.
5. Commit the new bucket count after success; bases never change. Restore
   the taken scratch owner on EVERY Result path, clearing used content while
   retaining capacity for ordinary reuse. Any error after mutation must go
   through force_unforced's existing full construction rollback, never expose
   an incomplete current map. Earlier published maps remain readable.

Use a single growth algorithm, not G1's append-plus-compaction alongside a
new alternate mode. Keep M1's duplicate-boundary policy change isolated for
now. No public Rust/TypeScript, persisted-layout, load-factor or quota change.

## Four-owner arithmetic, including the bigger scratch buffer

`g2-geometry-1/` calibrates baseline and G1's three-arena models to their
saved observations, then includes scratch in ALL variants. For the saved
100,000-row / 43,236-key width-one root, growth occurs at 13,107 and 26,214
keys. The scratch-capacity prediction is derived from current reserve_pool's
same geometric reservation, not an uncounted temporary or a new policy.

| These four owners only | Published append | G1 compaction | Proposed staging |
| --- | ---: | ---: | ---: |
| Three-arena capacity bytes | 4,423,680 | 3,604,480 | 2,490,368 |
| Scratch capacity bytes | 64 | 64 | 419,424 |
| Combined retained bytes | 4,423,744 | 3,604,544 | 2,909,792 |
| Allocation requests | 22 | 21 | 22 |
| Requested bytes | 7,176,224 | 6,094,880 | 5,052,784 |
| Whole-table relocation bytes | 0 | 3,499,620 | 0 |
| Scratch payload bytes written | 314,568 | 314,568 | 629,136 |

Staging is not a request-count win over G1: it predicts one more request.
It predicts 694,752 fewer retained bytes and 1,042,096 fewer requested bytes
than G1 for these four root owners. Do not hide the 419,424-byte retained
scratch or add this projection to actual whole-query savings. The two write
rows are different operations, not a complete memory-traffic accounting or
speed prediction. Other COLT pools, images, allocator usable size, RSS, mmap
and query peak are outside this arithmetic model. G2 is NOT measured yet.

## Required discriminators before acceptance

- Baseline/G1 versus candidate must count the scratch capacity explicitly,
  not just ctrl/bucket/dense, including after repeated reset/rebind and release.
- Zero/fixed/wide keys; no/one/multiple growths; boundary and duplicate keys;
  singleton and promoted children; maximum packed child payloads; exact dense
  order, resumed tokens, earlier root/child maps and same-shape cloning.
- Failure before staging, after staging/reservation, during control reset,
  and during reinsertion: restore scratch, roll back the unfinished force,
  retain older published maps/children, and succeed on a healthy retry. Keep
  synthetic seam checks distinct from public-operation guarantees.
- Validate allocation failure/overflow ordering without allocating enormous
  test buffers. Retained capacity may grow on a failed reservation sequence;
  do not promise process-wide OOM recovery or undo successful reservations.
- Actual saved S/seed-1 SQL windows and all active/parked owners, with a
  matched observer including scratch in both baseline/candidate records.
  Reuse existing inputs, then ordinary targeted timings with complete
  controls and tails. No full trace is authorized.

At the time this lead was written, no G2 engine edits or worktree existed.
The next action was a separate
red/green construction/staging experiment from published source, preserving
the frozen G1 implementation and its complete evidence. This lead is open,
not an exhaustion declaration or a reason to bundle other unaccepted changes.
