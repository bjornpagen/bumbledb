# Owned predicates and explicit Event guards

`ObservationPredicate` retains an exact true/false/undefined partition and the
entire calculation or exact region membership that produced it. Fixed observations yield an optional truth
value. Shared-parameter observations yield three exact regions covering one
inhabited ambient domain. No prior is introduced over those assignments.

Rust query IR/macros and Node/SDK builders now produce and stage these values.
They are query observations, separate from stored Event fields and scalar Bool
columns. Explicit guard construction bridges them into the Event algebra.

## Algebra and identity

`number.where_sign(signs, limits, work)` retains the selected number and sign
mask. `left.compare(right, signs, limits, work)` retains their numerical
difference and tests its sign. ZERO means equality, POSITIVE means greater than,
NON_NEGATIVE means greater than or equal, and all eight sign masks are available.

The resulting predicate supports `negate`, all sixteen `BoolOp4` operations,
explicit `on_domain`, and `equivalent`. The owned expression exposes Sign,
Negate, Binary, OnDomain and Region nodes. Every written operand remains reachable,
including redundant branches and operands of a constant truth table.

Boolean operations are strict partial operations. Both inputs must be defined;
even `TRUE(a,b)` retains their holes. Negation exchanges true and false while
keeping undefined unchanged. Consequently, complementing only the true region
does **not** compute predicate negation when undefined assignments exist.

`predicate().possibly()` requires a true assignment. `always()` requires true
at every ambient assignment, including definedness everywhere. `is_total()`
tests whether undefined is empty. `equivalent` compares all three regions; it
does not identify original sources or evidence. Fixed predicates lift to a
parameter operand's domain. Two parameter operands require the same named
ambient domain. `on_domain` explicitly restricts that domain and keeps the
original observations in its child derivation.

## Query construction and explicit quantifiers

```rust
let decisions = query!(Game {
    interior observed(game, p: Probability(claim, evidence))
        | Claim(game, claim, evidence);
    interior verdicts(game, v: Predicate(Value(p) > 1/2))
        | observed(game, p);
    (game, v,
        opposite: Predicate(!v),
        possible: PredicateTest(Possibly(v)),
        certain: PredicateTest(Always(v)),
        defined: PredicateTest(IsTotal(v)))
        | verdicts(game, v);
});
```

`Predicate(...)` produces a retained truth partition. `PredicateTest(...)`
produces an ordinary Bool. The latter can be filtered in a subsequent stage;
the former can be projected, grouped, joined by identity, negated, combined,
imported or forwarded through projection-only recursion. Constructive predicate
heads remain outside recursive rules.

Numerical `> >= < <= == !=` compare an exact difference. `!`, `&`, `^`, `|`
compose predicates, with the usual precedence. `Sign(number, mask)` exposes all
eight sign masks; `Bool4(mask, left, right)` exposes all sixteen truth tables.
`Imported(name)` uses `use predicate name = &checked_benp;`. Explicit
`OnDomain(value, name)` uses `use number_domain name = &domain;` and validates
domain restriction. Boolean masks never authorize ignoring an invalid operand.

The TypeScript authoring API mirrors these operations without doing mathematics
in JavaScript:

```ts
const verdicts = query(Theory).rule((r) => {
    const p = v(observed)
    return r.match(observed, p).find({
        game: p.game,
        verdict: PredicateExpr.greater(NumberExpr.value(p.chance), NumberExpr.literal(half))
    })
})
const possible = query(Theory).rule((r) => {
    const v0 = v(verdicts)
    return r.match(verdicts, v0).find({
        game: v0.game,
        holdsSomewhere: PredicateTest.possibly(v0.verdict),
        holdsEverywhere: PredicateTest.always(v0.verdict),
        fullyDefined: PredicateTest.isTotal(v0.verdict)
    })
})
```

`PredicateExpr.sign`, `compare`, all six named comparisons, `apply`, `and`,
`or`, `xor`, `not`, `imported`, `region` and `onDomain` construct owned programs.
Sign masks accept the existing `PolynomialSigns` constants. Description replay
uses `predicateResult`. `PredicateAnswer` retains an `ObservationPredicate`
with BENP bytes and either a fixed `boolean | null` or parameter `holds`,
`fails`, `undefined` regions plus the exact `domain`. `null` means undefined.
`ObservationPredicate.fromBytes` copies a bounded envelope; native worker
admission replays its complete derivation before query preparation, including
imports in unreachable producers.

