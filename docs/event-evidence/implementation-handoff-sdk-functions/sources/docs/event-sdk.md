# TypeScript Event values, host algebra and query programs

This implements structural SDK slices of M2/M5 and fixed-law slices of M6/M7. `event` is an ordinary,
unparameterized field descriptor. An `Event` carries a named space, its legal
support, a region and any finite law encoded by BEVT. Probability is neither its
identity nor a schema weight column. The native Event core remains the algebra
and admission authority.

```ts
import { Effect } from "effect"
import { Event, event, key, mirrors, on, query, relation, schema, u64, v } from "@bjornpagen/bumbledb"

const Parent = relation("Parent", { group: u64, condition: event })
const Child = relation("Child", { group: u64, item: u64, condition: event })
const Partitions = schema("Partitions", { Parent, Child }, [
  key(Parent, ["group"]),
  key(Parent, ["group", "condition"]),
  key(Child, ["group", "condition"]),
  mirrors(on(Parent, ["group", "condition"]), on(Child, ["group", "condition"]))
])

const partition = Effect.gen(function* () {
  // The application supplies a stable 32-byte identity for this named space.
  const full = yield* Event.space(new Uint8Array(32).fill(193), 2n)
  const a = yield* Event.coordinate(full, 0n)
  const b = yield* Event.complement(a)
  return {
    parent: { group: 1n, condition: full },
    children: [{ group: 1n, item: 1n, condition: a }, { group: 1n, item: 2n, condition: b }]
  }
})

const covered = query(Partitions).rule(r => {
  const row = v(Child)
  return r.match(Child, row).find({ group: row.group, condition: r.pack(row.condition) })
})
```

Run effects through the ordinary `NativeRuntime.layer` and insert the returned
facts through `ChangeSet.builder`. These are the native dependency meanings:
the Child key rejects overlap between distinct child facts in one group; the
mirror requires the parent region to equal the union of child regions. Deleting
one nonempty child leaves a coverage violation. Inserting a second fact with
the same nonempty region and a different item violates the pointwise key.
Scalar grouping and scalar keys keep their usual meanings.

`Pack` returns one union per scalar group. No contributors means no result row;
there is no invented context for an empty group. Stored values, query literals,
single and set parameters, equality joins, prepared queries, description import,
row codecs and generated schema bindings use the same `event` field. Event
ordering and numeric folds are refused. Set inclusion is `Event.subset` or
`Event.includes`, not a numeric comparison.

## Owned transport and native admission

`Event.fromBytes(bytes)` is a pure `Result<Event, DbError>`. It accepts a bounded
BEVT v1/v2 envelope, checks its header and owns a copy. It **does not certify** a
decision graph, law, context or canonical encoding. Even a header-only envelope
can become a carrier; it cannot enter native algebra or storage successfully.
`Event.validate(value)` runs the canonical native decoder and returns the checked
encoding. Pure schema authoring and pure transport decoding never load the addon.

Values are frozen opaque objects backed by private owned bytes. `Event.isEvent`
checks actual ownership; spreading or reflecting a wrapper cannot forge it.
`Event.toBytes` returns a copy. Values remain usable after their query, database
and runtime close. JavaScript identity or generic deep equality does not decide
region equality: use native `Event.equal`. Explicit byte transport is the
boundary between separate package instances or realms.

The JSON field codec uses exactly `{"$event":"<lowercase BEVT hex>"}`. Unknown
keys, uppercase/nonhex encodings, detached input and SharedArrayBuffer backing
refuse. The application-facing JSON codec and the log grammar's `{"event":...}`
spelling are separate contracts. The binary BEVT contracts do not change.

## Algebra currently exposed

All operations below return lazy Effects requiring `NativeRuntime`. The new
`runtimeEvent` endpoint admits work through the existing bounded executor,
copies input bytes, checks cancellation, decodes with the shared core, and owns
its result until delivery or cancellation drains it. Every operand is decoded
and aligned before constant, empty or full shortcuts.

| Operation | Meaning |
| --- | --- |
| `space(identity, coordinates)` | Full unmeasured named space, 0–62 binary coordinates |
| `coordinate(context, index)` | One coordinate predicate in the context's legal support |
| `full`, `empty`, `complement` | Support-relative constants and complement |
| `restrict(context, support)` | Full region of an unmeasured structurally restricted context |
| `apply(mask, a, b)` | Any of the 16 binary Boolean functions; bit `(a << 1) \| b` |
| `and`, `or`, `xor`, `difference`, `implies`, `equivalence` | Named Boolean functions returning Events |
| `ite(condition, high, low)` | Conditional region construction |
| `count`, `signature` | Legal-world cardinality and four-cell nonemptiness mask, as bigint |
| `isEmpty`, `isFull`, `subset`, `includes`, `equal`, `disjoint`, `contains` | Structural predicates returning Boolean values |

