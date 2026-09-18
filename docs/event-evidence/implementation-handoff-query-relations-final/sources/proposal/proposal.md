# Event: implementation contract

**Revision 0.9; proposed extension to public BumbleDB v1.3.1.** This is the
current contract. [Representation](representation.md) chooses an initial backend;
[implementation](implementation-plan.md) orders the native integration. Historical
representation candidates are evidence, not alternative semantics.

## 1. Value, owner and equality

`event` is a structural schema field; `Event` is its owned Rust value. It denotes
a subset of one nonempty explicitly admissible world space. A world fixes the
modeled finite outcomes and any retained source parameters. Events are equal
exactly when they select the same admissible worlds in an aligned context.
Formula spelling, scalar probability, hash equality and almost-sure equality
cannot replace that judgment.

Rust `Eq`/`Hash` compare scoped resident identity. Independently decoded values
must be aligned before that comparison has the semantic meaning above. Canonical
persistent bytes compare source identity, original support and supported
membership. The native query registry performs that alignment for its bindings;
standalone API callers use checked alignment explicitly.

```rust
#[repr(C)]
struct EventKey { space: u64, region: u64 }
```

This is the resident row/binding key. An owned API value additionally retains
its owner. A checked registry resolves the space, coordinate meanings, legal
support, representation and optional designated law. Constants also retain and
validate their owner. Context alignment precedes simplification, including
`Empty(a) & b`. Unrelated sources never acquire a coupling by accident.

Empty and full are ordinary values through construction, storage, transport,
queries and publication. `Some(empty)`, absent row, absent group, invalid source
and failed evaluation are distinct. A source universe cannot itself be empty.

## 2. One region algebra

The Boolean core supplies all sixteen binary truth functions, complement, ITE,
finite cardinality predicates, structural inclusion/equality/emptiness, and
legal witnesses. Results remain Events. Counts and witnesses refer to legal
worlds, not aliases introduced by the machine encoding.

A typed world relation is an Event with checked endpoint faces. Its ambient
space is the full product of legal endpoint fibres over one retained environment.
Transition restrictions live in the relation. The role certificate additionally
proves that the region depends only on its declared faces and environment.
Support compatibility does not by itself prove that role.

Required operations are identity/tests, converse, composition, domain/range,
May, All, nonvacuous Must, both residuals, information abstraction, and sealed
finite closure/fixed points. Composition retains one shared middle world and
one shared source environment. Logical products do not assert probabilistic
independence. General coordinate maps require the specified support, projection
and law certificates for operations that depend on them; those capabilities are
distinct. The [native finite map layer](../docs/event-maps.md) already implements
arbitrary Event readouts, pullback, image and separate surjectivity admission.
The [finite relation core](../docs/event-relations.md) now adds full relation
products, complete projection-square certificates and role-checked operations.
The [inspection API](../docs/event-inspection.md) exposes both original support
and completed membership through owned snapshots with safe borrowed node/table
views. Reconstruction checks the same named legal context across working orders.
Snapshot indices and physical shape never define Event identity.
The [finite information layer](../docs/event-information.md) now supplies readout
saturation, explicit-evidence cases, FD factorization and indistinguishability
relations. The [finite program layer](../docs/event-fixed-points.md) now supplies
owned instruction DAGs, monotone admission, least/greatest iteration and Star.
The [partition helper](../docs/event-partitions.md) supplies parent-relative
value/count rosters, ordinary grouping, shared-world refinement and readout
conversion, while scalar values remain ordinary columns.
Measured-source/query integration retain their own implementation gates.
Law pushforward and continuous-parameter integration remain source work.
Total maps suffice for Boolean pullback;
surjectivity additionally reflects possibility/equality, and complete fibres are
the stronger, square-specific condition required for quantifier base change.

`All(R,E)` includes dead ends. `Must(R,E)` also requires enabledness. The exact
composition rule is:

```text
Must(R, Must(Q,E))
  = Must(Compose(R,Q), E) & All(R, Domain(Q))
```

Dropping the last factor loses dead intermediate branches. The
[public modal proofs](semantics/ModalContract.lean) establish this rule and retain
a counterexample to the unqualified rewrite. They also prove typed residual
adjunctions, greatest uniform permissions and finite-path closure laws.

