# Relational identity must be a symbolic constructor

The original relation program enumerated an XYZ bitplane to construct `Id(X,Y)`.
At eighteen coordinates, that program preparation cost more than the packed
query itself. At sixty coordinates, enumeration would prevent the query from
running at all. A symbolic region manager does not establish a symbolic query
system if preparation still enumerates its universe.

## One mathematical constructor, competing physical algorithms

The diagonal on two binary faces is

```text
Id(X,Y) = intersection over i of (Xi iff Yi)
```

It is an ordinary Event: other coordinates remain free. The lab's `RegionOps`
now exposes `diagonal(pairs)` as a derived operation. Pairs name coordinates,
not row weights or an external probability model. Repeated and overlapping
pairs retain their usual equality meaning; an empty list selects full support.
All coordinates are checked before any trivial result is accepted.

Three algorithms coexist in one executable:

| Mode | Construction | Scope |
| --- | --- | --- |
| `native` | Delegate to the carrier's diagonal operation | Symbolic formula for diagrams; one direct table fill for finite carriers |
| `symbolic` | Conjoin coordinate biconditionals | Same formula on every representation, including a finite control |
| `table` | Fill and import the entire diagonal bitplane | Explicit cap at twenty ambient coordinates |

The mutable-borrow implementation forwards `diagonal` to its owner, preserving
the owner's physical override. Query operations do not acquire arena cloning as
a side effect. The implementation stays in the experimental internal interface;
the public database field remains `event` / `Event`.

This does not imply that every equality is compact under every order. The
retained grouped/interleaved equality experiment already demonstrates the
opposite. The large relation test uses corresponding X/Y/Z bits interleaved.
Changing a coordinate presentation may still require expensive normalization.

## Product validation precedes relation construction

A useful counterexample appeared while removing enumeration. The support
`X=Y=Z` is preserved by all face permutations, but it is not the full XYZ
product. Invariance under those permutations was therefore an insufficient
test for the relation program's ambient-space precondition.

The binary lab constructor now checks:

1. There are exactly three faces of the declared width, within the exact count
   and coordinate-mask range.
2. The support count equals `2^(3*width)`. A support is already a subset of that
   binary cube, so equality of these finite cardinalities proves it is full.
3. The four local coordinate maps are valid and preserve that support.

Only then does it construct the diagonal. The symmetric constrained-support
counterexample and a wrong face width are rejected explicitly. Exact counts are
obtained structurally for symbolic carriers; this test never exports a bitplane.

This is a correct check for this lab's **unconstrained binary products**. It is
not a production proof for a general fibre product over shared real parameters.
That constructor still needs endpoint support, environment and complete-fibre
certificates as described in the [map review](../../research/space-maps.md).
Transition constraints belong inside relation Events, rather than secretly
restricting the ambient product in which complement and residuals are computed.

## Independent semantic checks

[diagonal_checks.rs](src/diagonal_checks.rs) compares all three constructors and
the mutable-borrow path against direct small-world equality. Each of fourteen
carriers checks 1,530 support/pair-list cases, including repeated pairs,
transitive equalities and invalid-coordinate rejection. Three physical layouts
also compare a ripple-carry successor circuit and its analytical closure against
explicit eight-state tables.

Nine symbolic carriers additionally check sixty-coordinate identity and a
twenty-bit involution that flips the low and high state bits. They verify left
and right identity, composition with the inverse, converse, residual identities
and closure using exact resident IDs. Identity contains `2^40` XYZ assignments;
the containing presentation has `2^60`. No world table enters these tests.

The [shared check record](results/diagonal-check.json) is a correctness run, not
a native performance claim.

## A million-state program through the native executor

[symbolic_relations.rs](src/symbolic_relations.rs) adds a separate native fixture.
It constructs a non-wrapping counter from coordinate gates:

```text
R(X,Y) = (Y = X + 1) and X is not the maximum value
Goal(Y) = Y is the maximum value
Allowed(X,Y) = Full
```

Three ordinary relation rows enter the actual image/COLT/Free Join path. Its
one matched binding runs the same five-result program used by the finite
relation lane. For face widths six, twelve and twenty, the analytical answers
are:

| Result | Exact answer |
| --- | --- |
| `Star(R)` | `X <= Y` |
| `Converse(Star(R))` | `X >= Y` |
| `LeftResidual(R, Full)` | Full relation |
| `May(Star(R), Goal)` | Every state |
| `Must(R, Goal)` | Only the penultimate state |

