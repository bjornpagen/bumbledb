# The world descriptor and its probability law

**Revision 0.9.** The Event key names a region of explicitly admissible worlds.
A measured space additionally owns its designated normalized law; a structural
space can be unmeasured. This supports the [larger relational algebra](world-relations.md)
without inventing a probability law for a logical product. The
[source-process research](research/process-theory.md) still supplies the exact
law fragment, repeated sampling, and shared-parameter semantics.

## 1. A sealed space

```text
LogicalSpace {
    format_version,
    named_sources_and_outcomes,
    finite_outcome_encodings,
    parameter_domain D,
    ordered_guard_definitions,
    feasible_guard_and_outcome_support S,
    designated_measurement: Unmeasured | NormalizedJointLaw,
    coordinate_order,
    checked_construction_or_alignment_record
}
```

D is nonempty and rationally defined semialgebraic. An admissibility predicate
`H(theta,omega)` specifies the world support. Every theta in D must admit at
least one omega. A structural constructor can first compute the feasible
parameter domain `D & exists omega.H`, but must capture that domain explicitly
and refuse an empty result. Empty regions remain values within a nonempty space.
For a measured space, the joint law
`mu_theta(omega)` is defined for every theta in D and finite outcome assignment.
It is nonnegative, zero outside H, and sums to one on each parameter fiber.
It may assign zero to admissible worlds. Repeated draws can reuse
parameters; their outcome coordinates remain distinct. An explicit prior is a
separate constructor with its own supported integration class.

The adopted universe is

```text
W = {(theta, omega) : theta in D and H(theta,omega)}.
```

This replaces 0.4's implicit positive-mass support. Event equality is equality
as subsets of W, not almost-sure equality. Nonempty events can have zero mass.
An explicit `Supported(law)` view may restrict to positive mass, but neither
model output zero nor probability measurement silently changes admissibility.
Law changes create new designated contexts and explicit event maps; old keys
are immutable. MissingLaw is distinct from ImpossibleObservation and source
infeasibility. Full means W regardless of whether a measurement is designated.

## 2. Compile infinite parameter cases into finite logical guards

An event presentation contains finitely many semialgebraic predicates of theta:
structural source guards, imported event guards, and predicates defining H.
Positive-mass predicates are added only for an explicit supported view or another
operation that needs them. Give each predicate a logical guard coordinate. Its bit is
the truth of that predicate; it is not a Bernoulli variable.

For a guard valuation g and outcome assignment omega, include `(g,omega)` in S
exactly when some theta in D realizes g and satisfies H(theta,omega). The guard
roster must distinguish every event predicate and every support condition in the
presentation. Then every admitted event is constant on each `(g,omega)` cell,
and equality of finite support-masked Boolean functions is equality on W.

This is a finite quotient, not enumeration of real parameter values. A complete
reference constructor can use real quantifier elimination to decide feasibility
of each guard pattern and support condition. A symbolic builder can avoid
materializing all patterns, but must give the same S. Guard contradictions must
not be represented as independent Boolean freedom. Infeasibility, undecided
construction, resource exhaustion, and incompatible source binding are failures
with distinct diagnostics; none is silently converted into an empty event.

For a binary forecast with `0 <= p <= 1`, both named outcomes can remain
admissible even at p=0 or p=1. Heads at p=0 is nonempty and has zero mass.
An explicit supported view uses guards `p=0` and `p=1`, excludes heads at zero
and tails at one, and rejects both endpoint guards being true. The earlier
representation checker exercises this explicit support-restriction example;
the new algebra checker also checks the unreduced structural space.

Boolean query expressions introduce no new guards. Relational operators may
eliminate finite outcome coordinates while retaining source parameters. A
constructor introducing another parameter
predicate creates a refined descriptor and a checked lift. The finite guard
roster is part of the sealed scope, not an unbounded mutable global truth table.

## 3. The law is a separate arithmetic object

When a measurement is designated, use an ordered algebraic decision diagram, or its compiled contraction circuit,
whose leaves are exact **per-outcome joint mass functions**. It shares semantic
coordinate identities with the Boolean arena. Guard variables select cases;
outcome variables select finite stochastic assignments. A law decision diagram
may reduce equal children, so evaluation must account for skipped coordinates.

A proposed `WeightRef` points to an owned exact scalar-function arena:

