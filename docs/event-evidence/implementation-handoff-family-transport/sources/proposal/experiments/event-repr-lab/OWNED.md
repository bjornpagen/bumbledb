# Compute new Event results in their retained owner

The first transport lane restored inputs **and expected outputs**, then reran
the query in a carrier clone. That establishes semantic rebasing and canonical
replay, but does not show that new query results belong to the retained arena.
This experiment tests that specific gap while keeping production crates unchanged.

## Region operations do not require cloning an arena

The lab now separates two internal capabilities:

```rust
trait RegionOps { /* exact region operations and structural views */ }
trait Carrier: RegionOps + Clone { /* arena construction and snapshots */ }
```

`RegionOps` is also implemented for `&mut C`, forwarding to the actual arena,
including fused product, canonical table imports and symbolic graph views.
`Algebra<&mut C>` therefore executes the same relation program while borrowing
the owner. It cannot clone its borrowed manager. Construction and snapshots
remain available to experiment setup and explicit callers that own a Carrier.
This is an internal Rust interface split; the public database field stays
`event` / `Event`, without a model parameter or backend parameter in the schema.

The existing finite relation checks now run through both owned and borrowed
operations. The [shared check](results/owned-operations-check.json) includes
composition, converse, residual adjunction, closure, all carrier operations and
new owner-publication/lifetime checks. These are semantic tests, not native timings.

## One retained owner for the computation and result batch

`Space::compute` holds the owner mutex, lends its carrier to a compiled lab
program, publishes the returned roots and produces a Batch retaining that owner.
The result keys remain sixteen-byte copied values. One-bit carriers publish
complementary classes, so either polarity is already valid; the root-BDD baseline
retains its different complement behavior explicitly.

Only inputs are admitted before the query. The program's outer operation memos
and temporary vectors are released when it returns. The canonical arena retains
the computed regions, and the output batch keeps that arena alive after the
input batch and executor are dropped. There is no per-binding atomic retain.

`compute` is a **trusted lab program boundary**: its closure must return roots
constructed in the borrowed arena. It is not a public unchecked-root constructor
or a general user callback API. The native adapter still stores scope/region as
ordinary word columns; native Event typing and decoding remain production work.
The prototype also does not establish transaction rollback, recovery after a
panicking program, concurrent publication, multi-owner spill or within-arena GC.
Those guarantees cannot be inferred from the successful lifetime path.

## The native experiment

[owned_bench.rs](src/owned_bench.rs) uses the full existing relation fixture:
actual canonical images, COLT and Free Join, followed by closure, converse,
residuals, May and Must. For each source/target combination:

1. Construct and serialize the 24 **input** Events. Drop the source carrier.
2. Create a fresh target owner and restore only that input packet.
3. Build native rows with the target's actual scope token and restored region IDs.
4. Borrow the target carrier, prepare the relation program, execute Free Join,
   and publish its 80 result roots in that same owner.
5. Require at least one previously unpublished canonical class. Compare every
   output with the direct matrix/bitset oracle, and its complete packet bytes
   with an independently constructed oracle arena.
6. Drop the executor, input batch and original owner variable. Require the
   result batch to be the sole remaining owner, then resolve and verify all
   outputs again. Dropping that final batch must reclaim the owner.

The oracle arena is never the query manager and its roots never enter the input
packet. This avoids making ownership validation pass through preloaded answers.
The test does not equate numeric IDs from independent arenas; canonical byte
comparison is separate from resident-ID comparison within a scope.

Destinations match the transport cases: the same carrier in bit-major and
face-major order, plus common packed512/bit-major for other source carriers,
at twelve and eighteen coordinates. The source order remains explicit.

## What the times include

Record owner setup, checked input restoration, program setup, query execution,
and total owned computation separately. Owned computation includes acquiring
the owner lock, preparing the program, executing it, publishing results and
dropping temporary program state. The query phase excludes cardinality and law
observation. Program setup constructs identity, checks full-product support and validates
the four maps; its cost is deliberately visible. The [diagonal experiment](DIAGONAL.md)
keeps symbolic and enumerated constructors as matched controls.

Source construction/export, strict packet decoding, native image/executor
preparation and independent oracle comparisons are outside those phases.
Target setup uses the fixture's unconstrained binary-product constructor.
Input restoration includes a fresh support-map proof, currently using the
source carrier as verifier. Different import paths can leave different internal
Apply caches; these are fresh restored arenas, not identical cache snapshots.

The next representation review should use these costs rather than silently
excluding required program preparation. A general full fibre product of
constrained endpoint supports is not the same as the fixture's free bit product.
The original identity-bitplane constructor exposed that same problem even for
a free binary product; the diagonal experiment removes that specific enumeration
boundary and adds a separate large symbolic relation fixture.

## Completed ownership evidence

All fourteen carriers passed [the initial five-sample sweep](results/owned-sweep.json):
82 source/target/size cases, each checking 80 computed output Events and final
owner reclamation. [The retained baseline table](OWNED-BASELINE.md) records the
original enumerated-identity constructor. At eighteen coordinates in bit-major
order, packed64's median program/query/compute times were 1.022/0.829/1.850 ms;
packed512's were 1.246/0.698/1.972 ms. Preparation was the dominant cost in both,
which motivated the diagonal experiment. Those old times are not a same-binary
comparison with the new symbolic constructor. The subsequent [matched sweep](results/diagonal-owned-sweep.json)
passes 246 cases across native/symbolic/table construction in one executable.
Packed512's eighteen-coordinate bit-major total changes from 2.258 ms under the
table control to 0.717 ms under the native symbolic constructor. Dense keeps its
direct fill; forcing symbolic formula construction there increases its cost.
[The diagonal report](DIAGONAL.md) and [complete table](OWNED-MEASUREMENTS.md)
retain those boundaries.

## Reproduce

```sh
python3 proposal/experiments/event-repr-lab/run.py --lane owned --candidate packed512 --layout bit-major --transfer-import words --trials 7 --output my-owned.json
python3 proposal/experiments/event-repr-lab/owned.py my-owned.json
```

Use `--no-build` with a matching compiled revision. Keep timing sweeps serial
and separate from compilation. The generated table retains incomplete process
outcomes and distinguishes publication classes from the number of Event results.
