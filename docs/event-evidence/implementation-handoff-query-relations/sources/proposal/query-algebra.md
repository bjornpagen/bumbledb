# Querying the event algebra

**Revision 0.9; proposed query extension with partial native host support.** This specifies the query
surface for the [event carrier](event-surface.md). Ordinary joins bind rows;
explicit event expressions construct regions; `Pack` unions those regions;
explicit probability observations measure them. [Coup queries](coup/queries.rs)
and the [walkthrough](coup/query-walkthrough.md) exercise that base path.
Sections 8–11 extend it to constructive possibility reasoning, checked world
faces, information, and observables. [Advanced Coup queries](coup/algebra-queries.rs)
show the proposed additions together. The [native complete-binding query slice](../docs/event-queries.md)
now compiles Event/Test Boolean, ITE and fixed-roster cardinality heads, grouped
Event Pack, staged interiors and captured `use map` readouts. Pullback,
Image/UniversalImage/NonvacuousImage and Possible/Guaranteed construct Events
with checked input/output contexts. Typed face/product imports, relation operators,
modalities and finite Star are also implemented. Probability/Expectation and
general Least/Greatest query binders remain proposed; the complete Coup templates
are not yet executable.

## 1. Three head operations

```rust
region: Event(a & !(b | c))
covered: Pack(region)
chance: Probability(region, given)
```

`Event` constructs one event value per binding. `Pack` aggregates a bound event
column by union, grouping on the other, nonaggregate head values. `Probability`
observes the conditional probability of its first expression given its second.
It is a computed output, not an aggregate over database rows. The comma avoids
confusing conditioning with event union.

Event-valued `Pack` produces one event per group, however many disconnected
conditions its region contains. Today's interval `Pack` can emit several maximal
segments; the proposed event carrier is closed under arbitrary finite Boolean
combinations, so the whole union remains one composable value.

The base Event grammar is implemented on the branch; Probability remains proposed:

```text
headterm := existing headterm
          | name ':' 'Event' '(' eventexpr ')'
          | name ':' 'Probability' '(' eventexpr ',' eventexpr ')'

eventexpr := var | 'Empty' '(' var ')' | 'Full' '(' var ')'
           | '!' eventexpr | '(' eventexpr ')'
           | eventexpr '&' eventexpr
           | eventexpr '^' eventexpr
           | eventexpr '|' eventexpr
```

Precedence is Rust's: `!`, then `&`, then `^`, then `|`. All variables must be
bound event values in the body. `Empty(anchor)` and `Full(anchor)` use the
anchor's captured scope; they do not condition on the anchor. `Full(given)` is
the universe, even when `given` is a proper subset of it.

| Expression | Region |
| --- | --- |
| `!a` | Worlds outside `a`, relative to its captured universe |
| `a & b` | Worlds in both |
| `a \| b` | Worlds in either, including both |
| `a ^ b` | Worlds in exactly one |
| `a & !b` | Worlds in `a` but outside `b` |

All operands must belong to a compatible, explicitly aligned source context.
Scalar IDs help select the intended values, but do not prove scope compatibility.
Validation precedes rewrites: even `Empty(a) & b` rejects an incompatible `b`.
No product coupling or source renaming is invented to make an expression work.
Distinct draws can share unknown parameters without being the same event.

A head alias is an output column, not a new body binding. To consume a
computed event or `Pack` result, use an `interior` and read it in a later stage.
No nested `Pack`, implicit evaluation order between head terms, or event
computation in ordinary recursive feedback is introduced here. Finite structural
fixed points have a separate sealed-stage contract in §10.

## 2. Keep the Boolean algebra closed, including empty

**Query and intermediate event values include a scoped empty event.** Thus
`Event(a & !a)` emits a row containing empty, and complementing that value
produces full. This is a deliberate query contract. It differs from the current
interval `Segments` producer, where an empty construction drops the binding.
Borrowing that behavior would make event complement incomplete.

Stored event fields retain this same total algebra. A writer can insert a
query's empty value and later retrieve or complement it. An empty event, a
missing result row, and an evaluation failure are three different outcomes.
The [storage contract](event-storage.md) preserves that distinction and makes
explicit the nonemptiness premise needed for pointwise-key distinctness.
The [resident representation](representation.md) retains the owner/wire checks
even for constant results.

