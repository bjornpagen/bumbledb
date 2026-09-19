# Owned numerical predicates and explicit Event guards

`ObservationPredicate` retains an exact true/false/undefined partition and the
entire calculation that produced it. Fixed observations yield an optional truth
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
Negate, Binary and OnDomain nodes. Every written operand remains reachable,
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
`or`, `xor`, `not`, `imported` and `onDomain` construct owned programs.
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

## Portable replay and limits

`ObservationPredicateImport::capture` and `from_bytes` reconstruct independent
owned observations and replay every arithmetic, sign and Boolean operation.
BENP v1 contains no trusted truth partition. Equality and hashing use complete
encoded derivation identity, independent of Arc sharing and allocation order.
Double negation can preserve the truth partition while remaining a different
written derivation.

The prefix `BENP` and version byte `1` are followed by predicate tags:

| Tag | Payload |
| --- | --- |
| 0 | Sign-mask byte, then an embedded BENO numerical node (without its envelope) |
| 1 | Negated predicate |
| 2 | `BoolOp4` byte, then left and right predicates |
| 3 | Length-prefixed BEPR inhabited domain, then restricted predicate |

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
after reopening. This is a host-mediated consumer, not a predicate query opcode.

Nineteen `PredicateGuards.lean` reports establish exact truth coding,
world-preserving refinement, Boolean lifting, finite contraction preservation,
zero-weight possibility retention and the old-cell factorization requirement.
They do not verify the Rust solver, codec, allocator or query engine.

`event_query_predicates` additionally checks both Free Join paths, identity
joins/antijoins, mixed observations, rebindings and producer refusals. Internal
sink/finalization tests force spills, exhaust shared work and roll back a newly
copied predicate after a later row fails. SDK and raw Node tests cover portable
replay, exact parameter holes, explicit quantifiers, strict grammar and combined
bounds. Six macro fixtures pin deliberate syntax refusals.

Fourteen `PredicateQueries.lean` reports establish strict definedness, quantified
truth with inhabited domains, shared-witness conjunction and counterexamples to
quantifying operands independently or treating missing false evidence as truth.
The generic faithful-encoding grouping laws in `NumberQueries.lean` also apply
to complete predicate derivations; encoding faithfulness remains a premise.

Query construction of Event guards/refinements, parameter-region query consumers,
multivariate solving and every other open M0–M8 gate remain required.
No release, tag or version bump.