The exact shared parameter matters. For `low = θ < 1/2` and `high = θ > 1/2`
on `[0,1]`, both `Possibly(low)` and `Possibly(high)` hold, while
`Possibly(low & high)` is false. Combining two existential answers would lose
the requirement for one shared witness. Similarly, absence of a false witness
implies `Always` only when the predicate is total.

Before aggregation, completed predicates receive canonical execution tokens
keyed by full BENP derivation. Identical source/evidence/expression receipts
share a group; two different derivations that both happen to be true do not.
Ordinary predicate equality in a join compares this complete identity. It is
neither pointwise truth equivalence nor an implicit test that a claim holds.
Stored scalar comparisons, numeric folds, Event operands and query parameters
refuse predicate values without an explicit operation changing their type.

One execution arithmetic counter spans numerical/predicate construction and
probability/expectation contraction across stages. Producer failures remain
observable behind downstream filters. Spill and answer append rollback retain
owned derivations; closing a database or reusing prepared storage does not
invalidate returned predicates. Query tree admission counts numerical children
inside predicates: 65,536 nodes / depth 256 in native IR, 4,096 nodes / depth 128
in the macro/SDK/Node grammar. Worker import and delivery budgets additionally
bound bytes and exact work. These are not an aggregate memory accounting claim.

## Direct parameter-region membership

An existing exact `ParameterRegion` can enter the same predicate algebra without
manufacturing a probability observation. `ObservationPredicate::region(domain,
region, limits, work)` is total on the explicit inhabited domain: its true region
is `domain ∩ region`, its false region is `domain \ region`, and undefined is
empty. Both inputs must name the same parameter, even for empty/full regions.
The complete region is checked and retained, including parts outside the domain.

```rust
let domain = NumberDomain::capture(ambient, codec_limits, &mut work)?;
let region = ParameterRegionImport::capture(allowed_bias, codec_limits, &mut work)?;
let query = query!(Game {
    use number_domain d = &domain;
    use parameter_region r = &region;
    use guard g = &plan;
    interior truth(game, p: Predicate(Region(d, r))) | Position(game);
    (game, p, permitted: Guard(Holds(p, g))) | truth(game, p);
});
```

`PredicateExpr::Region { domain, region }` is the native IR node. The SDK spells
it `PredicateExpr.region(domain, region)`, using existing owned `ParameterDomain`
and `ParameterRegion` carriers. Raw Node uses
`{kind: "region", domain: fullDomainBEPR, region: regionBEPR}`. Both buffers share
the query byte budget; worker admission checks both exact encodings, including
unreachable imports. Domain/name compatibility is checked at interpretation.

Membership can be negated, quantified, combined with numerical predicates,
staged, joined by complete derivation identity and imported through BENP. A
partial numerical companion keeps its own holes in a Common guard roster;
strict Boolean combination with it still inherits those holes. All existing
source/domain requirements and unresolved-cell refusals apply to Event guards.
No probability law or prior is needed to construct or interpret these regions.

Two authored regions that coincide only inside the ambient domain have equal
truth partitions but different derivations. For example, the whole real line
and `[0,1]` both yield true everywhere on `[0,1]`; their BENP identities remain
distinct. Exact disconnected sets, open endpoints and irrational singletons use
the existing BEPR solver. This is a univariate query consumer, not a new solver
or a parameter-changing map.

## Crossing into Events

```rust
let comparison = advantage.where_sign(
    PolynomialSigns::POSITIVE, number_limits, &mut work,
)?;
let refined = comparison.refine(
    presentation_id, &source, event_limits, source_limits, &mut work,
)?;
let cases = refined.events();
let profitable = cases.holds();
let unprofitable = cases.fails();
let unknown = cases.undefined();
let carried_claim = refined.refinement().lift(&claim, work.control())?;
```

The three returned Events are disjoint and cover the complete refined source.
The result retains the predicate and source. The explicit presentation identity
names a deterministic guard extension of the same actual worlds and designated
law. It does not condition, renormalize or bind a prior. Existing Event values
move through the returned ordinary `ParameterRefinement`; exact descent refuses
when an added guard is essential.