| Function kind | Stored description |
| --- | --- |
| Rational constant | Reduced arbitrary-precision numerator/positive denominator |
| Polynomial | Interned arithmetic DAG, with sparse rational coefficient normal form for exact comparison/export when needed |
| Rational function | Numerator/denominator functions plus a proved positive/nonzero domain guard |
| General semialgebraic function | Rationally defined, functional graph and its domain, with exact comparison/elimination obligations |

A polynomial DAG is not itself a canonical function encoding. Cache keys based
on structure are only structural keys. Source identity is its declared named
allocation and validated definition; arbitrary equivalence of two parameterized
laws is a separate exact judgment. Canonicalization of rational/polynomial
fragments is concrete; general semialgebraic equality requires the retained
logical machinery. The initial native implementation can stage those source
constructors without weakening event semantics or substituting interval hulls.

Conditional policies must form a full joint law. Multiplying arbitrary marginal
Noul answers is not a constructor for dependence. Conditional product is valid
only where the declared generator explicitly draws independently given the
retained parameters and parent information. Alternative admitted couplings can
be encoded by joint-mass parameters and normalization/marginal constraints.

## 4. Exact measurement is a contraction

```text
mass_theta(E) = sum_omega mu_theta(omega) * indicator_E(theta,omega)
Probability(E,G) = mass_theta(E & G) / mass_theta(G)
```

A joint traversal combines the Boolean indicator and law diagram. At an outcome
bit, add the two branches; at a parameter guard, retain/select the corresponding
conditional expression. **Never sum guard branches as stochastic alternatives.**
If both diagrams skip an unconstrained binary outcome bit, per-outcome weights
contribute twice. Invalid categorical codes are outside structural support and
have zero mass; valid categorical codes may also have zero mass.
Skipped guard bits contribute no multiplicity. A contraction state therefore
includes its next semantic coordinate, not only two graph roots.

For an arithmetic circuit with factored weights, equivalent variable elimination
or compiled model counting can replace the ordered-law traversal. Its correctness
requires the actual factorization, deterministic alternatives, and handling of
skipped variables. Recursing on an event BDD with a vector of marginal bit
probabilities would be wrong for Coup's shared deck.

Contraction returns an exact function DAG and its defining domain. Fixed rational
sources give exact rationals; parameterized cases can give polynomial, rational,
or piecewise semialgebraic functions. The result retains numerator, evidence
mass, positive-evidence domain, and source ownership. A reported range also
carries endpoint attainability and whether it is defined on the full source
domain. Impossible evidence is a result, not a fabricated probability zero.

Complement allows `mass(!E) = 1 - mass(E)` for a normalized whole-space law.
Given evidence, the numerator for `!E` is `mass(G) - mass(E & G)`, not generally
`1 - mass(E & G)`. This lets one pair cache share its unconditioned measurement
without confusing a proper evidence region with the universe.

The proposed owned result shape is:

```rust
enum ProbabilityObservation {
    Impossible { owner: Arc<EventSpace> },
    Conditional {
        owner: Arc<EventSpace>,
        numerator: ExactFunctionRef,
        evidence: ExactFunctionRef,
        defined_on: DomainRef,
    },
}
```

Here `Impossible` means zero evidence mass throughout the designated law family,
so this conditional measurement is undefined. It does **not** establish structural
emptiness: G can be a nonempty admissible zero-mass region. Structural queries
about G remain valid. The result must explain this distinction to callers.

The owner validates all function/domain references. MissingLaw, source construction,
and scope errors remain errors outside this result. A point rational and a reported
range are views computed from Conditional, not additional copies of the law.
Its query value/serialization is a separate owned observation type; it is not
encoded as an EventKey or silently squeezed into today's `Value::F64` slot.

## 5. Projection and composition are different operations

A query's ordinary projection drops columns and retains event values. `Pack`
unions events from matching rows. Neither operation forgets outcome coordinates
inside a captured source.

Existentially forgetting an internal logical variable asks whether some witness
exists; summing out an outcome measures all its alternatives. Those are different
operations. Event cofactors, substitution, checked coordinate lifts, finite
existential abstraction, and exact measurement are useful internal algebra APIs,
but any public scope-changing operation must state its support and law mapping.
Renaming a source, resampling it, conditioning, and causal intervention are not
aliases for one substitution routine.