Restriction is structural, not Bayesian conditioning. Existing BEVT v2 carriers
retain finite laws through operations that preserve their context. This slice
does not add a second JavaScript arithmetic or decision-diagram implementation.

Each SDK Event carrier and standalone operation input/output has a 16 MiB encoded
limit. This is a per-value limit, **not** an aggregate retained-memory budget.
Native support, graph and law resource limits still apply. No performance result
or fixed worst-case construction time follows from these limits.

## Query construction and captured descriptors

`EventExpr`, `EventTest` and `RelationExpr` author pure, owned programs in the
existing native query grammar. `Event` remains the host-value and Effect API.
Constructing a query loads no addon and performs no Event arithmetic. For example:

```ts
import { EventExpr, EventTest } from "@bjornpagen/bumbledb"

const Claim = relation("Claim", { player: u64, duke: event, blocked: event })
const Game = schema("Game", { Claim }, [])
const claims = query(Game).rule(r => {
  const claim = v(Claim)
  return r.match(Claim, claim).find({
    player: claim.player,
    unblocked: EventExpr.and(claim.duke, EventExpr.complement(claim.blocked)),
    impossible: EventTest.isEmpty(claim.duke)
  })
})
const covered = query(Game).rule(r => {
  const row = v(claims)
  return r.match(claims, row).find({ region: r.pack(row.unblocked) })
})
```

The inferred output fields are Event and Boolean. A computed region can feed
another query stage, ordinary matching, Pack, storage, or another Event program.
Tests describe whole legal regions. They can filter a later stage using ordinary
Boolean equality. `describeQuery` and `queryFromDescription` preserve these
programs, including captured descriptors and nested relation operations.

| Builder | Operators |
| --- | --- |
| `EventExpr` | `variable`, `empty`, `full`, `complement`, all sixteen truth functions via `apply`, named `and`/`or`/`xor`/`difference`/`implies`/`equivalence`, `ite` |
| Event cardinality | `cardinality(minimum, maximum, ...events)`, `atLeast`, `atMost`, `exactly`; bounds are u64 BigInts and rosters are nonempty |
| Captured readouts | `pullback`, `image`, `universalImage`, `nonvacuousImage`, `possible`, `guaranteed`, each taking `(event, descriptor)` |
| Relation views | `region`, `domain`, `range`; `may`, `all`, `must`, `post` take `(relation, event)` |
| `EventTest` | `isEmpty`, `isFull`, `subset`, `equal`, `disjoint`, `covers` |
| `RelationExpr` | `bind(event, faces)`, `test(event, faces)`, `identity(faces)`, `complement`, `converse`, `apply` and named truth functions, `compose`/`leftResidual`/`rightResidual(left, right, product)`, `star(relation, product)` |

`EventExpr.apply` and `RelationExpr.apply` take a number mask in 0–15, matching
the query IR. The host `Event.apply` takes a bigint. `empty` and `full` anchor to
a body-bound Event variable. Repeated cardinality positions retain their meaning:
`exactly(1n, a, a)` is empty. Every variable must have a positive body binding,
including ignored truth-function operands and both conditional branches. Native
context admission still happens before shortcuts. Empty results remain rows;
an absent body binding produces no row. Relation views require explicit roles.

`EventDescriptor.fromBytes(bytes)` returns a pure `Result` with an immutable,
owned BEDC v1 envelope. It checks the envelope and 16 MiB bound, not the
mathematics. `toBytes` returns a copy. Query preparation reconstructs and certifies
the actual map/face/product on its worker, including on an empty body.
`EventDescriptor.admit`, `describe` and `inspect` now construct and inspect all
seven structural BEDC kinds through that same native core.

Programs expose a frozen structural `node` with variable references and owned
descriptor carriers. The same generic IR grammar represents builder and wire
trees; one structural translation assigns variable ordinals and copies bytes.
There are no per-row JavaScript callbacks or independent algebra evaluators.
Whole-query snapshots copy binders and expression references together. Spreading
a wrapper cannot forge a checked program; portable descriptions use the checked
description parser. Builders count written occurrences, including reused subtrees,
against the existing 128-level, 4,096-node and 16 MiB import-payload limits.
These bounds do not account for total retained native memory.

## Constructing and inspecting structural descriptors

