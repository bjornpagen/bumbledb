# M1: map construction ownership and its benchmark model

Source audit during the source-frozen P1 evaluation. No engine change, build,
measurement, or new CPU trace in this audit. This separates two experiments;
neither is accepted from source inspection alone.

## First experiment: do not grow for an existing key

`force.rs::ingest_one` currently tests the next-distinct-key threshold before
checking whether the key exists. Change that decision only after an initial
probe reports a missing key. If growth occurs, probe again in the new table;
the old insertion slot is invalid. Existing-key append must still propagate
its fallible child/chunk allocation and preserve row multiplicity.

The result-set WordMap already solves this boundary in `wordmap/entry.rs`,
with a direct `duplicates_at_the_load_boundary_do_not_grow_or_rehash` test.
Reuse that reasoning when selecting the COLT control flow; do not invent a
new allocation policy or claim a corresponding WordMap fix is still needed.

Discriminator: 50 input rows, 25 distinct first-level keys, each repeated, with
a duplicate after the 25th distinct key. `force_nbuckets(50)` starts at eight
eight-slot groups. At 25 distinct keys, `(25 + 1) * 5 > 8 * 16` makes the old
code double on that duplicate. The candidate should retain eight groups. An
otherwise comparable 26-distinct-key input must still double. Vary widths
0/1/2/3/4/5/8 and input order. The zero-width case has at most one distinct key,
so is a correctness/control case, not a fabricated 25-key growth fixture.

Inspect table width, distinct count, dense length, live arena lengths and
retained capacities separately; also check child enumeration, later force,
clone_bound_from and cancellation rollback. A smaller table is not an RSS
measurement. The completed census confirms cold pool ownership, not prevalence
of this particular boundary case in production data.

### Newly identified dependent model

`crates/bumbledb-bench/src/displaced.rs::forced_spoke_map_bytes` repeats the
old policy using `(landed + 1) * 5`. After the engine fix it must use the actual
distinct count, with threshold tests in `displaced/tests.rs`. Its public helper
models the final one-word-key table's control and bucket lanes, **not** all
retired construction tables, dense indices, retained capacity, images or RSS.

At the stock displaced fixture's 1,048,576 positions and 453,241 distinct keys,
both thresholds still select 262,144 groups. The existing 34 MiB final-table
claim therefore remains unchanged by this isolated duplicate-growth fix. Do
not claim that this fixture's steady-state working set shrank. The boundary
tests are what distinguish the corrected policy; the existing large-fixture
layout test alone cannot catch a stale formula.

## Separate experiment: compact only once at successful construction finish

`grow_map` appends new control, bucket and dense tables; `clone_bound_from`
copies entire arena lengths, including retired construction tables. A possible
next experiment is a final compaction in `force_fresh`, not on every growth:

- Save the initial ctrl/bucket/dense bases already available there.
- Complete all ingestion and fallible rehash work as today.
- If growth left a later final table, copy its final control/bucket/dense
  ranges down to those initial bases, then truncate the construction tails.
- Update only the newly constructed local Map's bases before publishing it.

The force path does not construct nested maps while ingesting: child promotion
only creates unforced nodes/chunks. Thus these three initial bases bound a
private construction tail. Older published maps lie before the saved bases and
must remain byte-for-byte readable. `Vec::copy_within` supplies overlap-safe
movement; this needs no new persistent arena owner or indirection.

Keep generic `grow_map` behavior separate: direct tests/diagnostics can grow an
already-published map with later live tables, where truncating is not licensed.
Compacting once at force completion also avoids an extra copy on every doubling.

This is not free. It copies the final tables, may keep essentially the same
peak Vec capacities, and needs an explicit cancellation responsiveness audit
for large copies. On a failed construction, the existing outer pool mark must
restore all lengths and leave the root unforced; no partially compacted map may
publish. Test later old-map reads, repeated reset/rebind, same-shape cloning,
zero-width keys and mid-copy refusal if cooperative chunking is selected.

Accept only with measured cold construction and same-shape clone costs plus
live-length/capacity evidence. Do not bundle this representation experiment
into the smaller duplicate-threshold patch or into the current P1 candidate.

## Do not confuse map iteration with ownership cloning

The saved r4 hot symbol is `Colt::copy_map_batch::<1, false>` (batched map-key
enumeration), **not** `clone_bound_from`. `run_join` first calls `memo.bind`
and skips the cloning branch entirely on an active matching view. Thus the
saved copy-map CPU percentage is not evidence of repeated full arena clones
on warmed r4 executions. Any compaction claim must be tied to actual cold,
changed-view or same-shape cloning windows; the allocation census and source
ownership still motivate investigating its retained construction tails.
