# Live shared-parameter families in the TypeScript SDK

The SDK can now author shared-parameter Event sources, designate exact normalized
laws, calculate owned probability/expectation functions, and refine the guard
presentation explicitly. The resulting Events are ordinary `event` values in
schemas, keys, containments, storage and Free Join. No probability or weight column
is introduced. No prior over the shared unknown parameter is inferred.

```ts
import { Effect } from "effect"
import {
  Event, ExactPolynomial as P, ExactRational as Q, FamilyFunction as F,
  ParameterDomain as D, ParameterRegion as R, PolynomialSigns as S,
  ParameterFunction as PF, ParameterRefinement as RF
} from "@bjornpagen/bumbledb"

const authored = Effect.gen(function* () {
  const name = new Uint8Array(32).fill(3)
  const p = yield* P.parameter(name)
  const one = yield* P.constant(yield* Q.fraction(1n))
  const tail = yield* P.subtract(one, p)
  const allowed = yield* R.and(
    yield* R.whereSign(name, p, S.nonNegative),
    yield* R.whereSign(name, tail, S.nonNegative)
  )
  const domain = yield* D.new(allowed)
  const raw = yield* Event.space(new Uint8Array(32).fill(4), 1n)
  const scope = yield* Event.withParameters(raw, domain, [])
  const yes = yield* Event.coordinate(scope, 0n)
  const source = yield* F.designate(yield* F.new(scope, [
    { region: yes, value: { numerator: p, denominator: one, defined: allowed } },
    { region: yield* Event.complement(yes),
      value: { numerator: tail, denominator: one, defined: allowed } }
  ]))
  const event = yield* Event.coordinate(source, 0n)
  const observation = yield* Event.parameterProbability(event, source)
  // observation.value is the exact function p on [0,1].
  const self = yield* Event.parameterProbability(event, event)
  // self.value is one on (0,1], and undefined at zero.

  const half = yield* PF.ratio(domain,
    yield* P.constant(yield* Q.fraction(1n, 2n)), one)
  const threshold = yield* PF.whereSign(
    yield* PF.subtract(observation.value, half), S.positive)
  const refinement = yield* RF.new(new Uint8Array(32).fill(5), source, [threshold])
  const { refined } = yield* RF.describe(refinement)
  const aboveHalf = yield* Event.parameterEvent(refined, threshold)
  const liftedEvent = yield* RF.lift(refinement, event)
  return { source, event, observation, self, refinement, aboveHalf, liftedEvent }
})
```

Designation changes the complete named context, so the example selects the
coordinate again from the measured source. It does not align an unmeasured Event
into a different law by coincidence. Different sampled outcomes, copied Events,
and repeated use of the same unknown parameter remain different operations.

`aboveHalf` is a deterministic parameter predicate. Its conditional mass is zero
at p=1/4 and one at p=3/4. It has no unconditional mass of one-half unless a prior
and integration operation are supplied separately. `liftedEvent` preserves the
original actual worlds and law; refinement adds expressive distinctions, not
new randomness.

## Source and observation operations

| Operation | Result and contract |
| --- | --- |
| `Event.withParameters(full, domain, guards)` | Reinterpret declared coordinates as deterministic guards of one parameter; retain all feasible domain cases |
| `Event.describeParameters(event)` | Owned inhabited domain, `{ coordinate, region }` guards and `outcomeCoordinates` bitmask |
| `Event.parameterEvent(context, predicate)` | Ordinary Event when the predicate is exactly representable; otherwise refinement is required |
| `Event.parameterMass(event)` | Exact `ParameterFunction` after summing finite outcomes at the same parameter |
| `Event.parameterProbability(event, given)` | Owned source Events, original mass functions, partial conditional value and defined region |
| `FiniteFunction.parameterExpectation(function, given)` | Same observation contract for signed finite rational payoffs |
| `FamilyFunction.expectation(function, given)` | Same contract for parameter-dependent signed payoffs |
| `Event.containsParameter(event, world)` | Exact membership at a rational/algebraic parameter and finite outcome code |
| `Event.parameterWitness(event)` | Owned actual world, or `null` exactly for an empty Event |
| `Event.worldCardinality(event)` | `{ kind: "finite", count }` or `{ kind: "continuum" }`, distinct from `atomCount` |

`withParameters` requires an unmeasured full space marker that is not already
parameterized. It accepts restricted structural support, provided every allowed
parameter still has a legal outcome fibre. It refuses missing fibres instead of
silently deleting parameter cases. Each guard uses an existing distinct coordinate;
remaining coordinate positions are outcome bits. The guards capture the same
named parameter as the domain.

A `ParameterWorld` is `{ parameter: RealWitness, outcomes: bigint }`. Outcome
bits occupy their original coordinate positions; supplied guard bits, out-of-range
codes, illegal structural support and out-of-domain parameters refuse. Guard bits
are derived from the exact parameter. A false membership result denotes a legal
world outside the particular Event. `FamilyFunction.at` separately returns `null`
for an out-of-domain rational argument, retaining its native function contract.

