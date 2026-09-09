# P2: borrowed aggregate row input instead of repeated full-binding copies

Investigation only, while P1's source-frozen evaluation runs. No aggregate
code is changed and no new capture is requested. This is not a selection or
performance claim. Source checked against saved df737a6f o4 attribution again.

## Evidence and current mechanism

The saved o4 caller stacks lead through pipeline -> pump -> probe_pass -> pump
-> probe_pass -> run_node -> run_leaf_fast -> run_leaf_pinned -> emit_node_batch
-> AggregateSink::emit_batch -> fold_batch_rows -> fold_scratch_row ->
load_group_key. Relevant exclusive owners are fold_batch_rows 9.91%, key load
6.64%, shape-cache check 6.23%, row fold 5.98%, and group lookup 3.46%. These
are historical statistical owners, not predicted additive speedups. The trace
has 1.32% unresolved-stack CPU and 14.79% missing leaf source locations.

`fold_batch.rs`, `groups.rs` and `run/leaf.rs` have no diff between df737a6f and
the current baseline. `fold_row.rs` only changed old work-pressure plumbing;
current `maybe_spill_groups` is not the old sampled budget-pressure cost.
The main ordinary baseline o4 median is 27.213 ms over 500k Sale facts, a
three-way join and 64 output groups. One output group count is not input work.

The current pinned leaf gathers its leaf words and emits a one-survivor
LeafBatch. Pipeline input previously copies the parent's entire binding row.
The sink then refreshes/compares the shape cache, copies cached outer slots,
copies every key word into binding_scratch, then copies just the group words
from binding_scratch into key_scratch and reads fold arguments from it.
`emit(&Bindings)` also copies the whole binding array before the same row fold.

This is **copy/routing work, not per-row heap allocation**. The 66-family warm
counter roster and complete census rule out that conflation. o4's 40 requested
bytes/public operation fit WorkContext plus a two-word dense finalization key.

## Falsifiable experiment after P1

Consider making the ordinary row fold consume a borrowed word getter, shared
by scalar and batch sources. Only gather a complete binding into existing
scratch when the exact dedup regime requires it. A checked distinct witness
or active physical set-traversal witness permits skipping that copy; output
arity or a count aggregate alone never grants that permission. Read only the
group and fold/Pack inputs directly otherwise.

Keep the established constant-group multirow reductions, exact floating
accumulator sharing, and dedup semantics. Do not introduce a second persistent
binding representation. Isolate singleton batch dispatch/cache savings from
bulk reductions if needed for attribution. `LeafBatch::source_of` currently
linearly searches key_slots, so replacing copies with repeated source searches
could lose: measure both small pinned leaves and wide/multirow batches.

This should lower copied words/routing CPU for witnessed distinct grouped
folds. It should not claim reduced warm allocation. It may reduce usage of the
existing binding scratch but shortening Vec lengths is not a retained-memory
claim; don't bolt on a new allocation policy to make that claim.

## Correctness obligations

- Preserve the gate order: representation spill/error/cardinality checks and
  complete-binding or union distinctness before any count or accumulator update.
- Bindings, written Union and DnfUnion have different dedup keys. `aim` changes
  real slot count and union spans; cached layout must not outlive those changes.
- Keep the physical traversal witness scoped to the execution; reset clears it.
- Full-width group spans include multiword scalar/interval representations;
  packed intervals read both endpoints. A getter must be word-addressed, not
  confuse one logical field with one u64.
- Keep signed/unsigned widened sums, exact F64 Sum/Mean primary/alias handling,
  checked group cardinality, empty aggregates and sticky failures unchanged.
- Preserve sparse/repeated/scrambled survivor indices and keys sourced from
  both the batch and outer bindings. Zero/one/many-row batches and changing
  key-slot layouts are separate cases.
- Compare owned outputs and exact accumulator behavior against a scalar
  reference across dense/hashed groups, ordinary/witnessed/physical distinctness,
  written/DNF unions, Pack and forced representation-flush tests.
- Verify no warm allocations are added. Broad ordinary controls must retain
  o1/o2/o3/o4/o5, j5, stats, cheap point and join/overlap families, not o4 alone.

## Separate M2 follow-up