`Pack` alone does not produce a group for nonexistent input. Seed the desired
groups explicitly with empty, using ordinary multi-rule interiors:

```rust
let held_roles = bumbledb_query::query!(Coup {
    interior fragments(position, seat, role, region: Event(e)) |
        Influence(position, card, seat, revealed == false, when: e),
        Card(id: card, role);
    interior fragments(position, seat, role, region: Event(Empty(given))) |
        PositionSeat(position, seat),
        Position(id: position, perspective),
        Perspective(id: perspective, given),
        Role(id: role);
    (position, seat, role, holds: Pack(region)) |
        fragments(position, seat, role, region);
});
```

Both arms have the same computed head shape. The second arm supplies one empty
seed for every admitted position/participant/role. Union with that seed changes
no actual region. If all three Dukes are revealed, Bob's Duke group still exists
with empty; its complement is full. Missing positions and perspectives still
produce no group. This interpretation relies on complete card and slot coverage,
not a universal closed-world assumption about database absence.

The helper is unparameterized and nonrecursive, respecting today's query-import
restriction. A consuming query can import it and apply its own parameters.

## 3. Ordinary joins and event conjunction remain explicit

```rust
held(position, bob, captain, b),
held(position, cleo, captain, c)
```

The shared `position` and `captain` variables join ordinary values. `b` and `c`
retain distinct regions. `Event(b & c)` subsequently intersects them. Reusing
one variable in both event positions instead requires event-value equality.
Equal marginal probabilities do not satisfy that equality.

`!Relation(...)` remains a relational anti-probe: no matching row.
`Event(!a)` constructs a complement within worlds. A holding row can exist
because Bob has a Duke in some worlds, while `!holds` is true in many other
worlds. Relational anti-probing cannot calculate that event.

Joining a `Claim` row does not condition a probability. Its correctly bound
observation event must occur in `given`, or be explicitly intersected into the
second argument of `Probability`, to include that declaration as evidence.

## 4. Lowering into the existing stage architecture

| Source | Existing responsibility | Proposed addition |
| --- | --- | --- |
| [`query!` grammar](../crates/bumbledb-query-macros/src/lib.rs) | Bindings, interiors, imports, aggregate heads | Parse implemented Event/Test heads; add probability heads |
| [`ir.rs`](../crates/bumbledb/src/ir.rs), `FindTerm` / `Interior` | Pure-data outputs; topologically ordered stages | Event-expression and probability-observation terms |
| [`build.rs`](../crates/bumbledb/src/api/prepared/build.rs), `find_specs` | Bind expression variables to typed slots | Bind event operands and retain scope ownership |
| [`reach.rs`](../crates/bumbledb/src/api/prepared/reach.rs), `run_free_join_into_projection` | Execute Free Join into its sink | Carry events as typed bindings |
| [`computed.rs`](../crates/bumbledb/src/api/prepared/computed.rs), `OutputProgram` / `ComputedSink` | Evaluate fully bound outputs before projection or aggregation | Boolean event construction and explicit measurement |
| [`sink.rs`](../crates/bumbledb/src/exec/sink.rs), `AggregateSink` | Group full bindings and evaluate `Pack` | Event union as the event-domain packing operation |