`EventDescriptor.admit(description)` returns an Effect of an owned, canonical
BEDC carrier. The plain description contains Events, ordered readouts and explicit
32-byte identities. Source and target must be **full space markers**: a proper
region is refused, never silently widened. No application-authored certificate
is trusted. The worker reconstructs total maps, onto maps, shared environments,
complete products, endpoint roles and complete squares.

`describe(descriptor)` re-admits the carrier and returns a canonical
`EventDescriptorDescription`. `inspect(descriptor)` re-admits it and returns an
`EventDescriptorInspection` with useful derived spaces and descriptors. A caller
can edit a description by building a new record and admitting it again; mutation
of an exported identity buffer cannot change the original carrier. Inspection
results also retain no dependency on a native owner or runtime.

| Description kind | Input | Inspection result |
| --- | --- | --- |
| `map` | `{ map: { source, target, readouts } }` | Source, target, ordered readouts |
| `surjective` | Same map data; admission additionally proves onto | Same fields, tagged `surjective` |
| `faces` | `{ identity, environments: [map, ...] }` | Full `space`, onto `projections` and `environments` |
| `fibre` | `{ product: { identity, left, right, reversed } }` | Full `space`, onto `left`/`right` projections and `leftEnvironment`/`rightEnvironment` maps |
| `relation` | `{ product, region }` | Owned `region`, product descriptor, full `input`/`output` spaces |
| `composition` | `{ identity, st, tu, su }` with three product descriptions | Full `workspace`, three `products` and onto `projections`, both in ST/TU/SU order |
| `square` | `{ product, left, right }` with two maps from a common source | Product descriptor, certified onto `joint`, left/right map descriptors |

Each row also requires its `kind` field. Map readouts follow target semantic
coordinate order and may be arbitrary source Events. Fibre `left` and `right`
are environment maps in original coordinate order. `reversed: true` exchanges
endpoint roles while preserving stored coordinates and support. It does not
invent another space. Products impose shared-environment agreement, so they can
retain nonrectangular supports and correlations. A square whose individual maps
are onto may still fail: the joint map must cover every legal pair.

```ts
import { EventDescriptor, EventExpr, RelationExpr } from "@bjornpagen/bumbledb"

const descriptors = Effect.gen(function* () {
  const states = yield* Event.space(new Uint8Array(32).fill(211), 1n)
  const environment = yield* Event.space(new Uint8Array(32).fill(212), 0n)
  const toEnvironment = { source: states, target: environment, readouts: [] }
  const product = {
    identity: new Uint8Array(32).fill(213),
    left: toEnvironment, right: toEnvironment, reversed: false
  }
  const pairs = yield* EventDescriptor.admit({ kind: "fibre", product })
  const plan = yield* EventDescriptor.admit({
    kind: "composition", identity: new Uint8Array(32).fill(214),
    st: product, tu: product, su: product
  })
  const view = yield* EventDescriptor.inspect(pairs)
  if (view.kind !== "fibre") throw new Error("expected a fibre product")

  const Move = relation("Move", { allowed: event })
  const Game = schema("Moves", { Move }, [])
  const reachable = query(Game).rule(r => {
    const move = v(Move)
    const edges = RelationExpr.bind(move.allowed, pairs)
    return r.match(Move, move).find({
      reachable: EventExpr.region(RelationExpr.star(edges, plan)),
      enabled: EventExpr.image(move.allowed, view.left)
    })
  })
  return { Game, Move, reachable, pairSpace: view.space }
})
```

Here `Move.allowed` must be an Event on the returned pair space. The query
computes reachability and enabled states using the exact admitted projection;
applications do not encode BEDC or reconstruct product supports themselves.
All seven construction paths are data descriptions, not JavaScript callbacks.

Effects copy input when executed. Pure imports, carrier operations and Effect
construction remain addon-free. The SDK rejects unknown fields, accessors,
sparse lists and shared/detached byte backing. The native bridge independently
copies intrinsic-checked byte views before registering worker mathematics.
Admission/description uses the core's 4,096-item and 16 MiB descriptor limits,
including the final encoded envelope. Inspection separately bounds its combined
encoded payload at 16 MiB and 4,096 output items (root, lists and byte leaves);
each derived descriptor also obeys the core descriptor limits. Repeated embedded
spaces count repeatedly. These are transport bounds, not total allocator or
retained-memory quotas. Encoding finishes on the worker; delivery transfers
owned byte payloads. Cancellation drops unpublished descriptions and inspections.
Structural maps preserve endpoint law descriptors without claiming a map
preserves a measure. BESC source construction is a separate remaining SDK gate.

