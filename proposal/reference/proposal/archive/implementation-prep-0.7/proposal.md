# Event: a representation for an algebra of possibility

**Design proposal: revision 0.5 algebra, revision 0.7 representation experiments.** The public field is `event`; the owned Rust
value is `Event`. The target is an algebra of admissible worlds and relations
between them: composition, converse, domain, possibility, necessity, residuals,
and finite closure. Boolean combinations are its unary predicate fragment.
The closest established program-algebra lineage is **Kleene algebra with domain**
in the relational model, augmented with the typed relational structure below.

The representation is a scoped handle to a canonical two-way partition of
admissible worlds. The [Rust experiments](experiments/event-repr-lab/REPORT.md)
reopen the physical representation: finite complemented sets, an
[anchored BDD](experiments/event-repr-lab/ANCHOR.md), and
[diagrams with packed truth-table terminals](experiments/event-repr-lab/PACKED.md)
establish exact identity and one-bit relative complement in different ways.
Relations are regions on checked product faces of that same representation. Probability laws are optional designated interpretations beside
the region; they are not row weights or a prerequisite for structural reasoning.

This is an engineering proposal with executable representation and instruction
probes under `proposal/`, plus a disposable Rust laboratory using real Free Join.
No production Event feature is implemented.

The [current design review](design-review.md) summarizes the representation
experiments and their conclusions. The [storage-closure revision](event-storage.md)
keeps empty Event values through persistence and records the extra premise
needed before inheriting Allen's complete-key planner witness.

The decision order is **native dependency-language synergy, algebraic elegance,
then performance**. A candidate must preserve pointwise fact identity, exact
target-key coverage, the full relational operators, and explicit source
dependence before its timings can win. The objective is one mathematical value
participating throughout the database. A fast formula evaluator attached to a
join sink does not establish that integration. [The native dependency review](experiments/event-repr-lab/DEPENDENCIES.md)
traces the current enforcement and planner boundaries.

## 0. What the representation must make possible

An Event is a condition your program can manipulate before learning whether it
holds. TypeSafe can contribute a focused judgment while the database retains its
alternatives and follows their consequences. An uncertain target can imply a
certain expenditure; one observed move can change beliefs about another player;
a relation can reveal the conditions under which a plan is safe.

The defining interfaces are specified in [world relations](world-relations.md),
[the operator catalog](event-algebra.md), and [the TypeSafe bridge](typesafe-inference.md).
They are required algebraic interfaces, not optional callbacks to a later solver.

| Capability | Constructive answer |
| --- | --- |
| Domain / May | The states from which a relation has a desired continuation |
| All / Must | Weakest liberal preconditions, plus an explicit nonvacuous form |
| Composition / converse | Join intermediate worlds, eliminate them, or reason in the reverse direction |
| Residuals | The largest continuation relation satisfying a containment constraint |
| Information abstraction | What remains possible or guaranteed when only certain observations are available |
| Finite closure | Reachability and validated monotone fixed points on a sealed finite state presentation |
| Observable partitions | Uncertain ordinary values represented through the existing relational idiom |
| Exact law interpretation | Probability, expectation, posterior revision, and sensitivity with retained dependencies |

**A substantive change from 0.4:** admissibility is explicit. A model assigning
zero probability to a legal action does not make that action structurally
impossible. A structural product needs no invented independent probability law.
Measured contexts keep one designated normalized law/family; unmeasured contexts
refuse probability observations. Explicit supported views recover the old
positive-support semantics when desired. The canonical pair invariant survives.

## 1. Put the mathematical distinction into the data

```rust
#[repr(C)]
struct EventKey {             // 16 bytes in resident rows and query bindings
    space: u64,              // registry identity of a sealed, owned world space
    region: u64,             // canonical representative plus selected polarity
}
```

The manager owns nonempty support `S`, semantic coordinate identities, a fixed
canonical representation, and the storage/caches needed to operate on regions.
The row carries a handle. Complementary events share a representative and differ
in the selected-side bit; empty and full are real values in that space.

There are now several concrete implementations of this invariant. Finite sets
can intern a canonical complementary subset. The anchored diagram stores the
side excluding a fixed admissible world, plus polarity. A packed diagram uses
that same invariant with packed truth tables at its leaves. The
earlier two-root BDD pair retains both support-masked sides explicitly.
A further [completed-function candidate](experiments/event-repr-lab/FIBRE-RETRACTION.md)
fixes a decoder onto legal worlds and interns `A(decoded_world)`. Its normal
form is a membership FD. Per-face and prefix-preserving decoders can make
logical independence physically visible and permit direct certified projection.
Original support and source laws remain separate from those completed functions;
multiple codes for a legal world never multiply its probability.

Each has an exact interning rule; none treats an expression hash as proof of
semantic equality. The important structure is the canonical partition of **the
same support**. Storing its two sides as two roots is one physical design.
[Representation](representation.md) records those layouts and ownership rules;
[the laboratory](experiments/event-repr-lab/REPORT.md) measures them through
Free Join before a production backend is selected.

