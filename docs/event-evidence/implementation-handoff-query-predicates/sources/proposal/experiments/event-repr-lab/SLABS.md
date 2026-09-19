# Compact essential storage: one payload, exact interning

Both stores are implemented in [essential_store.rs](src/essential_store.rs).
The same executable selects `EVENT_LAB_ESSENTIAL_LAYOUT=enum` or `slab`, with
`enum` retained as the default. The [derived essential-table results](ESSENTIAL-MEASUREMENTS.md) still trail
fixed-tail controls. Sampling and source inspection identify a concrete layout
confound: `essential_raw::Arena` stores each `Node` in its arena and again as
an interner key, including duplicate table buffers. Several operators clone
that owned node merely to retain branch data across a mutable recursive call.

Keep the [essential-coordinate normal form](ESSENTIAL.md), constructor proofs,
physical order, external handles and algebra unchanged. Compare two storage
implementations in the same executable before changing the cutoff or operation
schedule. This separates the algebraic choice from its current allocation shape.

## Candidate resident storage

The slab raw record is sixteen bytes, checked at compile time:

```text
record.variables : u64     exact essential-coordinate mask
record.payload   : u64     branch children + coordinate, or table offset
```

Constant false occupies the distinguished zero record. Its complemented
reference denotes true. A tagged table payload holds an offset into a contiguous
`Vec<u64>`; its word length follows from the essential mask. A branch payload
can pack two 28-bit complemented child references and a six-bit coordinate,
leaving spare tag bits. The present two-million-node refusal fits comfortably
inside that reference range. Check the packing limit at construction; never
truncate an index. Physical references still have a separate public scope.

Both stores retain exact counts in the existing side array for the derived
algorithm. Slab interner collision links occupy a separate index array. The hash index maps a full
content fingerprint to a chain head; a probe compares the complete variable
mask, branch content or table words before accepting equality. A fingerprint
never establishes identity. This pattern already exists in the lab's finite
carriers. Table contents occur once in the resident arena, not in every key.
On a miss, append words only after confirming that no equal table is resident.

The borrowed view reads table slices and copies branch fields. Metadata-only
reads access the mask/tag directly. The existing algorithms retain their owned
node copies where they already copied nodes across mutation; eliminating those
copies is a separate control. Borrows end before either growing vector moves.
No allocator address becomes identity. No garbage collection, persistence or
concurrent mutation is implied by a slab.

## Keep a second allocation change separate

The current alignment path also allocates axis lists and temporary bitplanes.
For a fixed local cutoff, those arrays have a proved bound. Stack or retained
scratch storage is a second control, with its own label and timings. Recursive
frames must own disjoint scratch or use a checked stack discipline. Measure
slabs first, then bounded scratch; changing both at once obscures the cause.
Read-only classification can use the same borrowed records without mutating
interning state, but its temporary memo is still charged separately.

## Acceptance and falsifiers

Both layouts pass the full shared algebra, support, ownership, transport and law
suite: [enum](results/slab-enum-check.json), [slab](results/slab-complete-check.json).
The raw test forces all fingerprints to zero, matches raw IDs and every stored
node against the enum store, exercises duplicate rejection, vector growth,
cloning, operations, abstraction, renaming and product, then reads old roots.
It also passes in the [optimized standalone classifier check](results/slab-classify-check.json).
The shared suite checks that split memory estimates sum to the carrier estimate,
including the actual cloned carrier. These are correctness results, separate
from native timing evidence.

In the same native binary, compare:

- Fresh and repeated Coup joins in both ordinal and card-coordinate layouts.
- The owned eighteen-coordinate relation query and its eighty novel outputs.
- Full sixty-coordinate counter algebra, including closure and residual outputs.
- Shared-parameter law observation and direct relationship filtering.

Retain construction, query, transport and observation boundaries. Measure the
actual post-clone owner footprint, as the classifier smoke regression requires.
Report arena/interner/word/cache bytes separately, along with process RSS; these
are not allocator-complete heap measurements. Preserve node and table counts so
a physical-layout control cannot silently alter the normal form or answer.
Only matched modes in one executable establish a speedup. Fixed-tail and dense
controls remain in the sweep. Lower retained bytes alone do not choose a winner.

## Memory accounting

Native join, owned, symbolic relation, law and classifier rows now include
`input_storage` and `final_storage` for essential carriers. Other carriers
report null for this optional breakdown and retain their existing byte estimates.
The fields are record capacity, table-word capacity, interner capacity, metadata
capacity and raw operation-cache capacity, followed by logical record/table/word
counts. Metadata includes order/rank vectors and the exact-count side array.
The interner includes collision links in slab mode and duplicate key records in
enum mode; duplicate key table buffers are charged to table bytes.