## Exact scalars, finite functions and source observations

`ExactRational` is an owned BERA carrier. `fraction(numerator, denominator)`
accepts bigint or signed integer text; `decimal(text)` preserves authored decimal
meaning; `binary64(number)` preserves the exact IEEE value. Thus decimal `0.1`
is `1/10`, while binary64 `0.1` is `3602879701896397/36028797018963968`.
All construction, arithmetic, comparison and formatting run on native workers.
`add`, `subtract`, `multiply`, `divide`, `equal`, `compare`, `isZero`,
`isNegative`, `isProbability` and `toString` return Effects. `fromBytes`,
`toBytes` and `isExactRational` are pure transport operations. Envelope recognition
does not certify a reduced rational; `validate` invokes canonical native admission.
Division by zero and exhausted exact-arithmetic limits refuse, never round.

`FiniteFunction` is the owned BESC function role, a host object rather than a
schema field. Its operations expose the native [finite function algebra](event-functions.md):

| Operation | Contract |
| --- | --- |
| `new(space, pieces)` | Full context plus disjoint `{ region, value }` cells; unspecified worlds explicitly have value zero |
| `constant(space, rational)`, `density(space)` | Constant function, or the space's designated finite law |
| `add`, `multiply` | Exact signed pointwise algebra; addition sums overlapping contributions |
| `pullback(function, map)`, `pushforward(function, map)` | Substitute readouts, or sum every legal source world in each fibre exactly once |
| `alignTo(function, space)` | Require the same complete designated context; this is not a law revision |
| `equivalent`, `isZero`, `isNonnegative`, `at(function, world)` | Native inspection and exact values |
| `describe(function)` | Canonical owned full space and equal-value pieces, reusable with `new` |
| `designate(function)` | Require a nonnegative density with total mass exactly one; return the full measured Event |
| `expectation(function, given)` | Owned exact signed conditional observation |
| `fromBytes`, `toBytes`, `isFiniteFunction`, `validate` | Pure owned transport and explicit native BESC admission |

Every supplied function cell participates before zero/empty simplification.
Even overlapping zero cells are invalid. Designation never normalizes a malformed
law by dividing by its sum. A cell's density is per legal world in that region,
not a mass to spread implicitly across it. Zero-mass worlds remain possible.
No independence is inferred from readouts, separate names or scalar marginals.

```ts
import { ExactRational, FiniteFunction } from "@bjornpagen/bumbledb"

const correlated = Effect.gen(function* () {
  const raw = yield* Event.space(new Uint8Array(32).fill(230), 2n)
  const a = yield* Event.coordinate(raw, 0n)
  const b = yield* Event.coordinate(raw, 1n)
  const density = yield* FiniteFunction.new(raw, [{
    region: yield* Event.equivalence(a, b),
    value: yield* ExactRational.fraction(1n, 2n)
  }])
  const measured = yield* FiniteFunction.designate(density)
  const target = yield* Event.coordinate(measured, 0n)
  const given = yield* Event.coordinate(measured, 1n)
  return yield* Event.probability(target, given)
})
```

The result retains `event`, `given`, `numerator` (`1/2`), `evidenceMass` (`1/2`)
and `value` (`1`), all owned. `Event.mass(event)` returns an exact scalar.
`Event.probability(event, given)` returns `ProbabilityObservation`;
`FiniteFunction.expectation(function, given)` returns `ExpectationObservation`,
retaining the original function and evidence as well as numerator, evidence mass
and signed value. Both have `value: null` when evidence mass is zero, including
nonempty evidence. Missing laws and foreign contexts are errors. The observation
and its inputs survive database/runtime release and may be inspected with a later
runtime. No database observation field or native query observation slot is added.

These are fixed rational observations. A future shared-parameter observation
must retain its explicit function/domain; this scalar value is not a substitute.
Likewise, a total host function's explicit zero default does not authorize treating
missing query-observable rows as zeros. The expectation aggregate still needs
its independent equal-value union, disjointness and coverage gate.

Source operations bound input transport at 16 MiB / 4,096 operands. Descriptions
and BESC outputs obey the core descriptor limits; observations additionally bound
their **combined** encoded input/evidence/scalar payload at 16 MiB. One native
arithmetic budget covers every embedded law/scalar and the complete operation,
including measured map endpoints/readouts. `Event::from_bytes_with_arithmetic`
and `MapDescriptor::admit_with_arithmetic` expose the same cumulative admission
contract to Rust callers. Native tests prove that two individually admissible
measured inputs can exhaust the shared budget together. These are work/transport
bounds, not aggregate allocator or retained-memory quotas.