The [cookbook's staged interval queries](../docs/cookbook.md) already separate
packing, measuring, and summing. General computed expression trees have a
structural Rust IR today. Event/Test and Pack forms compile on this branch;
probability, relation-import and expectation forms retain their separate gates.
The complete-binding adapter now resolves owned Event keys, constructs two-word
outputs and retains complete semantic context-fault sets through stage sealing.
Event Pack now retains exact participating claims through scratch and seals grouped
unions under canonical context minima. Exact probability results remain pending.

The native IR is `FindTerm::Event(EventExpr)` and `FindTerm::Test(EventTest)`.
`EventExpr` carries bound variables/anchors, complement, arbitrary `BoolOp4`
application, ITE and inclusive cardinality over a nonempty expression roster.
`EventTest` carries the six structural predicates in §8. These are inspectable
pure-data trees; shape admission precedes normalization. The Event domain of
existing `Pack { over: VarId }` is now implemented. `Probability` and full
relation/face-import programs remain future additions.

The Coup steal query becomes:

```text
Influence ⋈ Card                    ordinary matching of card IDs
    → fragments + explicit seeds    event regions, including empty
    → Pack by position/seat/role    one union event per group
    → held ⋈ held ⋈ held ⋈ view     bind B, C, A, and evidence G
    → Event(B & !(C | A))           construct one composite event E
    → Probability(E, G)            measure E under the retained joint law
```

`FreeJoinPlan` continues to choose covers, probes, residuals, and anti-probes.
It does not multiply row probabilities, choose independence assumptions, or
enumerate possible worlds as part of ordinary matching. The event evaluator
can use an exact symbolic representation; the finite-world checker merely
supplies reference semantics. Revision 0.8 starts with the
[canonical completed-function carrier](representation.md), using symbolic splits
and essential-coordinate tables. [Source-law.md](source-law.md) specifies the
joint-law contraction; [kernels.md](kernels.md) maps Boolean programs to ARM64.

Interiors make the dependency order explicit. A later join can consume the
constructed event; a later `Pack` can union results from many matching paths.
The scope and source owners must survive staging and result lifetimes. Errors
must prevent publishing partial answers, following the existing computed-stage
failure discipline.

Event validation failures follow the [current deterministic fault-set contract](proposal.md):
all participating operands are validated, including after saturation; unmatched
operands remain unevaluated. Stable logical descriptors determine diagnostic
presentation, not physical join order or arena IDs. The lab proves/tests unordered
fault-set equality; native Event/Test heads now own canonical, sorted fault descriptors
with stage, written-rule and syntactic operand identity. The native tests include
DNF collapse, duplicate rules, traversal order, saturation and unmatched values.
Resource refusal cannot masquerade as a complete set. Extending these guarantees
to factored schedules remains required. Captured map and relation boundaries
now supply exact input/output contexts; repeated variables retain separate
expected contexts at each written occurrence. Pack now uses the
lexicographically least canonical full-space descriptor of each group as its
expected context, retains written claims before idempotent union, and reports
all mismatches at sealing. A binding with an invalid computed group key has
head faults and no defined Pack group; valid bindings still participate after
earlier faults. Empty seeds establish presence without skipping context checks.

## 5. The math that gets lowered

Write `iA(w)` for A's indicator: one when A holds in world w, zero otherwise.

```text
i(!A)    = 1 - iA
i(A & B) = iA * iB
i(A | B) = iA + iB - iA*iB
i(A ^ B) = iA + iB - 2*iA*iB

i(B & !(C | A)) = iB * (1-iC) * (1-iA)
```

These multiply indicators **in the same world**, which is valid with arbitrary
dependence. Multiplying already-measured marginals requires independence.
For a finite presentation with source assignment theta:

```text
mass_theta(E) = Σw mu_theta(w) * iE(theta,w)
P_theta(E | G) = mass_theta(E & G) / mass_theta(G)
```

The sums range over named finite outcomes; theta has no invented prior. Shared
parameters and outcome coordinates survive every expression. The retained
source algebra supplies exact finite-sum and semialgebraic reasoning; simple
fixed-event cases give polynomial masses and rational conditional probabilities.
More general event membership may require piecewise expressions.

`Probability(E,G)` preserves the following semantic result:

- One fixed law, positive evidence: exact ratio and evidence mass.
- An unmeasured structural space: `MissingLaw`, never an invented uniform law.
- A constrained family: conditional function on precisely the positive-evidence
  parameter domain, plus the original evidence-mass function. Reported ranges
  must identify partial definedness and endpoint attainability.
- No source gives G positive mass: an explicit impossible-observation result,
  not zero, certainty, or a missing row. An inconsistent source domain or an
  incompatible scope is an error, not an impossible observation.

An interval hull is a report derived from that result, not a replacement for
the underlying source relationships. [Source-law.md](source-law.md) specifies
the owned observation shape; its function/domain encoding and native value
integration belong to the storage and measurement implementation steps.
This extension introduces neither automatic priors nor a probability solver
already present in Free Join.

## 6. Rewrites with useful consequences

After scope validation, ordinary Boolean laws apply exactly:

```text
A & A           = A             repeated evidence is one observation
A | A           = A             duplicate derivations add no probability
A & !A          = Empty(A)      contradiction is a representable event
A | !A          = Full(A)       excluded middle has probability one
A | (A & B)     = A             a more restrictive duplicate adds nothing
!(A | B)        = !A & !B       neither requires both negations
A & (B | C)     = (A & B) | (A & C)
```

Distributivity permits intersecting each group member with a group-constant
event before packing, or intersecting the packed result afterwards. Complement
has a different rule: `!(union Ei) = intersection !Ei`. **`Pack(!Ei)` generally
does not compute `!Pack(Ei)`.** This matters when a player can hold two Dukes.

Admitted pointwise keys provide further disjointness facts. If two predicates
place the same physical card in different locations, their intersection is
empty. Mirrors supply coverage identities. An optimizer can use these when it
can identify the relevant schema proof; today's optimizer does not already
perform these event rewrites.

Existing full-binding deduplication does not replace event idempotence. Two
different cards or graph paths can yield distinct bindings whose events overlap.
Pack must count their shared worlds once. Summing individual probabilities
answers a different question.

The [dependency experiment](experiments/event-repr-lab/DEPENDENCIES.md) makes
another distinction concrete. Admission retains both covered worlds and worlds
owned by multiple distinct facts. Pack needs only coverage; a pointwise key
needs the conflict region as well. Counting duplicate query derivations as
distinct facts would manufacture conflicts. Both summaries use the same Event
operators, with different contributor-identity contracts.

The [finite count experiment](experiments/event-repr-lab/FACTOR-COUNTS.md) exploits
another dependency: if an Event's membership is determined by a retained
readout, count that readout with its exact number of legal completions. This is
a partition of the original worlds. On a certified product of complete legal
states, ignored state faces contribute their domain cardinality; on coupled
support the number of completions may vary with the retained value. The latter
requires a more general contraction, not multiplication by marginal sizes.
The implementation prepares the product factors once and observes the same
resident Event without mutation. This experiment concerns finite cardinality;
a probability observation must contract the captured joint law instead.

An observation can also determine **membership**, rather than a count. For a
readout f, `Possible_f(A)` is the least Event observable through f containing A;
`Guaranteed_f(A)` is the greatest such Event contained in A. The
[information-algebra proofs](experiments/event-repr-lab/INFORMATION-ALGEBRA.md)
connect their rewrites directly to dependencies. A fixed filter B can move
through `Possible_f(A & B)` for every A exactly when `f -> membership_B`.
Possibility distributes over Pack's grouped union, but it cannot split a
conjunction into separate witnesses. Guaranteed distributes over intersection;
moving it through an arbitrary grouped union changes the answer. The current
native laboratory constructs all these output Events after grouping; planner
pushdown remains implementation work.

For partial decisions, all non-Tax alternatives cover `occurs & !tax`.
They cover `!tax` only when `occurs` is full. Complement is always relative to
the captured universe; a subset does not become its own universe.

## 7. What is checked

[`coup/query-checks.py`](coup/query-checks.py) compares relational matching,
explicit group completion, packing, and event expressions against predicates
evaluated directly on complete Coup worlds. It covers the compound steal query,
Tax posterior, proof/replacement, XOR, empty groups, duplicate derivations,
anti-join/complement and complement/Pack counterexamples, scope refusal, and
impossible evidence. [Saved output](coup/research/query-checks.json).

This is a finite semantic oracle, not execution of `query!` or Free Join. The
[native ledger](../docs/event-implementation.md) supplies separate evidence for
implemented typing, equality, persistence, admission and query slices. Those
tests now include typed relation imports, composition, residuals, modalities and
finite closure; they do not qualify measured query forms or arbitrary fixed-point binders.

## 8. Structural tests, branching, and cardinality

Extend the base head grammar with `name: Test(eventtest)`, and the event grammar
with ITE and fixed-roster cardinality:

```text
eventtest := IsEmpty(eventexpr) | IsFull(eventexpr)
           | Subset(eventexpr,eventexpr) | Equal(eventexpr,eventexpr)
           | Disjoint(eventexpr,eventexpr) | Covers(eventexpr,eventexpr)
eventexpr += Ite(eventexpr,eventexpr,eventexpr)
           | AtLeast(integer,eventexpr,...) | AtMost(integer,eventexpr,...)
           | Exactly(integer,eventexpr,...)
```

Thresholds are nonnegative literals; at least one event supplies the scope.
Dynamic cardinality uses a checked roster/partition operation at the library
boundary. The roster has ordinary item identities: duplicate derivations of
one card do not count twice, while different cards with equal EventKeys do.

`Test` produces a bool about whole regions. `Event(!a | b)` produces the region
where implication holds; `Test(Subset(a,b))` establishes that it holds everywhere.
`Test(IsEmpty(a & b))` can filter a later interior, where the result bool is now
bound. It cannot filter its own head binding. This preserves the stage/error
discipline of other computed terms. Every operand is validated before evaluation.

Information abstraction can therefore be written with existing cell relations:
join `PolicyCase`, compute a counterexample region and its emptiness test, filter
the bool in a later stage, and Pack qualifying whole cases. Include a seed for
each intended output group. Pack unions the **cells**, not only the portion of
the target event inside them. Evidence nonvacuity is a separate test.

### Captured readout queries (implemented structural slice)

The [native query API](../docs/event-queries.md) retains an `EventImport` built
from a checked map or surjective-map BEDC descriptor:

```rust
use map observed = &observation;
possible: Event(Possible(a, observed))
certain: Event(Guaranteed(a, observed))
visible: Event(Image(a, observed))
back: Event(Pullback(b, observed))
```

These are fragments inside a schema-bound `query!`. `UniversalImage` and
`NonvacuousImage` use the same `(event, imported_map)` order. For f: S→T, the
images consume S and produce T; pullback consumes T and produces S; possibility
and guarantee stay on S. Sibling Boolean terms must share the output context,
while each map input is checked against its own required context. A variable
does not silently change its source identity when reused outside a map.

The pure-data IR retains owned descriptor bytes and checked operations. Static
incompatible contexts refuse even with no body results; dynamic operand faults
retain every syntactic occurrence, including empty/full anchors and ignored
branches. QueryMaps' scope-indexed Lean reference proves demand preservation and
whole-program congruence when demanded inputs are retained. Native lowering,
marshalling and owner alignment remain tested correspondence obligations.

## 9. World relations are typed views of bound Events

The native [BEDC descriptor layer](../docs/event-descriptors.md) now supplies
portable finite maps, pair roles and shared-workspace plans with checked import.
Its plain data is separate from admitted owners. The implementation macro imports
those owned face descriptors, without adding another
database field type or running an arbitrary host callback:

```rust
use faces step = &step_faces;
// r is a bound Event on the product described by step.
safe: Event(Must(Relation(r, step), goal))
possible: Event(May(Relation(r, step), goal))
```

`step_faces` is a validated descriptor with input, output, product presentation,
shared environment, and coordinate maps. It is built at the source/library
boundary. This **implemented macro import form** lowers to a retained descriptor binding.
It is not today's ordinary query `use`, nor a claim that scalar IDs establish
scope compatibility. The prepared plan checks its shape; each bound region
is checked against its expected space before execution.

The implemented finite typed expression grammar adds:

```text
relationexpr := Relation(eventexpr, faces)
              | Id(endofaces) | TestRelation(eventexpr,endofaces)
              | !relationexpr | (relationexpr)
              | relationexpr '&' relationexpr | relationexpr '|' relationexpr
              | Converse(relationexpr) | Compose(relationexpr,relationexpr,product)
              | LeftResidual(relationexpr,relationexpr,product)
              | RightResidual(relationexpr,relationexpr,product) | Star(relationexpr,product)
eventexpr += Region(relationexpr) | Domain(relationexpr) | Range(relationexpr)
           | May(relationexpr,eventexpr) | All(relationexpr,eventexpr)
           | Must(relationexpr,eventexpr) | Post(relationexpr,eventexpr)
```

Relation operators have their own checked input/output faces; there
is no implicit cast from a unary event to an action. An `endofaces` descriptor
has matching state presentations, with separate before/after coordinate roles.
`Region` exposes the result as an ordinary Event on its product space, allowing
it to be packed or stored. Reapply `Relation` with compatible faces to consume
that stored region later. Face metadata is retained by the space but is never
inferred from the numeric probability of the row.

`use product path = &composition` retains an `EventImport` whose BEDC Composition
descriptor supplies the authored ST, TU, SU products and STU workspace. This
third argument is mandatory for composition, residuals and closure. Endpoint
identities alone cannot determine the named result product or workspace.

For example, `Region(Compose(Relation(r, first), Relation(q, second), path))`
constructs a result region on the checked outer product. Middle worlds are
existentially eliminated; shared source parameters are retained. Product
construction supplies structural admissibility, and remains unmeasured unless
an explicit law constructor supplies a justified joint law.

The native pure-data IR now contains `EventExpr`, `RelationExpr`, `EventTest`,
and retained `EventImport`s. Relation trees expose child operators and descriptor
data; shape admission checks their signatures against actual environment readouts.
Composition retains a `RelationalProduct` with named products and workspace.
Binding slots carry EventKeys and operation owners. Library authors can
construct the same IR directly, inspect its maps, or consume a borrowed diagram
view. The macro is a surface for this algebra, not its only access path.

## 10. Finite fixed points have a sealed evaluation stage

`Star(relation, product)` validates an endorelation and its finite stable-state certificate. It
iterates a monotone closure program with canonical equality until stabilization,
with cancellation/resource failure aborting publication. It does not use the
ordinary recursive-rule scheduler or call TypeSafe while iterating.

The owned library/IR also accepts positive predicate fixed-point programs:
`Least(X, E | May(R,X))`, `Least(X, E | Must(R,X))`, and analogous greatest
fixed points. The binder, variable variance, finite carrier, and parameter
environment are checked before execution. These binder forms are **IR notation**;
this revision does not invent an additional Rust macro recursion syntax for them.
Their Event results can be bound into ordinary query stages.

The [native host implementation](../docs/event-fixed-points.md) now exposes
`EventProgramBuilder`, inspectable typed `EventProgram` instructions and
`FixedPointProgram::least`/`greatest`. All sixteen truth functions, ITE, checked
maps and fixed-relation modalities participate in a conservative variance
analysis; a private builder identity protects operand indices. `finish` prunes
only unused nodes. Referenced operands still execute under constant truth
functions, so simplification cannot erase their participation. The [native query adapter](../docs/event-queries.md) now supports multiple body
bindings for constructive Event/Test trees, captured readout boundaries and stable
per-occurrence context faults.
Typed relation trees and captured face/product imports now build on this host
layer, including exact finite Star. General Least/Greatest query binders retain
their separate M5/M7 integration obligation.

The bounded reference is now [checked in Lean](../crates/bumbledb-event/semantics/FixedPoint.lean):
iterate from empty N times for an exhaustive N-entry legal roster, then apply
once more to test equality; top iteration is its complement dual. N is a semantic
upper bound, not a required materialized roster or a promise about cost. Native
execution may stop earlier at canonical equality or explicitly refuse exhausted
resources. A diagram's node count is not a legal-state bound. The native compiler
must certify that its carrier and monotone operator refine this reference.
Native finite spaces now supply a support-count carrier and positive variance
gate. Detection includes at most N+1 applications and has separate cumulative
instruction and application budgets. The carrier's original support count and
canonical equality are explicit Rust correspondence obligations; the current
certificate does not certify continuous source presentations.

Finite quantification over action faces builds controllable predecessors.
The planner must preserve alternation: a uniform action precedes universal
hidden-world/outcome checks. Fixed-point safety and reachability need the
explicit arena contracts in [world-relations.md](world-relations.md).
An ordinary projected result column, a bound source parameter, and an internally
quantified state coordinate are three different kinds of variable.

The [native action host API](../docs/event-actions.md) now builds these controlled
predecessor programs and retains first-entry ranks plus progress/safety policies.
Uniform one-step permissions use the inhabited residual construction. Those
results are ordinary Event regions usable in bindings/storage; arena, role and
strategy descriptor transport and query-head compilation remain unimplemented.
Fully observed strategies require an adequate information/memory state before
they can model repeated partially observed play.

## 11. Exact expectation and Free Join execution

`name: Expectation(value, when, given)` is a proposed aggregate head. The three
arguments are body-bound columns. Its inputs are an ordinary exact numeric value
and two Event values. The remaining nonaggregate head values are group keys.
Within each group, union equal-value regions first, require one aligned `given`,
then establish disjointness of different values and coverage on that evidence.
Intersect with evidence before measurement. Overlap outside evidence is harmless;
missing value coverage on evidence is an error, even if that region has zero
mass. Validate the partition before contraction. Zero-mass evidence returns
Impossible for an otherwise valid observable; this need not mean structural emptiness.
An empty input has no group; explicit parent seeds/complete value relations are
required when a group must exist. A missing numerical value is never zero.

The [native partition helper](../docs/event-partitions.md) now supplies the
structural evidence-relative admission and value regrouping contract. It checks
contexts before clipping, retains empty indexed buckets and requires complete
coverage. The caller still supplies scalar equality/value mapping; constructive
query heads, group extraction and exact-law contraction remain unimplemented.

Probability and expectation share exact joint-law contraction. Expectation has
an owned signed exact result and a retained positive-evidence domain; it cannot
reuse a probability interval or silently reduce to binary64. Initial numeric
columns can be signed integers; exact rational/function values require their
own supported query slots. Score levels acquire utility only through an explicit
ordinary value mapping, never from the provider's confidence field.

Free Join selects and matches ordinary rows. Computed stages operate on their
Event values, and aggregate stages union or measure the represented regions.
Inside a relational product, BDD elimination handles world coordinates. These
two elimination layers can share a planning language and factorization laws,
but have different carriers, costs, and variable roles.

The initial ComputedSink path requires complete bindings: suffix skipping and
fused leaf scans remain forbidden. One row witness does not establish the union
of all Event witnesses. Early termination at full requires both a coverage proof
and assurance that skipped inputs cannot hide validation/evaluation errors.
Likewise, an empty intermediate Event permits pruning only when every remaining
completion stays empty and output-group existence is preserved. `Event(empty)`
still emits a row; dropping its last witness can otherwise change a present
empty group into an absent group. Explicit seeds or a separate group-existence
proof are required before using that optimization.
Mixed existential, universal, sum, min and max stages retain their order unless
a specific distributivity/independence theorem licenses a rewrite. This is the
useful connection to FAQ, not a claim that native Free Join already evaluates
arbitrary semirings or stochastic programs.

A concrete rewrite is now [proved and executed in the lab](experiments/event-repr-lab/FACTORIZED-PACK.md):
when a group's scalar binding relation is a Cartesian product of branch rows,
its union of row-local Event products can be computed from branch unions. The
condition is necessary and sufficient for arbitrary Event factors. Group-row
presence remains separate from Event possibility, and the same world witness
remains shared. Native execution uses three branch Free Join plans and one
summary join, with summary construction charged to the query. All 5,376
differential runs pass. This is an isolated implementation for a constructed
clover query; the production complete-binding sink has not acquired a general
skipping capability. Several small warm queries lose to staging, so the algebra
licenses a costed choice rather than an unconditional rewrite.

Validation follows participation: every row with joined companions must be
checked, even if its Event fold has saturated. Unmatched invalid rows remain
unevaluated. The experiment preserves unordered fault sets; preserving the first
error of a particular traversal would require a different contract or proof.

The same row-elimination principle has a relational form:
`Pack(i,j, Compose(R[i],S[j])) = Compose(Pack(i,R[i]),Pack(j,S[j]))`
when the row indices have the required Cartesian binding certificate. The
intermediate world remains shared. A union in the antecedent of LeftResidual
instead produces an intersection of obligations. These typed relational laws
have separate Lean proofs and now run in an isolated
[native relational Pack experiment](experiments/event-repr-lab/RELATIONAL-PACK.md).
The residual program uses an explicit intersection aggregate over row-pair
obligations; substituting the ordinary union Pack would ask a different question.
A conservative checker validates the supplied branch partition against the
actual normalized query. General partition discovery, mixed-operator planning
and production lowering remain future work. These programs do not inherit FAQ's
arbitrary commutative factor reorderings.