## 6. Constructor obligations

Before a space or extension is admitted, establish: nonempty parameter domain;
finite named encodings; feasible admissibility/guard interpretation; a nonempty
world fiber at every admitted parameter assignment; scope alignment; and all
referenced owner lifetimes. A measured
space additionally establishes one normalized nonnegative joint law per parameter
fiber, zero outside admissibility. Source construction is explicit upstream work.
FD/IND admission validates event facts under the supplied space; it does not
invent a joint law or repair malformed inference output. Structural products
remain unmeasured until a justified coupling/channel law is supplied.

TypeSafe Noul supplies one binary probability; Choice supplies an option
vector. Their adapters preserve question, information case, decision identity,
source version, and exact decoded numerical values. They construct a joint
partition or constrain an existing law through a declared operation. No
separate Noul confidence field, automatic confidence prior, or per-row mass
normalization is introduced. The active [Coup example](coup/README.md) shows
these boundaries in a real schema.

The complete import/revision contract is [TypeSafe inference](typesafe-inference.md).
Likelihood and Jeffrey revision normally retain structural admissibility while
changing the law and its positive-evidence parameter domain. Restricting the
world space to evidence or positive posterior mass is an additional explicit
view. A posterior can therefore assign zero to a structurally possible event;
only structural restriction merges previously distinct regions.


## 7. Native fixed finite slice

The [native source contract](../docs/event-sources.md) now implements the fixed
finite rational fragment through canonical equal-density Event partitions.
`Space::with_density` validates a normalized joint law; `Event::mass` contracts
it using legal counts, and `Event::probability` retains both exact masses and
source ownership. No parameter predicates enter this finite outcome roster.
BEVT v2 retains the law in canonical identity and rechecks it on decoding.

This is an initial representation of the arithmetic function described above.
The [finite function/channel layer](../docs/event-functions.md) now also
implements conditional categorical generators over checked structural readouts.
Its weighted image sums each outcome coordinate exactly once, validates every
parent row and proves the reference old-marginal law under closure.
[Fixed-law revisions and signed expectation](../docs/event-revisions.md) now
retain evidence/factors/target distributions and explicit prior-to-posterior
Event translation. Zero-posterior worlds remain legal, and zero evidence or
unsupported positive targets produce owned impossible results. Shared univariate
parameters and guarded rational functions now have the separate source layer
described below. General semialgebraic functions, solver certificates, TypeSafe
import intents and database observation query results remain acceptance gates.
In particular, the finite scalar
observation must not replace the retained domain/function semantics of a
parameterized observation.

The [native polynomial foundation](../docs/event-polynomials.md) now implements
canonical named-parameter rational coefficients, simultaneous substitution,
exact evaluation and explicit Beta moment integration. BEPL v1 transports that
arithmetic identity. The arithmetic object alone does not designate a Space law
or admit a parameter domain. Its equality is unconstrained polynomial identity;
domain-relative equality, guard feasibility and normalized fiber laws retain
their separate solver obligations. The [polynomial SDK](../docs/event-sdk-polynomials.md)
now exposes the same exact expression algebra and explicit Beta moments without
inventing source domains or priors. Its stored consumer specializes an explicitly
supplied full prior cube, checks the resulting finite joint law and preserves
shared-prior correlation through reopen and queries.

The [native exact-root capability](../docs/event-algebraic-roots.md) now isolates
all distinct real roots of a univariate rational polynomial and checks polynomial
signs there. This admits irrational witnesses without rational-grid assumptions.
Root equality is an exact numeric judgment across checked presentations, not
byte equality of arbitrary defining polynomials. Multivariate elimination remains
required; univariate source transport is implemented below.

[Native parameter regions](../docs/event-parameter-domains.md) now implement
univariate sign domains and their exact Boolean algebra, including irrational
singleton witnesses and strict endpoints. Guarded rational functions retain
their ambient domain, numerator/denominator and all inherited undefined points.
They are host arithmetic objects; the source constructor below additionally
validates and owns them as laws. Multivariate constraints remain open.

[Canonical real-algebraic and parameter-region transport](../docs/event-algebraic-identity.md)
now uses monic minimal polynomials with ordered real-root indices for numeric
boundaries (BEAR v1), and essential boundaries/membership with the captured
parameter name for regions (BEPR v1). Bounded exact factor search can refuse;
an unfinished search never asserts irreducibility. These standalone arithmetic
formats are now embedded in the parameterized Event source envelope below.