Information cases describe all hidden states consistent with the declared
observation. A uniform action must work throughout its case. Quantifier order
is explicit: `exists action, forall hidden state` differs from its reversal.
Permission construction excludes empty observation cases and incorporates
actual action enabledness. Repeated strategies additionally require an adequate
information-state/memory model.

The [native action layer](../docs/event-actions.md) now checks the state/action
product against the transition's environment readout, constructs enabled Good
relations and supplies the greatest uniform one-step permissions on inhabited
cases. Fully observed reachability retains first-entry rank partitions and
actions whose every outcome strictly decreases rank. Continuing safety retains
enabled actions inside its invariant. Supplied policies can restrict those
choices only while preserving the required state coverage. Building adequate
belief/memory states for repeated partial-observation play remains separate work.

`Star` denotes finite paths, including the empty path. Native fixed-point
execution requires a sealed finite stable presentation and a validated monotone
program. Stochastic eventuality and arbitrary continuous state transformations
are not inherited from those structural laws.

For a complete finite roster of N legal states, bottom iteration stabilizes by
step N; one more application can detect equality. Complement duality gives the
greatest fixed point. The [bounded iteration proofs](../crates/bumbledb-event/semantics/FixedPoint.lean)
establish both statements. The same uniform bound holds across a fixed shared
environment when operators act separately in each environment; a proved
counterexample shows why mixing environments invalidates that argument.
This semantic bound does not promise affordable execution or validate native IR.
The native evaluator checks positive variance, counts original legal support,
aligns each iterate and includes the final equality-detection application in its
N+1 bound. Budgets can refuse execution but never return a truncated approximant
as a fixed point. The current certificate is for finite unmeasured spaces;
source-parameter presentations still need their separate admission contract.

## 3. Native dependency semantics

Within a scalar determinant group, a pointwise key rejects overlap between
**distinct whole facts**. Containment requires a source region to lie in the
union of matching target regions. A mirror gives both coverages. Scalar roster
constraints and exact target-key resolution retain their existing meanings.
Each statement group checks a common Event context before using those set laws,
including empty source-only groups. Different scalar groups may use different
contexts. Canonical whole-fact deduplication precedes conflict accumulation.

For a disjoint cover of full, any normalized designated law gives branch masses
summing to one. For a cover of a proper parent, their masses sum to the parent's
mass. The [finite measure proof](semantics/FiniteMeasure.lean) connects distinct
fact IDs, pointwise uniqueness and coverage to this partition identity. The
source constructor separately establishes a valid normalized law.

Empty-valued facts add no coverage/conflict but still exist and satisfy scalar
laws. Equal complete pointwise keys imply one fact only with a nonempty region
witness or a separate ordinary key. The planner must not inherit Allen's
nonempty-interval shortcut unconditionally. Deletion recomputes coverage from
remaining contributors; subtracting a removed region from a union is unsound.
The [admission reference proofs](../crates/bumbledb-event/semantics/Admission.lean)
establish coverage/conflict summaries, traversal independence and exact conflict
citation membership under the aligned-context and distinct-fact premises. The
native ledger separately records field/full dependency tests. The
[contextual projection contract](../docs/event-projections.md) implements typed
`true` without a stored owner, proves its nonempty-world premise and preserves
exact target-key resolution. It is a full Event in a checked statement context,
not a Boolean cell or an invented field ID.

Finite structural imports now have a separate [BEDC v1 descriptor format](../docs/event-descriptors.md).
It captures authored names, original space markers, readouts and endpoint
orientation. Import reconstructs support, surjectivity, complete squares and
shared-environment checks. Parsed descriptors are inspectable untrusted data;
only admission returns executable owners. This does not supply measured-source
formats or implement the proposed query import syntax.

## 4. Queries and diagnostics

Ordinary Free Join binds facts and Event handles. Explicit Event heads construct
regions; Event-valued Pack unions a bound region by group; Probability observes
a bound region under explicit evidence. Dependent outputs use separate stages.
Relational anti-probing remains distinct from Event complement. An absent Pack
group remains absent unless explicitly seeded with an empty value.