## Inspecting a failed Event stage

For a complete participating-context failure, `DbError.reason` has `_tag: "Engine"`,
`kind: "event"` and an `eventFaults` array of `EventOperandFault` records. Each
record retains the logical `rule`, `find`, syntactic `operand`, `variable`, and
`category` (`SpaceMismatch`). `stage` is absent for main and is an interior ordinal
otherwise. `expectedSpace` and `offendingValue` are owned canonical BEVT bytes;
use `Event.fromBytes` to inspect them through the ordinary Event API.

The array is the core's complete, sorted, duplicate-free set for the first failing
stage. Repeated written rules retain their own coordinates. Ignored Boolean
operands, empty/full values and repeated syntactic occurrences still participate.
One-shot and prepared executions preserve the same logical records. The arrays
and byte payloads remain usable after the query, snapshot and runtime close.
The concise message gives the fault count; applications inspect records for details.

Decoder, resource, cancellation and scalar refusals have their own reason and
carry no `eventFaults` array. They never present a partial set as complete. Native
queries move their already encoded diagnostics into the executor; JavaScript
delivery transfers those bytes without running Event algebra. A failure while
converting the reason cannot publish an incomplete reason object. The core's
existing diagnostic count/payload limits remain; JS object overhead is outside
that accounting.

## Qualification and remaining boundaries

`scripts/check-event-sdk.mjs <compiled addon>` snapshots SDK sources, copies the
addon to a private temporary package, builds JavaScript/declarations and runs
tests there. It neither overwrites a loaded addon nor publishes a package.
Generated bindings are compiled and imported, then compared with the original
native schema identity. Tests also exercise all 16 functions, symbolic 62-bit
support, measured transport, malformed envelopes, foreign empty contexts,
ordinary dependency admission, persistence/reopen, parameters, Pack, retained
results and interruption between native completion and JavaScript delivery.

Supported row, key-read, parameter and query paths copy owned Event/descriptor
bytes on the JavaScript thread and admit their mathematics on cancellable workers.
The private pending query tree contains bytes rather than unchecked Events or
certificates; admission reconstructs the native IR with the shared decoder.
No JavaScript values or borrowed backing cross this stage. Intrinsic backing
checks reject shared/detached byte views before copying, including views whose
`.buffer` property is shadowed. Each Event leaf retains the 16 MiB encoded limit;
descriptor-bearing query heads retain their existing aggregate byte/shape limits.

Shape errors remain `InvalidArgument`. A well-shaped but mathematically invalid
Event or descriptor produces an operation whose result refuses with `Engine`
kind `event`; cancellation and allocation errors retain their work-error identity.
Failed worker admission spends a change draft and releases its prior staged rows.
Rejection citations encode their Events on the worker before JavaScript delivery.
`scripts/check-event-ingress.cjs` exercises these boundaries through the actual
addon, including operation registration before semantic refusal, ownership after
input mutation and runtime reuse. Native tests cover cancelled admission,
draft-prefix reclamation, cancellable citation encoding and ownership transfer
of complete Event fault payloads.

The Event query suite also executes exact tiny-world answers for Boolean,
cardinality, readout and relation programs, typed staged Pack, description
roundtrips, descriptor/variable ownership, foreign contexts and wrong import
roles. Its transport fixtures are independent of native constructors.
The descriptor suite adds all seven native construction/inspection paths,
editable roundtrips, independent BEDC grammar, distinct ST/TU/SU projections,
correlated product supports, reversed roles, measured endpoints, incomplete-square
refusal, worker copy boundaries, runtime release and cancelled inspection delivery.
A real stored query uses inspected projections and an authored composition plan.
The source suite checks exact decimal/binary intent, independent BERA bytes,
signed functions and copied readouts, 62-coordinate law designation, zero-mass
evidence, operand-copy boundaries and cancellation. Its persisted Coup fixture
recovers `92/147`, `5/21`, evidence mass `49/130` and signed expectation `-37/147`
after reopening and dropping the original runtime. Pure construction is also
tested with the addon made unresolvable.

Contextual full schema terms, Event-valued schema selections/closed rosters and
Event capacity positions retain their existing explicit native guards. The
complete SDK still needs general fixed-point programs, kernel/revision BESC
authoring and operations, parameterized source functions and observations,
and native query observation/expectation slots.
The native algebra is broader than this SDK surface. These boundaries do not
narrow the M0–M8 goal.