## 8. Native shared univariate source layer

[The source implementation](../docs/event-parameter-sources.md) now seals one
inhabited real domain, guard roster and finite outcome presentation into `Space`.
Every guard code denotes an exact parameter region. Admission removes infeasible
codes and refuses a requested parameter assignment with no legal outcome.
Events inhabit the finite quotient while denoting actual `(parameter, outcome)`
worlds, so schema keys/containments retain possibility and shared correlation.

Optional guarded rational-function densities are nonnegative, defined and
normalized on every fibre, including isolated irrational points and endpoints.
Contraction fixes guards and sums each outcome once. Piecewise partial functions
retain holes; owned probability observations retain source, numerator, evidence
and the positive-evidence domain. Fixed scalar APIs refuse these families.

BEVT v3 captures canonical domain/guard bytes and the validated declared law.
Law identity follows section 3's named-allocation/definition contract, not
arbitrary numerical equivalence of fractions. The decoder reestablishes support,
feasibility and every normalization obligation before admitting a value.

Checked maps/products preserve the actual parameter. Every source cell must
equal one whole target cell; source-domain inclusions are allowed. Explicit common refinement/lifts are implemented below;
scope-changing maps remain required. Matching only coarse codes would be unsound.
Sealed fixed points count logical
atoms while semantic world counts distinguish finite sets from a continuum.

This closes the initial univariate source ownership/contraction slice. It does
not close multivariate/general semialgebraic capabilities, family channels and
general revisions, source prior integration, query observation heads, TypeSafe intents or
complete SDK authoring/results. M0–M8 retains its complete acceptance scope.

## 9. Native guard refinement and exact representability

[ParameterRefinement](../docs/event-parameter-refinement.md) now extends sealed
presentations with exact deterministic predicates of the same real parameter.
Checked lifts retain every old Event and its family law. Common refinement gives
multiple sources presentations of the same exact parameter partition; existing
maps can be lifted and revalidated without identifying different real values by
their coarse codes. Outcome coordinates remain unchanged.

Exact descent first constructs a logical existential candidate, then accepts it
only when its lift equals the original refined Event. This recovers precisely
the old representable Events, including expressions where new distinctions
cancel. It refuses an essential new guard rather than publishing an approximate
condition. The original scope and its fixed-point certificate remain immutable.

Refined Events and resulting maps/products use existing BEVT/BEDC transport and
ordinary schema/query paths. Refinement certificate transport, SDK authoring,
query refinement instructions and parameter-changing maps remain required.

## 10. Native domain inclusion and conditional families

[ParameterRestriction](../docs/event-parameter-conditioning.md) now captures a
smaller inhabited parameter domain, retaining every original outcome fibre and
the same law on that domain. It makes the boundary expressible in a refined
prior and constructs a checked inclusion. Each source cell equals a whole target
cell, so an image cannot borrow a witness from a different parameter. Missing
target cells are unreachable, universal images there are vacuous, and proper
inclusions are not surjective. Explicit function-domain rebasing retains holes.

`Space::parameter_condition` contracts the exact evidence mass, solves its
positive region and constructs the normalized posterior there. Every admitted
parameter keeps all outcomes, including posterior-zero outcomes. The receipt
owns the prior, original evidence, complete evidence-mass function and exact
defined region. Everywhere-zero evidence returns an owned impossible receipt,
not an empty source. Zero-evidence parameters stay explicit in the prior/receipt;
their absence from the posterior domain is undefined conditioning, not a claim
of prior structural impossibility.

The posterior translation targets the refined prior and need not be onto.
Posterior Events and maps use ordinary BEVT/BEDC, FD/INDs and both Free Join paths.
Family likelihood/Jeffrey updates, channels and law pushforwards, prior
integration, receipt transport and full consumer APIs remain required. The
fifteen new Lean reports state domain/image and pointwise conditioning laws under
explicit source/roster premises; they do not certify native solver or codec code.

## 11. Signed finite payoffs under a family law