The consequences are direct:

| Operation | Consequence of the representation |
| --- | --- |
| Complement | Toggle the selected-side bit: `region ^ 1` |
| Empty/full | The two orientations of the distinguished empty/full representative; no optional row or null pointer |
| Equality | Equal handles within the same canonical resident space |
| Copy | Copy two words; retain the same outcome identity |
| A fresh draw | Extend the source construction with a new outcome coordinate |
| Conjunction, union, XOR, difference | One canonical Boolean Apply engine, selected by a four-bit truth table |
| Disjointness | The conjunction has empty true region |
| Inclusion | The source's region outside the target union is empty |
| Relational product | Lift faces, conjoin and existentially eliminate middle coordinates |
| Domain / modalities / residuals | Reuse relational product, complement, and checked projections |
| Finite closure | Iterate a validated monotone finite program until canonical identity stabilizes |
| Probability | Contract with the designated joint law; MissingLaw if unmeasured |

A context or coordinate mismatch is rejected at alignment, before Boolean rewrites.
The owner is retained once per image, stage, or result batch. A standalone owned
`Event` includes its owner; it is not claimed to occupy only sixteen bytes.

The [checked relation laboratory](experiments/event-repr-lab/LEGAL-RELATIONS.md)
now makes an important refinement concrete: the value stays an Event, while
its execution view carries an owner and a proved dependency. A binary XY view
passes `Exists_Z(E) = E`; lawful relation operators preserve that fact. Legal
domains can leave unused encoding values and vary with a retained environment.
Containment plus exact cardinality verifies their product workspace without
enumerating it. Separate map certificates can remove redundant support gates.
These are internal capabilities over the same region, with independent Lean
laws and Rust checks; they are not a new public parameterized type.

## 2. Why the support boundary is part of the value

In Coup, the same physical card cannot be in two hands. A raw Boolean formula
can mention that assignment, but it lies outside the legal-world support `S`.
Mathematically, an event E determines the partition:

```text
T = S & E
F = S & !E
```

Two formulas differing only outside S have exactly the same partition.
Complement swaps its sides. This is the equality a manager must implement,
regardless of its physical data structure. Equal probabilities do not imply
equal events, and differently written formulas may denote the same event.

For example, fixing an admissible anchor ω gives an especially useful normal
form: store whichever side excludes ω. Its representative is false at ω and
outside S. After accounting for the public polarity bits, every Boolean Apply
can use a zero-preserving truth function. The result then stays normalized
without another support mask. Quantification and coordinate substitution still
need their own normalization proofs; the bit trick alone does not settle them.

The chosen support includes explicitly admissible worlds, including their
parameter conditions. A nonempty region may have zero mass under the designated
law. Parameter guards are compiled as logical coordinates with
explicit feasibility constraints; they are not random draws. The finite guarded
presentation and its limits are specified in [source-law.md](source-law.md).

## 3. One algebra across the database

The existing containment meanings survive:

```rust
At(position, card, when) -> At;
PositionCard(position, card, true) -> PositionCard;
PositionCard(position, card, true) == At(position, card, when);
```

Within one scalar determinant, a pointwise key rejects overlapping distinct
facts. A mirror establishes coverage in both directions. Those two laws prove
that a card's locations partition full, so their probabilities total one.
That numerical consequence assumes a measured context whose source constructor
has supplied a normalized joint law. The same partition is meaningful without one.

For event regions the implementation uses union and emptiness, not the temporal
endpoint sweep. A key can accumulate `seen`, test `seen & next`, then union the
next region. A containment packs the matching target regions and tests
`source & !covered`. Diagnostics still identify actual offending facts; a
combined region alone is insufficient provenance for a native violation report.

The [dependency experiment](experiments/event-repr-lab/DEPENDENCIES.md) now
implements a compositional version: retain Events for worlds covered at least
once and worlds covered at least twice. Their associative merge supports a
balanced update tree, so deleting a fact recomputes coverage without erasing
another contributor's overlap. Both key and containment proofs remain expressed
in the same region algebra. Whole-fact deduplication precedes that summary.

[Event surface](event-surface.md) gives the schema/constructor boundaries.
[Query algebra](query-algebra.md) gives explicit construction, structural tests,
world-relation operations, Pack, Probability, and Expectation over ordinary
relation bindings. Free Join keeps ordinary
row matching. It receives owned event handles; it does not multiply row odds.

## 4. Coup is the complete application

The [Coup schema](coup/schema.rs) covers physical cards, slots, perspectives,
claims, observations, and future decisions. The [queries](coup/queries.rs)
construct situations such as:

```rust
Event(bob_captain & !(cleo_captain | cleo_ambassador))
Event(old_duke & !new_duke)
Probability(cleo_duke, given & bobs_tax)
```