Hash entries are estimated using tuple size plus one control byte per usable
entry. Allocator metadata, some hash slack and temporary traversal scratch are
not captured. External expression/signature memos remain in their original
separate accounting. All counters are sampled outside timers, from the actual
post-clone owner where cloning occurs. Process RSS is a separate whole-process
peak and cannot be interpreted as the arena's heap size.

`slab_sweep.py` runs both layouts serially with fixed shuffled order and the same
native executable. Packed64/512 and dense-dispatched controls run once per
workload. `slab_comparison.py` rejects different binaries or mismatched logical
node/table/word counts and output summaries, keeping the latest supplied process
per exact case. It never chooses the fastest repetition.

## Native result: smaller storage, mixed latency

The [matched tables](SLAB-MEASUREMENTS.md) retain the same native executable,
506 cases and 146 enum/slab case pairs. All paired logical record, table and word
counts match. The initial sweep uses five samples; the focused core-query repeat
uses nine. Table medians below use that repeat. Carrier KB excludes external
query memos; query timings retain each lane's complete original contract.

| Essential512 query | Enum ms | Slab ms | Enum carrier KB | Slab carrier KB |
| --- | ---: | ---: | ---: | ---: |
| Ordinal Coup, clover, expression memo on | 28.189 | 28.094 | 5,078.2 | 3,618.1 |
| Card-coordinate Coup, same query | 82.080 | 77.671 | 14,911.6 | 10,929.1 |
| Owned 18-coordinate relation program, bit-major target | 2.414 | 2.390 | 495.0 | 302.3 |
| Full 60-coordinate counter relation program | 3.904 | 3.881 | 770.0 | 497.6 |

The retained carrier reduction is roughly 27–39% in these cases. Other tested
sizes/orders have their own values in the raw breakdown. This is a capacity
estimate, not an allocation census or whole-process RSS reduction. The exact
normal form remains identical; compact storage has removed duplicate payloads
and changed record/interner overhead, not omitted logical work.

Latency does not improve uniformly. Some modest differences change direction
between processes; the direct scalar classifier has a repeatable regression
recorded below. Even the compact essential carrier remains behind controls:
the same nine-sample repeat gives dispatched dense 0.633 ms and packed512
3.358 ms for ordinal Coup, packed512 0.736 ms for the owned relation program,
and packed512 0.997 ms for the full counter program. These query contracts
cannot be replaced with a weaker result to improve the ranking.

The physical change is itself a bundle: smaller records, contiguous table words,
removal of duplicate keys, and a different exact interner. It does not isolate
which part changes latency. Both layouts now share runtime storage dispatch;
these timings do not measure that dispatch against the preceding executable.

Keep the enum control and its default. The slab is a valid compact alternative,
not a selected universal backend. The subsequent
[word-classifier experiment](WORD-CLASSIFIER.md) consumes borrowed/aligned tables
instead of repeatedly decoding/evaluating individual assignments. It retains
both stores, the scalar classifier and the constructive-cell control. That is
a new matched execution comparison; the scalar results below remain historical
evidence from the slab executable, without attributing cross-build differences.

## The classifier regression is retained

The [eleven-sample independent repeat](results/slab-classifier-repeat.json)
confirms that smaller storage can cost time in the present scalar consumer.
For ordinal Coup, clover inclusion, essential512 direct classification takes:

| Pair memo | Enum fresh ms | Slab fresh ms | Enum min–max | Slab min–max |
| --- | ---: | ---: | --- | --- |
| Off | 91.471 | 106.768 | 90.438–93.765 | 105.421–128.657 |
| On | 7.726 | 9.044 | 7.561–7.885 | 8.874–9.283 |

Both retain exactly 717 records, 283 tables and 2,222 logical words. The enum
carrier occupies 108,739 estimated bytes; slab occupies 60,855. Neither direct
query adds resident nodes or bytes. Pair memo reuse reduces warm query time to
about 0.11 ms in both layouts, hiding the first-use regression.

The [actual storage/evaluation symbols](results/assembly-slab-storage.json)
show compact branch extraction with shifts, `UBFX` and `BFI`. Decoding a slab
table derives its length with `CNT`/`ADDV` and checks the word-slice bounds.
The scalar evaluator calls this decoder, then gathers an assignment's local
index one essential coordinate at a time. This is concrete repeated work that
a borrowed word view can avoid. Static assembly alone does not attribute the
whole measured slowdown to any one instruction or prove the proposed kernel
will win.

[Eight operator symbols](results/assembly-slab.json) also retain the actual
word kernels. Boolean combination, permutation and cofactor routines contain
NEON Boolean instructions; the recursive Apply and table constructors still
perform ordinary scalar/control work. These are instruction sites in named
functions, not a dynamic instruction count or a vectorized whole query.

The final evidence comprises 105 serial native processes: a smoke run, the broad
sweep, a focused core repeat and the classifier repeat. All pass. Raw samples,
phase ranges, source/executable hashes, split capacities and process RSS are
retained in the [comparison record](results/slab-comparison.json).