True and false distinguish all three cases; undefined is their ambient
complement. A total predicate needs only the true distinction. Already
representable cases add no guard. Exact irrational points and open boundaries
remain separate where their membership differs. Possible zero-mass outcomes
remain actual worlds on every truth region.

`comparison.events(&source, source_limits, work)` uses only existing guards. It
refuses if any region cuts a current logical cell. A parameter predicate must
have exactly the source's named ambient domain: silently applying a narrower
predicate to a wider source would invent truth outside its domain. Fixed
predicates can be interpreted as constant regions on any explicitly supplied
admitted source; their numerical origins remain distinct and retained.
`refine` requires a parameter source. Constant predicates on a finite source use
`events` directly.

These Events fit the existing dependency language:

```rust
relation Decision { game: u64 }
relation Truth { game: u64, case: u64, when: event }
Decision(game) -> Decision;
Decision(game, true) -> Decision;
Truth(game, case) -> Truth;
Truth(game, when) -> Truth;
Truth(game) <= Decision(game);
Decision(game, true) == Truth(game, when);
```

The scalar `case` labels true, false or undefined. The Event key prohibits
overlapping cases; containment requires complete coverage. Omitting a nonempty
undefined region fails admission. Joins and subsequent Event expressions use
these fields normally, including after persistence and owner closure. Extracting
an Event case preserves its mathematical meaning and source; it does not embed
the entire numerical BENP receipt in that Event. Retain the predicate alongside
the cases when the original calculation must remain inspectable.

## Guard queries and explicit transport

Capture a checked source once, then use it inside a query. A plan contains the
complete canonical BEVT source, including its domain and law. `None` requires
every truth region to be expressible in existing cells; `Some(identity)` names
an explicit parameter refinement. The source is independently owned.

```rust
let plan = PredicateGuardPlan::capture(
    &source, Some(presentation_id), codec_limits, &mut work,
)?;
let decisions = query!(Game {
    use guard g = &plan;
    interior observed(game, claim, p: Probability(claim, evidence))
        | Claim(game, claim, evidence);
    interior truth(game, claim, p: Predicate((3*Value(p)-1)/Value(p) > 0))
        | observed(game, claim, p);
    interior cases(game, p,
        yes: Guard(Holds(p, g)),
        no: Guard(Fails(p, g)),
        unknown: Guard(Undefined(p, g)),
        held: Guard(Lift(p, g, claim)))
        | truth(game, claim, p);
    (game, p, yes, no, unknown,
        profitableClaim: Event(yes & held),
        original: Guard(Descend(p, g, held)))
        | cases(game, p, yes, no, unknown, held);
});
```

The predicate argument can also be an inline expression, such as
`Guard(Holds(Value(p) > 1/2, g))`. All five heads return ordinary `event` fields.
Later stages can apply Event algebra, identity joins and grouped `Pack`; returned
values can enter the existing pointwise keys and containment laws. No schema
field type or serialized value format is added. Cases and transport must resolve
the same predicate roster. Reusing an identity alone does not silently construct
a common refinement; the explicit `Common` operation below does that work.

`Lift` checks an Event in the original source and transports it into the guard
presentation. `Descend` checks an Event in that presentation and returns it only
if the original presentation can express it exactly. Existing-source plans make
both operations checked alignment. Context faults retain stage, every written
rule, find position and transport input occurrence, with complete canonical
expected/offending values. Undefined truth produces a region; unresolved cells,
domain mismatch and essential-guard descent are errors. A downstream filter
cannot turn a participating producer error into an empty successful result.

```ts
const plan = GuardPlan.refine(presentationId, fullSource)
const cases = query(Theory).rule((r) => {
    const t = v(truth)
    return r.match(truth, t).find({
        game: t.game,
        predicate: t.verdict,
        yes: Guard.holds(t.verdict, plan),
        no: Guard.fails(t.verdict, plan),
        unknown: Guard.undefined(t.verdict, plan),
        claim: Guard.lift(t.verdict, plan, t.claim)
    })
})
```