With Alice initially holding Duke and Assassin under a fair deal, the first
predicate has probability `189/1430`, about 13.22%. Under the explicitly assumed
80%/20% Tax policy, Bob declaring Tax changes Cleo's Duke probability from
`23/78` to `5/21`. The joint source supplies that connection. A fresh query does
not call a model to reconstruct it. The [walkthrough](coup/query-walkthrough.md)
states the assumptions and game-rule limits.

The [extended Coup applications](coup/algebra-applications.md) add observation
partitions, multi-threat events, certain consequences of uncertain choices,
weakest preconditions, and safe-action construction. The model supplies behavior
judgments; structural relations supply the admissible game semantics.

## 5. Make the machine work follow the representation

`BoolOp4` selects all sixteen binary Boolean functions. `0x8` means AND,
`0xe` OR, `0x6` XOR, and `0x4` A minus B, indexing bits by `(a << 1) | b`.
The operation code drives canonical Apply; dense cofactors of that same Boolean
function admit word and NEON bitwise execution.

A separate four-bit signature records which of the pair's four Venn cells are
nonempty. It supports exact predicates for overlap, inclusion, equality,
coverage, and complementarity. Operand complement and swap permute its bits.
It is a useful finite comparison coordinate system, not a quantitative
probability model or a promised strong-composition table.

[Kernels](kernels.md) maps this to candidate `EOR`, `AND`, `ORR`, `BIC`, `BSL`,
and `TBL` paths. Fixed-size ARM64 probes demonstrate instruction selection and
check semantics. They do not benchmark graph construction, law contraction,
or an integrated query. Diagram traversal and exact arithmetic retain their
real costs; the design does not call pointer chasing SIMD arithmetic.

## 6. What is decided, and what remains empirical

The proposal fixes the denotation, scoped handle contract, support-relative
partition invariant, explicit admissibility, source/event identity distinction,
required relational operators and query-empty behavior. It specifies proposed
ownership and publication contracts. It does **not** select a universal BDD,
bitmap, packed diagram or automatically mixed manager.

The [Rust laboratory](experiments/event-repr-lab/REPORT.md) compares competing
canonical carriers, including a controlled dense-kernel variant. Finite Coup, contiguous
sets, structured relational elimination, and huge symbolic spaces impose
different requirements. Coordinate movement, canonical publication, contraction,
updates and reclamation must also be charged before a production choice.

The [selector-block experiment](experiments/event-repr-lab/BLOCKS.md) now tests
small partitions throughout a diagram, alongside four sizes of packed terminals.
Face-major, bit-major and paired-bit layouts preserve identical mathematical
coordinates while changing physical decomposition. Coordinate maps are an input
to representation selection: a block that contains corresponding bits from each
state face can survive face renaming without rebuilding decision levels.
This checked structural property is useful across converse, composition and
residuals; no assumption about probability independence follows from it.

The current matched evidence favors dense storage for compact finite Coup and
interleaved packed terminals for the structured relation programs. A repeated
full-algebra query takes about 0.79–0.81 ms with packed512/packed64 versus
2.52 ms with dense; compact Coup takes 0.66 ms with dense. The
[map experiment](experiments/event-repr-lab/MAP-MEASUREMENTS.md) keeps two
renaming algorithms in the same binary and shows a substantial gain from local
table permutations without changing any of the eighty returned events.
These results narrow the next prototype; they do not select a universal manager
or measure shared-parameter contraction, persistent conversion, or admission.

The [new representation research](research/representation-search.md) explains
why stronger graph compression can make canonical operations more expensive,
why repeated renamed structure deserves a competitor, and which effect-algebra
terminology applies. There is no proof that one measured implementation dominates
all possible workloads. General support solving and exact inference remain
substantive reasoning tasks; fast word kernels do not remove those questions.
The [difference-basis investigation](experiments/event-repr-lab/DIFFERENTIAL.md)
adds a concrete tradeoff: positive Davio compresses grouped bilinear results
dramatically, while the tested Coup construction favors Shannon. The
[counting argument](research/compact-counting.md) shows why a compact cubic
coefficient graph cannot promise cheap exact uniform observation in general.
These alternatives keep the Event denotation and implicit normalized law;
their observer and transformation costs belong in the carrier comparison.
[Implementation plan](implementation-plan.md) gives acceptance criteria for the
remaining work and the measurements required to narrow the design.

The [law registry](laws.md), [decision record](decisions.md), and
[research index](research/README.md) separate current contracts from older
kernel/polytope alternatives. The supplied representation brief grounds the
ordering of this revision: make the state and its invariants explicit first,
then derive the algorithms and machine code.

The [new research adjudication](research/algebra-resolution.md) resolves the
model-import and operator questions using KAD, knowledge compilation, information
algebras, Jeffrey/Pearl updates, AMC, and FAQ. It also records counterexamples:
dead-end vacuity, noncommuting information saturation, incorrect evidence fusion,
and invalid aggregate-order swaps. The design is broad because its operators
share a concrete denotation, representation, and set of proof obligations.
