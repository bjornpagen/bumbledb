# P2 implementation sketch, before source changes

This is a prospective, isolated experiment, not an accepted optimization.
The ABC/CBA P1 protocol remains source-frozen. No new trace is authorized.

## Source-level discriminator

The saved o4 query is Count grouped by customer segment and product category,
not a Sum workload: five real binding words, two grouping words, three joined
relations. Pinned leaf emission hands a singleton LeafBatch to the aggregate
sink. Historical fold_batch.rs line 10 (outer-word staging) owns 3.13% of the
selected CPU by itself; total nearest fold_batch_rows owner is 9.91%.
Read the line sites as statistical attribution, not an additive speedup bound.
The warm allocation census is separate: no per-row heap allocation claim.

## One exact dedup scratch, borrowed fold inputs

**Revision after ownership accounting:** the full consolidation below is not
the first candidate. At R real words, K leaf words, and U union-key words,
baseline logical scratch/routing payload (excluding group scratch) is about
16R+8U bytes. An exact-key scratch plus two-word cached routes would be
24U+8K for a union (24R+8K for Bindings), before Vec growth/spare capacity.
It can increase small-query retained state. Elided can shrink, but that alone
does not justify changing all dedup routing. Measure actual capacities, not
these length equations, if this larger design is pursued.

The first isolated experiment instead makes the row fold consume a borrowed
word getter plus the existing exact full-binding input for dedup. Distinct
scalar/batch paths skip staging entirely. Non-distinct batches keep current
outer-once staging and read fold inputs from that staged slice (no repeated
LeafBatch::source_of search). Take/restore the existing binding scratch once
around a non-distinct batch so the group mutator can borrow the row without
aliasing the sink. Typed errors must restore it. Scalar emission stages only
when necessary, and borrows Bindings for its fold inputs. No new persistent
metadata, no new buffer, no capacity reduction claim, no singleton dispatch
change. Keep all constant-group reductions unchanged. The larger single-key
owner consolidation is a separate open lead, not bundled into this test.

1. Replace full binding staging plus union staging with one `dedup_scratch`.
   Its width is the seen-table key width, or zero for Elided. Written/DNF
   union keys may be wider than real_slots and must retain ordered repeats.
2. DedupState enumerates its exact source slots: 0..real_slots for Bindings,
   ordered span words for Union/DnfUnion, empty for Elided. Its insertion
   method receives the already materialized exact key. Zero-word keys still
   insert into the seen set; emptiness is not a distinctness license.
3. The ordinary row fold consumes a borrowed word getter. It still checks
   spill/error/cardinality and the seen-set verdict before mutating a group.
   Group words, integer/float arguments, and both Pack endpoints are read
   directly. Float aliases are accumulated only at the existing primary.
4. Scalar emission gathers only the exact dedup key, and only without a
   semantic/physical distinctness witness; it never stages a full binding
   merely to read grouping or fold inputs.
5. Batch emission must not reload all outer dedup words for every survivor.
   Replace cached_outer_slots with cached exact-key routing, partitioned into
   outer and varying entries. Each entry is (dedup destination, source word).
   Outer sources copy once per batch, varying sources once per survivor.
   Reuse this preparation in both row-wise and constant-group dedup paths.
6. Preserve the existing constant-group bulk reductions, selection order,
   repeated-survivor dedup, constant outer arguments, and column sharing.
   Do not change the executor, planner, floating arithmetic, or finalization.
7. The first experiment retains shape-cache dispatch. A direct singleton
   bypass is a separate follow-up only if the measured copy/routing change
   supports it. No new routing table is needed for group/argument reads;
   LeafBatch::source_of is initially the checked lookup. Wide-batch controls
   must decide whether this substitutes too much searching for copying.

This saves one persistent scratch owner but does not, by itself, establish
lower retained bytes: routing metadata and its capacities must be counted too.
Bindings remains epoch-checked; do not add a raw slice escape to bypass it.

## Focused regression obligations

- Checked-witness scalar input leaves irrelevant binding epochs unbound. The
  baseline staging path must fail that structural discriminator; direct
  grouping/Fold/Float/Pack reads must succeed with explicit expected results.
- Same for varying singleton/multirow batches with sparse, repeated, and
  scrambled survivor IDs, widths including interval endpoints, and layout
  changes. Distinct fixtures cannot replay a full binding illegally.
- Non-distinct Bindings still distinguishes existential words; Union/DNF
  still collapses only exact ordered projected/shared keys, including empty
  and repeated keys wider than real_slots. Reaim with the same leaf layout
  but changed source spans must invalidate all cached dedup routes.
- Constant-group tests retain outer reads once per batch. Avoid changing the
  known exact-float primary/alias and shared reduction tests.
- Reset clears physical distinctness; release drops new scratch/routing
  owners, restores on reset, and preserves typed sticky failure semantics.
- Warm no-allocation assertion after shape/group/seen capacities are primed;
  cold capacities measured separately, never confused with RSS/mmap bytes.
- Explicit independent I64/U64/F64/Pack expectations plus the engine oracle.
  Comparing two callers of a shared new helper alone is insufficient.

## Timing acceptance

P1 disposition must precede tracked P2 changes. Compare a standalone P2
binary against the published baseline; do not bundle an unaccepted P1 form.
Gate the exact binary in a private verified corpus. Ordinary (untraced)
OLAP/join scenarios and cheap read controls plus a store-free wide/constant
group discriminator are sufficient for initial selection. Preserve all raw
rounds and pauses; do not normalize host noise into wins or claim Pi results.
