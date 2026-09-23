# Event in the query engine

This is a proposed extension to BumbleDB's existing IR, Free Join executor,
acyclic stages and aggregate sink. Event is neither a hidden row annotation
nor an embedded program. Its operators consume and produce normal values.

## A total classifier that the planner may move

For compatible universes, classify occupancy of four cells:

| Signature bit | Cell |
| --- | --- |
| 0 | Neither: `U \ (A ∪ B)` |
| 1 | B only: `B \ A` |
| 2 | A only: `A \ B` |
| 3 | Both: `A ∩ B` |

An inhabited universe gives a signature from 1 through 15. For different
universe descriptors, return a separate `DifferentUniverse` class. Encode
that class as classifier code 0, and the same-universe signatures as codes
1–15. The occupancy signature itself never admits zero. This produces sixteen
mutually exclusive, exhaustive cases over the whole Event value type.

An `EventMask(u16)` accepts a set of these cases, using bit `code`. Mask union,
intersection and complement are ordinary bit operations; complement is relative
to `0xffff`. Swapping operands swaps the A-only/B-only signature bits and leaves
DifferentUniverse fixed. Complementing an operand permutes occupancy cells
within a universe and also leaves DifferentUniverse fixed.

| Named mask | Accepted cases |
| --- | --- |
| `INTERSECTS` | Compatible signatures with Both occupied |
| `DISJOINT` | Compatible signatures with Both absent |
| `SUBSET` | Compatible signatures with A-only absent |
| `SUPERSET` | Compatible signatures with B-only absent |
| `EQUALS` | Signatures 1, 8 and 9: both empty, both full, equal proper sets |
| `COVERS_UNIVERSE` | Compatible signatures with Neither absent |
| `SAME_UNIVERSE` | Codes 1–15 |
| `DIFFERENT_UNIVERSE` | Code 0 |

Consequently `!INTERSECTS` includes different universes whereas `DISJOINT`
does not. `!EQUALS` agrees with total value inequality, including foreign scopes.
This is intentional: negating a predicate is different from asking a structural
question that requires a common universe. Full/empty masks exist algebraically;
follow Allen's query rule rejecting those two trivial predicate masks.

Proposed macro syntax follows Allen:

```rust
EventRel(a, INTERSECTS, b)
EventRel(a, SUBSET, ?evidence)
```

Masks are literals, not parameters. Start with named constants and the existing
literal-mask union grammar. The structural API supports arbitrary checked masks.
A syntactically valid Event-pair filter never raises a universe mismatch: it
classifies it. Type errors, malformed input, cancellation and allocation errors
remain errors at their existing boundaries.

This resolves a database problem in the previous draft. Consider a foreign
candidate at an early join node which a later scalar atom would discard. If
classification throws, moving the predicate changes whether the query fails.
A total classifier lets the planner reject or retain candidates without adding
an evaluation-order-dependent data error. The database need not collect faults
from every rejected Cartesian pair to preserve semantics.

Within a common universe, the signature answers possibility of any binary
Boolean construction without allocating its result. It cannot replace the
Event itself, count worlds, or establish a shared witness for three conditions.
`{1,2}`, `{2,3}`, `{1,3}` overlap pairwise but have empty triple intersection.
The historical relation-composition table is only a sound envelope on finite
worlds; no table-driven inference engine is included here.

## Native placement, equality and parameters

Add one comparison IR case and corresponding constant/column and column/column
predicates. Normalize Event equality and inequality consistently with the total
classifier; variable equality still binds the same complete value through normal
Free Join keys. Bind a shared variable to request equal regions; use EventRel
to request structural overlap or inclusion.

Place residual predicates at the earliest node where both operands bind, using
[the same planner mechanism as Allen](../crates/bumbledb/src/plan/fj/validate.rs).
Push constant-side filters into column scans where lawful. Run them in native
batch execution before later joins amplify the candidate set. Plan tests must
inspect placement, and execution tests must count later visits to demonstrate
that this path is exercised. A late computed head is not equivalent integration.

Events also work as bound literals, scalar parameters, equality-set parameters,
projected fields, group keys and exact anti-join bindings. Validate incoming
parameter shape before execution, then resolve values into the execution's
generation. An absent equality token may use the existing miss optimization;
a structural predicate needs the actual parameter region even when no stored
row equals it. Parameter-set membership means equality to supplied Event values,
not membership of worlds in their union.

Extend existing `PointIn` for `u64 in event`: the integer is an index in that
operand's universe. Out-of-range indices return false; no modulo or coercion.
Queries spanning games must still bind the game/scalar context. The index is
not a globally named world. This is direct bitmap membership, not enumeration.

Ordinary negated atoms continue to mean no matching fact. Complementing an
Event constructs `U \ A`; those operations are not interchangeable. Empty
facts still match equality, affect counts and participate in negation.

## Small, explicit value expressions

Add a closed `EventExpr` family for Event leaves, complement, intersection,
union, difference, and `Empty(anchor)` / `Full(anchor)`. Leaves are bound Event
variables, parameters or literals, never set-valued parameters or callbacks.
Anchors establish the universe without new context-inference syntax. Internally
a four-bit Boolean truth table can implement binary kernels; it is not a
predicate mask or an occupancy signature.

Proposed head syntax:

```rust
interior narrowed(id, when: Event(a & !b)) | Claim(id, when: a, blocked: b);
(id, when) | narrowed(id, when), EventRel(when, INTERSECTS, when);
```

Use ordinary `FindTerm` computed-output plumbing, with Event result typing.
`!`, `&`, `|`, and difference `-` lower to this finite expression family; allow
parentheses. Set precedence explicitly (`!`, then `&`/`-` left-associative, then
`|`) and test it. No arbitrary Rust execution, recursive expression binding,
quantifier, program value or solver call is admitted. Check a depth limit of
64 iteratively before recursive walks, following existing query validation.