Certified branch factorization is an optional execution schedule. It must prove
scalar rectangularity, preserve all world/environment witnesses, preserve group
presence and represented multiplicities, and validate every participating
operand even after saturation. Unmatched operands remain unevaluated. Residual
aggregation has its stated intersection contract; ordinary union Pack cannot
silently change into it.

For new Event validation failures, the production contract is a **deterministic
set of participating operand faults**, not whichever error physical traversal
encounters first. Each fault retains logical stage/expression/operand identity,
error category and canonical offending value/source identity. Deduplicate and
order these semantic descriptors for presentation; arena IDs and join order
must not select the diagnostic. Resource refusal may abort collection explicitly
and must not present a partial set as complete. Existing scalar error semantics
are unchanged. The lab validates unordered fault sets. The native complete-binding
implementation below supplies stable descriptors through captured maps and
typed relation programs. Extending them through factored execution remains
acceptance work.

The [native Event/Test query slice](../docs/event-queries.md) now implements
this diagnostic contract for constructive heads, structural tests and typed
map/relation boundaries. Each written leaf occurrence retains its expected context.
It retains every written-rule stamp through DNF collapse, validates ignored
operands before evaluation and defers semantic faults until stage sealing.
Query.lean proves the reference participation and collection laws. Native Event
Pack now groups complete bindings, retains claims through scratch spill, and
unions one value per present group. A group's least canonical full-space
BEVT descriptor selects deterministic context diagnostics; all claims, including
empty/full values, remain participating. Invalid computed keys contribute head
faults without inventing group keys; later valid keys still enter Pack. Pack.lean
proves these reference boundaries. Face imports, relational programs and
certified factoring still need integration.

## 5. Sources and exact observation

An unmeasured space supports structural reasoning and returns MissingLaw for a
probability observation. A measured space designates one normalized joint law
or constrained parameterized law family. Source names do not imply independence.
Copying an Event, allocating a new outcome, reusing an observation and drawing
again are different operations.

```text
Probability(E,G) = mass(E & G) / mass(G), where mass(G) > 0
```

The result retains numerator, evidence mass, source and domain of definition.
Impossible evidence is explicit. A range is a readout of the exact result,
including endpoint attainability and partial definedness, not the compositional
Event value. Parameter guards select logical cases; only outcomes are summed.
A nonempty legal Event may have mass zero, and probability one need not be full.

Fixed rational finite laws are the initial exact arithmetic fragment. Shared
polynomial parameters, guarded rational functions and general semialgebraic
constructors remain required staged source capabilities with explicit solver
obligations. Unsupported constructors and exhausted resources refuse explicitly;
neither sampling nor floating tolerance can decide equality or admission.

TypeSafe imports declare whether they create a judgment, extend a conditional
forecast, constrain an existing law, revise a posterior, or supply a likelihood.
The adapter preserves named propositions, alternatives, information cases and
provenance. Separate marginal answers cannot manufacture joint dependence.
Model confidence does not certify game rules, calibration or actor visibility.

## 6. Representation and release boundary

The initial general backend is a canonical completed Boolean function with
symbolic splits and essential-coordinate tables. Its decoder maps raw codes
onto legal worlds and fixes legal worlds; original support and laws remain
available for exact observation. The initial local-table cutoff is nine raw
coordinates, an internal tuning choice. Direct ITE, staged projection and
certificate-driven gate/local-repair selection follow the validated experiments.

Dense and packed512 carriers remain independent comparison controls. Backend
selection, graph compression and instruction selection do not alter the public
algebra. No universal performance winner or automatic cross-carrier policy is
claimed. Published owners keep immutable identities and order; alternate working
orders require explicit checked translation.

Persistence must establish canonical per-fact bytes independently of resident
allocation order, decoder choice and execution layout. The implemented
[BEVT v1 codec](../docs/event-value-format.md) selects fixed semantic-coordinate
order for unmeasured finite spaces and is integrated into native row storage.
It does not encode a measured law or a continuous parameter presentation.
The complete M2 gate still requires all supported descriptors/carriers and
consumers to satisfy identity, ownership, compatibility and publication checks.

The [proof matrix](semantics/README.md) states what is formally established and
what still requires native tests or new proofs. The [implementation plan](implementation-plan.md)
is the sole active work sequence. Historical performance experiments and the
shelved participation draft are not required implementation tasks.
