# Native finite Event partitions

A finite observable stays an ordinary relation: `(group, value, when: event)`.
At each world in the group's parent, exactly one value applies. Pointwise keys
and containments express that contract. `EventPartition` is an owned library
helper for validating and transforming the indexed Event roster; scalar labels
remain ordinary columns. It is not a new database field, a source law or a
parameterized `Model<T>`.

## Representation and admission

The helper retains a parent Event and a `Vec<Event>` behind one shared owner.
Cell position is roster identity. Empty cells keep their positions, even when
several positions have the same empty Event. The parent can be empty while its
underlying world space remains nonempty. The representation introduces no
second Event encoding and no per-cell probability weights.

`EventPartition::on(parent, cells, limits, control)` first aligns every cell to
the parent's context. It intersects each cell with parent, checks disjointness
against accumulated coverage, and requires that coverage to equal parent.
Overlap outside parent is harmless. Different contexts refuse even if parent or
all cells are empty. An empty roster is valid exactly when parent is empty.

For an observable with repeated scalar values, union equal-value rows before
admission. The helper validates **indexed cells**, not scalar equality or
database whole-fact identity. A gap is an error, never a missing numerical value
silently filled with zero. Parent restriction is structural evidence selection;
it neither normalizes a probability nor changes the original world support.

## Operations

| API | Result |
| --- | --- |
| `parent()` / `cells()` | Borrow the original parent and ordered admitted roster |
| `locate(world, control)` | Unique cell index at a legal world; `None` outside parent |
| `select(indices, control)` | Union named cells; empty selection returns an owned empty Event |
| `coarsen(destinations, output_cells, limits, control)` | Group cells by an ordinary supplied value mapping |
| `refine(right, limits, control)` | Same-world conjunction of each pair; parent is the intersection of parents |
| `pullback(map, control)` | Pull every cell and parent through a checked deterministic readout |
| `readout(target, codes, control)` | Convert a full partition and legal per-cell codes into a checked map |
| `Space::count_partition(events, limits, control)` | All exact-count buckets, from zero through the roster length |

Coarsening checks one valid output index per input position, including empty
cells. It retains every declared output bucket. Refinement emits the full
ordered pair roster, with index `left_index * right_length + right_index`.
It intersects conditions on the same world; correlated inputs stay correlated.
Neither operation asserts independence or manufactures a product source.

Readout conversion requires full coverage of the original world space; a
partial parent cannot invent values outside itself. All supplied target codes
must be legal, including those attached to empty cells. Repeated codes can
merge cells. Each target bit is the union of cells whose code sets that bit.
The result is an ordinary checked `CoordinateMap`, so existing information and
relation operators can consume it. Surjectivity needs its separate certificate.

Pullback preserves partition laws. Image can map distinct cells to the same
world and create overlap; there is deliberately no unchecked partition-image
constructor. After image, ordinary value grouping and partition admission must
establish the desired observable contract again.

## Counting and derived certainty

`count_partition` starts with bucket zero full and all remaining buckets empty.
For each roster Event E, it updates buckets in descending order using
`C'_k = Ite(E, C_(k-1), C_k)`. The descending traversal keeps both inputs from
the previous prefix. Each roster position counts once. Equal Event handles at
different positions are not deduplicated.

For two cards that have the same holding condition H:

| Ordinary count value | Event |
| --- | --- |
| 0 | !H |
| 1 | Empty |
| 2 | H |

Those three rows remain available for storage and queries. Mapping both count
0 and count 2 to one ordinary outcome and coarsening produces full coverage for
that outcome: an uncertain input can have a certain derived value. No probability
calculation is required. An empty dynamic roster produces the single full
zero-count bucket in its explicitly supplied space.

## Ownership, limits and evidence

`PartitionLimits::cells` defaults to 100,000 retained cells, including empty
ones. Constructors/coarsening bound their declared output; refinement checks
the complete pair count before allocation; counting checks n+1. Inherited
partitions cannot grow under pullback. Kernel limits, fallible vector allocation
and cancellation remain active. A failed operation publishes no partial roster;
canonical kernel intermediates can remain in their existing owners. Retained
partitions and extracted Events outlive their original inputs and database.

[Partitions.lean](../crates/bumbledb-event/semantics/Partitions.lean) adds thirteen
reports for evidence-relative admission, empty rosters, value grouping, refinement,
pullback, exact readout bits, legal readout construction and the count recurrence.
Its counterexample proves that even a valid partition can have overlapping images.
Native indexing, alignment, graph operations, allocation and resource handling
remain tested correspondence obligations; these are not extracted Rust proofs.

Seven native tests include 12,288 two-cell admission cases, 12,288 three-event
count rosters, all 1,296 two-by-three full partition refinements, value regrouping,
partial parents, readouts, illegal empty-cell codes, foreign contexts, budgets,
cancellation and a 62-coordinate symbolic case. The database consumer stores the
count roster using ordinary scalar values with native pointwise dependencies,
rejects a coverage-breaking deletion, reopens and rebuilds the partition after
closing the database. Empty buckets survive persistence. The compiled doctest
shows counting, regrouping and evidence-relative admission.

The [semantic run](event-evidence/native-partition-semantics/check.json) contains
163 native reports across ten modules. The
[native qualification](event-evidence/native-partition-qualification/check.json)
and [handoff audit](event-evidence/implementation-handoff-partitions/check.json)
pin exact sources and checks. This completes the indexed finite partition host
building block; query-level grouping/constructive heads, exact expectation,
measured sources and the remaining M0–M8 gates stay open. No release is made.