[`FiniteFunction::parameter_expectation`](../docs/event-parameter-expectations.md)
now returns a source-owned signed conditional function of the same parameter.
It contracts each explicit payoff cell with the captured law, retains the original
payoff/evidence and both mass functions, and excludes exactly the zero-evidence
parameters. No prior or parameter averaging is introduced. Signed cancellation
and a zero payoff cannot make undefined conditioning defined.

The exact positive region of that function can be admitted as a new parameter
guard and returned to ordinary Event algebra. This describes parameter cases,
not permission for a policy to read hidden information. Six native tests and
twelve Lean reports cover the finite-payoff contraction/definedness contract.
The [family function layer](../docs/event-family-functions.md) below now adds
guarded rational payoff values. Observation transport/query heads, SDK results
and query-observable coverage remain required.

## 12. Native family functions and likelihood revision

`FamilyFunction` now owns disjoint Event cells with exact signed guarded
rational-function values. Every coefficient is defined on its cell's active
parameter fibres; omitted worlds receive an explicit zero default. Active holes
refuse even at zero-prior worlds. Pointwise arithmetic, total division, exact
numerical equality/nonnegativity, sign Events after explicit guard refinement,
law capture/designation and parameter-dependent expectations use this contract.

Pullback preserves the same actual parameter. Weighted image factors each
coefficient from its finite outcome sum and uses the existing symbolic image
kernel for multiplicity. Its parameter-cell correspondence remains a map
admission premise. Unreachable target parameter cases receive zero; a density
cannot be designated over a larger domain where its total is zero. No prior
or independence is introduced.

`Space::parameter_likelihood` now accepts an explicitly interpreted nonnegative
family factor, retains its scale and exact normalizer, and constructs the
posterior on the positive-normalizer domain. Every outcome there remains
structurally possible. Indicator factors agree with conditioning; repeated
general factors multiply rather than becoming idempotent observations. Negative
factors on possible zero-prior worlds refuse. Everywhere-zero normalizers retain
an owned impossible receipt.

Eleven new core tests, a persisted Free Join consumer and seventeen Lean reports
cover the function/fibre-sum and likelihood contract. Family channels, Jeffrey
updates, multivariate/general semialgebraic capabilities, prior integration,
function/receipt transport and full query/SDK/TypeSafe integration remain required.


## 13. Native family channels and replacement targets

[Family dynamics](../docs/event-family-dynamics.md) now implements conditional
channels over shared parameters. Unit row sums include every zero-prior world;
closing against an explicit prior preserves its exact marginal. Numerical
function descent checks information FDs independently of authored coefficient
syntax. The same-parameter readout contract does not hide the parameter itself.

`parameter_jeffrey` accepts total nonnegative partition targets summing to one.
It retains ordered targets, original masses and per-cell regions where a positive
target has zero prior support. Their complement is the exact posterior domain.
Zero targets skip undefined conditionals while all outcomes remain possible at
admitted parameters. Piecewise targets refine guards without losing the original
prior translation. Repetition is numerically idempotent; overlapping replacements
can fail to commute. These targets are distinct from likelihood factors.

Nine new core tests, a persisted channel/replacement Free Join consumer and ten
additional Lean reports qualify this slice. Full family receipt/function/channel
transport, prior integration, scope-changing maps, multivariate/general function
solving and query/SDK/TypeSafe integration remain required.

## 14. Native family transport and replay

[BESC v2](../docs/event-source-descriptors.md) carries partial parameter functions,
total family functions, everywhere-normalized channels, guard refinements, domain
restrictions and complete conditioning/likelihood/Jeffrey receipts. BESC v1 fixed
objects retain their bytes. Parsing is untrusted; admission reconstructs native
objects with one shared exact-arithmetic budget and replays every update.

Parameter-function holes remain undefined, while family functions have a total
zero default and retain explicit zero definitions. Receipt equality checks
numerical functions together with defined domains and every indexed unsupported
region. Posterior and refined-prior contexts require complete named support/law
identity. Requested presentation identities survive even impossible updates.
Replayed refinements preserve translation from the original source. A checked
channel can close under an explicit prior after its original owner is released.

Ten core tests, a persisted import/reopen/Free Join consumer and 20 additional
Lean reference reports cover this slice. They do not verify native parsing or
solving. SDK family authoring/results, function-valued query observations,
parameter-changing maps, multivariate solving, prior integration and TypeSafe
response provenance/visibility remain required.