`GuardPlan.existing(fullSource)` uses existing cells. Plan authoring is pure and
copies the explicit 32-byte identity; the immutable source already owns its bytes.
Workers admit the complete source before query preparation, even in unreachable
rules. A valid proper-region Event is not a valid full source plan. SDK types
require predicate operands and Event transport variables; native validation
rechecks both. Descriptions replay through the ordinary `event` output field.

The raw head is `{kind: "guard", expr: {kind, predicate, plan}}`, with `input`
for `lift`/`descend`. A plan is `{kind: "existing", source: fullBEVT}` or
`{kind: "refine", source: fullBEVT, identity: bytes32}`. Every object has exact
fields. The outer guard and its predicate/numerical children share the expression
shape limit; source/identity/import bytes share admission limits, and all exact
arithmetic shares the execution's observation budget.

Plans currently capture one static source. Dynamic per-row source naming,
multiple-source query refinement remain
separate obligations. No implicit source identity, prior or independence is
introduced by this query API.

## Common truth presentations

Different comparisons can resolve different parts of the same source. Use an
explicit common roster to make their Event cases composable:

```rust
interior cases(game, low, high, known,
    a: Guard(Holds(low, Common(g, high, known))),
    b: Guard(Holds(high, Common(g, known, low))),
    hole: Guard(Undefined(known, Common(g, low, high))),
    held: Guard(Lift(low, Common(g, high, known), claim)))
    | truth(game, claim, low, high, known);
(game, both: Event(a & b), middle: Event(!(a | b)),
    original: Guard(Descend(known, Common(g, high, low), held)))
    | cases(game, low, high, known, a, b, hole, held);
```

The primary predicate joins the nonempty `Common` roster automatically. Every
head above therefore resolves `{low, high, known}`. Reordering or duplicating
the roster preserves the common presentation's canonical bytes, within admitted
resource bounds. With `low = p < 1/2`, `high = p > 1/2`, and `known = p/p > 0`,
`both` is empty, `middle` is exactly the half-point, and `hole` is exactly zero.
At zero, `low` is still true. Common refinement keeps each truth separately;
it is not strict conjunction and does not spread one predicate's undefinedness
to another. All Events still refer to one actual parameter assignment.

In TypeScript, create one owned authoring context and reuse it:

```ts
const context = Guard.common(plan, [t.low, t.high, t.known])
const a = Guard.holds(t.low, context)
const b = Guard.holds(t.high, context)
const hole = Guard.undefined(t.known, context)
const held = Guard.lift(t.low, context, t.claim)
```

This context is pure query data, not a new schema type. It owns its roster,
retains every variable demand, and shares the complete head's node/depth/byte
budget. Raw wire adds a nonempty `resolve: [predicateIR, ...]` field to the guard
expression; malformed or unbound companions refuse even when the primary is
constant. Existing-source plans require all companions to be expressible already.
Named refinement plans sort canonical BEPR truth regions, remove duplicate regions,
skip those already expressible, and build one checked `ParameterRefinement`.
Every written predicate is evaluated and its exact domain checked; normalization
does not authorize skipping a bad operand. A later filter cannot hide a refusal.

The host `PredicateRefinement::common(identity, source, predicates, ...)` returns
one `PredicateRefinement` per input position, each retaining its own numerical
origin and sharing the common presentation. The roster must be nonempty and
the source must have parameters. Its temporary canonical guard bytes share the
parameter codec byte limit; roster extent is bounded by source work limits,
alongside shared arithmetic and ordinary graph/source capacities.

Common canonical ordering does not rewrite existing Event values or singleton
guard presentations. Use the same common operation/roster for construction and
transport. This operation refines a captured source for query-bound predicates;
it does not combine different named parameter domains or build a joint law from
separate probability estimates.

## Portable replay and limits

`ObservationPredicateImport::capture` and `from_bytes` reconstruct independent
owned observations and replay every arithmetic, sign and Boolean operation.
BENP v1/v2 contains no trusted truth partition. Equality and hashing use complete
encoded derivation identity, independent of Arc sharing and allocation order.
Double negation can preserve the truth partition while remaining a different
written derivation.