Construction is evaluated for each complete surviving body binding. Validate
all referenced operand descriptors before simplifying, so `Empty(a) & b` cannot
hide a mismatch. A body with no bindings performs no per-binding construction;
malformed literal/parameter bytes still fail their input validation. Computed
heads preserve empty results. A later stage can filter, join or Pack them.
EventRel operands themselves remain ordinary terms; stage a computed operand.

Do not move these partial constructors across filters, stages or aggregates
without preserving their evaluation/failure domain. A downstream filter cannot
hide an earlier stage's failure. Error presentation uses a typed operation/site
and descriptors; it does not promise a traversal-independent enumeration of all
bad bindings. As with other operational failures, no partial result seals.

Projection-only recursion may carry an already supplied Event unchanged through
the existing `RecStep` rules. Computed Event heads and Pack do not become legal
recursive heads merely because they are useful outside recursion.

## Pack and result ownership

Overload the existing Pack by its input type. Interval Pack continues emitting
maximal nonempty segments. Event Pack emits one union Event per present scalar
group, including an all-empty group. No rows means no group, even with no group
columns. An explicit empty seed supplies the “known impossible” answer when a
caller needs a complete roster. Empty/Full anchors make such seeds ordinary
staged values.

Keep current restrictions: one Pack per head; no mixing it with numeric folds
in that head. First stage a computed Event, then Pack its bound variable. An
Event-valued group key uses whole-value equality. A group cannot combine foreign
universes; check every participating descriptor even after its union is full.
Repeated equivalent claims do not add multiplicity to a union.

Fold into a mutable word accumulator per group, then canonicalize/intern the
finished union once. Do not intern a fresh immutable bitmap for every incoming
claim. If group machinery spills, serialize the union's canonical value in the
existing scratch map and union partial accumulators when a group reappears.
Preserve group presence separately from nonempty membership. The exact same
value must emerge from resident and forced-spill paths.

Errors are sticky until stage finalization, as in the existing aggregate sink.
No failed group can publish a successful prefix. Tests must include wide group
keys and a foreign final claim after a full accumulator. A token-only spill
that retains every bitmap in an owner table is not evidence of memory relief.
The [storage contract](storage.md) specifies cache/result lifetimes.

## Index and kernel work after correctness

The first native path is exact classification over candidates already selected
by scalar Free Join keys. It needs no new persistent index. A predicate may
short-circuit once its answer is established; the full classifier may stop once
all four cells have witnesses. DifferentUniverse is a descriptor comparison.
Tail masking and constant polarity must be correct in scalar and SIMD paths.

For repeated selective probes, an optional snapshot/group-scoped directory can
store a union per candidate block. A disjoint block union proves that no row in
that compatible block intersects the query Event. Otherwise descend or scan
and check the exact mask. Partition or summarize descriptors separately: a mask
including DifferentUniverse must retain foreign candidates. The overlap skip
rule cannot discard blocks for DISJOINT. There is no interval-like logarithmic
search guarantee for arbitrary sets; mixed block unions may saturate to full.

Keep directory build cost, first query, cache reuse, updates and retained payloads
in measurements. Disable a losing directory. Do not add a permanent index based
on a warm query that excluded its construction cost.

ARM64/NEON can load words, apply Boolean operations and reduce occupancy flags;
constant-side batching can share decoding. Benchmark complete executor paths,
not just word loops. A scalar reference and architecture fallback remain required.
No alignment, page-tail or vector-width assumption may read past payload bounds.

A later, dependency-proved rewrite may use:

```text
union over i,j of (A[i] ∩ B[j]) = (union over i A[i]) ∩ (union over j B[j])
```

Only when the scalar join admits that Cartesian factorization. Hidden shared
keys, pair filters, empty-group presence and constructor failures can invalidate
an application of the law. Do not rewrite first and infer its premises later.

## Separate query proposal: Event-labelled reachability

No Event membership or pair operation requires fixed-point iterations. When an
application actually has a graph, store ordinary `Edge(from, to, when)` rows.
Require a finite endpoint roster, all endpoints in it, and one universe. Missing
matrix entries mean empty for this graph operation; duplicate edge alternatives
are unioned. At each world, edges form an ordinary directed graph.

Composition joins the middle endpoint and Packs intersections:

```text
(R ∘ S)[x,z] = union over y of (R[x,y] ∩ S[y,z])
I[x,x] = full, with empty elsewhere
C0 = I ∪ R
C(k+1) = Ck ∪ (Ck ∘ Ck)
```

This is reflexive transitive closure. Each round doubles supported path length;
a simple path between distinct vertices needs at most n−1 edges. For n>0,
`ceil(log2(max(1,n−1)))` expansion rounds suffice, optionally followed by an
exact stabilization check. An empty roster returns no rows. This is logarithmic
round count, not total work: closure can produce n² pairs. Do not enumerate worlds
to run the engine schedule; the world-by-world graph is the independent oracle.

The current reach driver uses one positive self-atom and projection-only heads.
It cannot execute this two-self-input aggregated recurrence. Implementing closure
would require a separately reviewed dedicated schedule over joins/aggregates,
not relaxing the general recursion grammar. An application can orchestrate the
same recurrence using ordinary materialized stage results before that exists.

The closure output is explicitly sparse (nonempty pairs); a roster can seed zero
answers if wanted. Rooted reachability may deserve a cheaper plan than all-pairs
closure; the historical literature warns against computing a large closure to
answer a small question. Native closure, rooted variants, dynamic maintenance
and strategy synthesis are not gates for shipping Event.
