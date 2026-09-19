# TypeScript Event values, host algebra and query programs

This implements structural SDK slices of M2 and M5. `event` is an ordinary,
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
the actual map/face/product on its worker, including on an empty body. These
carriers currently consume descriptors authored through the Rust API; native SDK
constructors for maps/faces/products and BESC sources are still to come.

Programs expose a frozen structural `node` with variable references and owned
descriptor carriers. The same generic IR grammar represents builder and wire
trees; one structural translation assigns variable ordinals and copies bytes.
There are no per-row JavaScript callbacks or independent algebra evaluators.
Whole-query snapshots copy binders and expression references together. Spreading
a wrapper cannot forge a checked program; portable descriptions use the checked
description parser. Builders count written occurrences, including reused subtrees,
against the existing 128-level, 4,096-node and 16 MiB import-payload limits.
These bounds do not account for total retained native memory.

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

Contextual full schema terms, Event-valued schema selections/closed rosters and
Event capacity positions retain their existing explicit native guards. The
complete SDK still needs general fixed-point programs, BEDC/BESC authoring,
source/revision/function operators and observations.
The native algebra is broader than this SDK surface. These boundaries do not
narrow the M0–M8 goal.