The prefix `BENP` and a version byte are followed by predicate tags. Trees
containing a Region node require version `2`; every other tree retains version
`1` and its original bytes. Version `1` refuses tag 4; version `2` without any
Region node is noncanonical. Unknown versions refuse.

| Tag | Payload |
| --- | --- |
| 0 | Sign-mask byte, then an embedded BENO numerical node (without its envelope) |
| 1 | Negated predicate |
| 2 | `BoolOp4` byte, then left and right predicates |
| 3 | Length-prefixed BEPR inhabited domain, then restricted predicate |
| 4 (v2) | Length-prefixed BEPR inhabited domain, then length-prefixed BEPR region |

All BENP blob lengths are little-endian u32, as in embedded BENO nodes.

One `ObservationNumberCodecLimits` budget counts predicate and numerical nodes
together. Its descriptor byte/item bounds cover the complete stream, including
embedded observation payloads. Numerical leaf codecs retain their source limits.
Every exact operation uses the caller's shared arithmetic counter. Both tree
walks use explicit stacks; combined depth has a hard ceiling of 256, including
numeric nodes below a Sign. Canonical reencoding rejects presentation aliases.
These are admission limits, not a complete aggregate retained-memory policy.

## Evidence and scope

Native tests cover every fixed partial truth pair under all sixteen operations,
all sign masks, negation, portable identity, source distinction, domain
restriction/refusal, disjoint endpoint holes, exact irrational boundaries,
guard lifting, repeated refinement, zero-mass worlds, malformed streams,
combined numerical/predicate depth and size, cancellation and shared work.
Replay rechecks original numerical leaves even when their final values cancel.

`event_predicate_guards` runs query arithmetic on both Free Join paths, retains
and replays its comparison after owner closure, persists all three Event cases,
checks missing/overlapping partition refusals, then queries the stored regions
after reopening. Its first test is a host-mediated consumer; additional tests
execute the five guard heads directly, check fixed and parameter sources, inline
predicates, existing resolved cells, source/type refusals, complete transport
faults, exact descent, repeated prepared execution and owner closure.

Twenty-nine `PredicateGuards.lean` reports establish exact truth coding,
world-preserving refinement, Boolean lifting, finite contraction preservation,
zero-weight possibility retention and the old-cell factorization requirement.
The query laws include disjoint complete cases, reconstructing a claim from
its three intersections, and exact descent iff membership is constant on old
cells, with uniqueness on legal cells.
Common-roster laws identify its exact joint observation cells, establish
permutation/duplication invariance and prove that any presentation resolving
all the predicates refines this common partition. They do not prove that the
native constructor chooses the fewest physical guard coordinates.
They do not verify the Rust solver, codec, allocator or query engine.

`event_query_predicates` additionally checks both Free Join paths, identity
joins/antijoins, mixed observations, rebindings and producer refusals. Internal
sink/finalization tests force spills, exhaust shared work and roll back a newly
copied predicate after a later row fails. SDK and raw Node tests cover portable
replay, exact parameter holes, explicit quantifiers, strict grammar and combined
bounds. Six macro fixtures pin deliberate syntax refusals.

Twenty `PredicateQueries.lean` reports establish strict definedness, quantified
truth with inhabited domains, shared-witness conjunction and counterexamples to
quantifying operands independently or treating missing false evidence as truth.
Six region laws add totality, existential intersection, universal inclusion,
complement, Boolean composition and the distinction between domain-relative
truth and complete authored-region identity.
The generic faithful-encoding grouping laws in `NumberQueries.lean` also apply
to complete predicate derivations; encoding faithfulness remains a premise.

Guard sink tests additionally force spill, retain empty Events, exhaust the
shared arithmetic budget and recover after cancellation. Five macro fixtures
pin guard-specific syntax refusals. SDK tests execute parameter refinements and
description replay after owner closure; raw Node tests independently execute
fixed-source cases/transport and reject malformed plans and combined shapes.

Host/native/SDK tests also check common-roster permutations and duplication,
retained numerical origins, joint emptiness and exact boundary membership,
independent undefined cases, common lift/descent, shape/byte/work limits and
hidden unresolved companions. The macro refusal roster contains 62 fixtures.

Dynamic/multiple-source query refinement,
multivariate solving and every other open M0–M8 gate remain required.
No release, tag or version bump.