The last answer excludes the terminal state, which has no outgoing transition.
The first answer retains every reachable pair; returning only a set of states
would weaken this query. The native owner receives only the three input roots.
Query results are computed in that owner and checked after the inputs and
executor have been dropped.

An independent arena constructs the answers with a direct comparator circuit,
not the closure algorithm or the query arena. Complete output packet bytes and
analytical cardinalities must match. Small-world tests check those comparator
and ripple-carry circuits against enumeration. Cardinality alone is never used
as a proof of output equality.

The largest state face has 1,048,576 states. This is deliberately a long chain,
unlike the earlier fixture's bounded components. Repeated squaring reaches its
fixed point in a number of iterations proportional to the state bit width.
This is not an implementation of the recursive Reach algorithm in
[Brand et al.](../../research/block-decomposition.md); it preserves full closure
and the four other requested outputs. A state-only query deserves its own
matched algorithm comparison.

One ordinary joined binding does not benchmark Free Join scaling. This lane
tests whether its computed Event stage can execute a genuinely symbolic
relation program without turning internal worlds into ordinary rows. The
existing finite lane retains its many-binding comparisons.

## Matched preparation costs

The [seven-sample same-binary sweep](results/diagonal-owned-sweep.json) passed
all 42 processes and 246 source/target/size/mode cases. Its native and symbolic
paths are intentionally the same formula for diagram carriers; separate
processes expose ordinary timing variation. At eighteen coordinates with the
same source/target carrier and bit-major order:

| Carrier | Table setup / owned compute (ms) | Native setup / owned compute (ms) | Forced symbolic setup / owned compute (ms) |
| --- | ---: | ---: | ---: |
| packed512 | 1.535 / 2.258 | 0.046 / 0.717 | 0.049 / 0.749 |
| packed64 | 1.256 / 2.087 | 0.015 / 0.843 | 0.013 / 0.809 |
| block64 | 1.473 / 2.583 | 0.015 / 1.109 | 0.016 / 1.111 |
| bdd-pair | 1.388 / 2.502 | 0.012 / 1.144 | 0.012 / 1.142 |
| dense-dispatched | 0.540 / 2.846 | 0.459 / 2.742 | 3.433 / 5.914 |

[The complete owned tables](OWNED-MEASUREMENTS.md) retain all fourteen carriers,
layouts, destinations, raw ranges and the legacy constructor separately. The
finite native and table controls perform the same direct fill on this full
product; their small difference is process variation. Forcing the diagram's
formula-construction strategy onto dense storage is substantially worse.

The result supports a mathematical diagonal operation with physical overrides,
not one universal construction algorithm. Query-only times exclude required
preparation, so the larger owned-compute improvement is the relevant result.
These formula constructors can also leave different internal Apply caches;
the experiment measures their complete downstream computation rather than
claiming identical query-cache states.

## Completed native symbolic evidence

All nine symbolic carriers passed [the seven-sample native sweep](results/symbolic-relations-sweep.json),
covering 27 carrier/size cases. The [complete table](SYMBOLIC-RELATION-MEASUREMENTS.md)
retains input, preparation, query and owned-computation phases. At sixty
coordinates, median owned-computation times were 0.753 ms for packed64,
0.821 ms for anchored BDD, 0.964 ms for packed512 and 1.607 ms for block64.
All used 21 closure iterations and checked the same five output Events.
These are single-process desktop medians; small gaps do not establish a universal
ranking or replace the earlier finite many-binding relation workload.

Packed512 retained an estimated 541 KB including intermediates/caches; its five
output roots serialized into 2,733 bytes. Block64 retained about 149 KB but its
output packet occupied 8,477 bytes. Arena estimates and packet bytes measure
different things. Compact output does not demonstrate within-arena reclamation,
and a structured counter does not predict arbitrary graph size.

At twenty bits per state, `X <= Y` contains 549,756,338,176 distinct endpoint
pairs. The XYZ count additionally includes the unused Z face. Neither number
is the number of ordinary rows visited by Free Join: there was one binding.

## Reproduce

```sh
python3 proposal/experiments/event-repr-lab/run.py --lane owned --candidate packed512 --layout bit-major --identity all --trials 7 --output my-diagonal-controls.json
python3 proposal/experiments/event-repr-lab/run.py --no-build --lane symbolic-relations --identity native --trials 5 --timeout 120 --output my-symbolic-relations.json
```

Build and time serially. The explicit table control may record a resource-cap
refusal in the large symbolic lane; it must not attempt to enumerate `2^60`.