`for_each_ram_group` currently allocates a key Vec only for dense groups. Both
resident finalization and spill staging call it. A likely small cleanup is to
borrow/take-and-restore the sink's existing key_scratch, passing it explicitly
to the helper; restore on typed-error paths before propagation. The current
finalization closure borrows the entire sink through emit_group, so use a
clear split borrow or existing take/restore idiom, not an extra scratch owner.
Keep this isolated from P2 CPU routing until its own allocation test is red on
baseline and green on the candidate. The general census schema did not itself
exercise this closed-domain dense-key allocation; the suite counters and
current source identify it, but direct targeted site/model proof remains due.

## Further ownership and test audit during A2 (September 9 UTC)

`DedupState::consider` lives in `exec/sink.rs`, and every production caller is
in this aggregate implementation. Its current contracts are more precise than
"dedup needs a complete row":

- Bindings inserts the complete slot array.
- Union/DnfUnion copy their ordered spans into `union_scratch`, then insert
  that exact key. The preceding full `binding_scratch` copy is only a staging
  representation, not part of the seen-set contract.
- Elided does not read either scratch. Physical distinctness bypasses
  `consider` while preserving the ordinary Bindings seen-set for reuse.

This supports a later single **exact dedup-key scratch** owner, read from the
same borrowed row input, rather than keeping both a complete binding staging
array and a second union array. Do not bundle this with an unmeasured caching
redesign. The first CPU discriminator remains witnessed grouped rows, where
no dedup-key materialization is required at all. Cold construction storage and
warm per-row copy traffic are separate measurements.

Important edge cases for any consolidation:

1. A written-union key can be wider than `real_slots`: aliased Sum/Mean or
   repeated head terms repeat spans. Preserve span order and repetitions;
   sizing the scratch by slot count, sorting spans, or uniquing them silently
   changes the fixed seen-table key contract across `aim`.
2. Count-only global union has an empty key. Its zero-word seen-set must still
   collapse re-derivations; an empty scratch does not license distinctness.
3. `aim` invalidates cached routing even when `key_slots` is unchanged, because
   finds and DNF span order can change. `reset` removes physical distinctness;
   an allocation-free distinct execution must not strand the next ordinary
   execution without usable dedup scratch. `release_memory` must release any
   replacement owner and support subsequent reset/refill.
4. Constant-group dedup currently copies outer words **once per batch** and
   overwrites only varying key words. A naive getter that rereads every outer
   word per survivor can regress this path. Preserve that amortization or
   demonstrate the cost is acceptable with a discriminating ordinary control.
5. Production leaf key-slot arrays are unique: `validate_with_signatures`
   rejects a repeated variable in an occurrence's partition, and
   `Executor::with_batch_size` expands disjoint variable spans. Thus the old
   sequential assignments and `LeafBatch::source_of` agree on their source.
   Sparse/repeated **survivors** remain valid and must not be conflated with
   duplicate slot metadata. No unchecked-indexing optimization is needed.

Existing concrete regressions to retain include
`shared_batch_reductions_follow_selection_and_layout_changes`,
`union_reaim_invalidates_fold_sources_even_with_the_same_leaf_layout`,
`written_union_does_not_share_float_inputs_that_alias_in_only_one_rule`,
`the_union_seen_set_keys_head_projections_across_rule_layouts`,
`the_dnf_union_seen_set_keys_shared_slot_arrays_across_clone_layouts`,
`interval_group_keys_span_both_words`, `dense_group_tables_match_the_hashed_map_word_for_word`,
and the Pack/spill/release tests. These already exercise important semantics,
but a batch-versus-scalar test using the same new row-fold helper is **not an
independent numeric oracle**. Keep explicit expected integer/F64/Pack results
and the independent engine oracle too.

A useful new structural discriminator is a store-free row getter that records
the exact word slots read. Under a checked distinct witness it should read only
the group words and actual Fold/Float/Pack arguments (Float aliases once), not
unrelated existential slots. Non-distinct Bindings must still read the entire
binding for dedup; Union/DnfUnion must read exactly their dedup spans, in order,
before group mutation. Run this alongside no-new-warm-allocation assertions and
ordinary o4/constant-group/union/wide/Pack controls. This is not another trace.
