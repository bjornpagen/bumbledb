# TypeScript Event field, owned values and host algebra

This implements the first public SDK slice of M2. `event` is an ordinary,
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

## Qualification and remaining boundaries

`scripts/check-event-sdk.mjs <compiled addon>` snapshots SDK sources, copies the
addon to a private temporary package, builds JavaScript/declarations and runs
tests there. It neither overwrites a loaded addon nor publishes a package.
Generated bindings are compiled and imported, then compared with the original
native schema identity. Tests also exercise all 16 functions, symbolic 62-bit
support, measured transport, malformed envelopes, foreign empty contexts,
ordinary dependency admission, persistence/reopen, parameters, Pack, retained
results and interruption between native completion and JavaScript delivery.

The existing row/query marshallers still perform some Event/descriptor admission
synchronously while preparing a job, and malformed row values retain their
`InvalidArgument` mapping. Moving that admission entirely onto cancellable workers
and exposing complete structured Event diagnostics remain SDK acceptance work.
Standalone Event algebra uses the new worker path described above.

Contextual full schema terms, Event-valued schema selections/closed rosters and
Event capacity positions retain their existing explicit native guards. The
complete SDK still needs constructive Event/Test/map/relation query heads,
fixed-point programs, BEDC/BESC authoring, source/revision/function operators and
observations. The native algebra is broader than this first SDK surface. These
boundaries do not narrow the M0–M8 goal.