A `ParameterProbabilityObservation` has `kind: "probability"`, `event`, `given`,
`numerator`, `evidenceMass`, `value`, and `defined`. The three numeric fields are
owned `ParameterFunction` values; `defined` is an exact `ParameterRegion`.
`ParameterExpectationObservation<A>` instead has `kind: "expectation"` and owns
its input `function` of type `A`. Every original numerator and denominator is
retained. Zero evidence makes `value` nowhere defined on that region; it never
turns into zero. Operational failure remains an error. These host results now also have final
[Probability](event-query-probability.md) and [Expectation](event-query-expectation.md)
query counterparts. Observation staging/arithmetic remains open.

## Total family functions and explicit refinement

`FamilyFunction` is a total signed function on actual `(parameter, outcome)`
worlds. `new(full, pieces)` takes disjoint `{ region, value }` pieces. Each value
uses the existing `ParameterFunctionPiece` shape `{ numerator, denominator,
defined }`, with the source's ambient domain. It must be defined wherever its
Event cell is active; holes outside that cell are allowed. Omitted worlds have
zero value, and explicit zero definitions remain inspectable. Invalid operands
are checked even in empty cells.

`fromParameter(full, partial)` requires totality on the source domain and
representable piece boundaries. `fromFinite` embeds an existing finite payoff.
`density` captures the designated source law, and `designate` validates
nonnegativity and exact unit mass in **every** fibre. It never rescales an invalid
function or deletes zero-mass possibilities.

The public algebra includes `add`, `subtract`, `multiply`, `divide`, `equivalent`,
`isNonnegative`, `whereSign`, `at`, `alignTo`, `refine`, `pullback`, `pushforward`,
`descend`, `outcomeSum` and `expectation`. Total division refuses any zero divisor
at a legal world, including a zero-prior world. `outcomeSum` needs no law;
expectation multiplies by the law before contracting. Weighted images count all
finite witnesses at one shared parameter. Descent requires an onto map and exact
constancy on its fibres; an average alone is not a descent certificate.

`describe` returns `{ space, pieces }` with owned Event cells and guarded
expressions, reconstructible with `new`. `fromBytes`, `toBytes`,
`isFamilyFunction` and worker-backed `validate` provide BESC v2 role 1 transport.
Numerical equivalence uses the native algebra, not descriptor byte identity.

`ParameterRefinement.new(identity, full, predicates)` appends deterministic guards
under an explicit new presentation identity. `describe` owns `source` and `refined`
full Events; `lift` preserves old Events, and `descend` refuses an Event for which
the added guards are essential. `FamilyFunction.refine` lifts its function through
the same checked change. Pure transport, ownership checking and `validate` use
BESC v2 role 3. Common refinement rosters, domain restrictions, family channels
and update receipts are native capabilities still awaiting their SDK surfaces.

## Ownership, limits and evidence

This extends the existing parameter worker endpoint. One native arithmetic budget
spans all nested BEVT/BESC/BEPR/BEPL admission, algebra and function serialization.
BEVT output reuses its admitted canonical context/law and encodes the Event graph
under the same cancellation control. JS performs shape/copy/ownership work only.
Inputs and structured output blob rosters each have a combined 16 MiB bound;
guards/refinement predicates have at most 62 entries, subject to the source's
remaining coordinate capacity. Authored family functions have at most 2,046
pieces under the existing 4,096-item descriptor policy. Native solver, function,
descriptor and traversal bounds remain in force. JS object overhead and retained
generation memory remain outside these operation limits.

All carriers and observations survive runtime release. Pure construction loads
no addon; malformed bodies are admitted only on workers, and cancellation drains
completed outputs before delivery. BESC roles check version and kind together.
No Event or source wire format changes.

Nine SDK tests cover native family construction, exact normalization, actual
worlds, signed functions, descriptions, weighted maps/descent, guard-local holes,
refinement and original-law preservation, malformed/unused inputs, input copying
and late cancellation. A stored source satisfies ordinary Event keys and
containments, then joins after reopen and retains conditional probability and
signed expectations after every database/runtime owner closes. Another case
normalizes 61 symbolic outcome bits plus a deterministic guard without enumerating
worlds or assigning probability to guard cases. A native bridge test checks shared
nested admission budgets, complete owned observation components and cancellation.

The existing [family](event-family-functions.md), [parameter source](event-parameter-sources.md)
and [refinement](event-parameter-refinement.md) reference proofs retain their
explicit mathematical premises. This boundary adds integration evidence rather
than a Rust refinement or performance claim. M0–M8 remains the full target;
general solving, prior integration, TypeSafe imports, query observations,
retained-memory policy and remaining qualification work are not closed here.
