# Exact Event selections

An Event selection matches a complete value: named context and coordinates,
legal support, declared law (including its absence), and supported region. It
neither tests overlap nor clips the projected region. `A <= B` on Event
projections still means pointwise coverage; `filter == A` chooses whole facts.
Two Events with the same probability need not match. An empty Event or a
nonempty zero-mass Event can match a literal and activate a scalar dependency.
Different contexts are different literal values, not an attempted alignment.

## Declaration surfaces

The SDK uses the existing selection syntax:

```ts
const Child = relation("Child", { group: u64, filter: event, region: event })
const Parent = relation("Parent", { group: u64, region: event })
const theory = schema("Selected", { Child, Parent }, [
  key(Parent, ["group", "region"]),
  contained(
    on(select(Child, { filter: accepted }), ["group", "region"]),
    on(Parent, ["group", "region"])
  )
])
```

`accepted` is an owned Event. A literal set is `[accepted, another]`. Every
selected row contributes its entire `region`. Selecting a projected field
remains invalid under the existing dependency-language rule. Scalar, interval,
Event and contextual-full projections can be filtered by a different Event
field; scalar capacity groups can also use Event selections.

Rust descriptors use `LiteralSet::One(Value::Event(accepted))` or `Many` on a
`Side`. `SchemaSpec` uses `LiteralSpec::Value` and its named selection fields.
The Rust `schema!` macro accepts a canonical BEVT byte string at an Event
literal position, using its existing typed byte-literal syntax. Expansion
checks the complete encoding and emits owned reconstruction. Runtime-created
world spaces are naturally authored through descriptors rather than copied
into source code as bytes.

## Owned representation

Resident `Event::Eq` remains owner-scoped. Schema admission captures canonical
BEVT bytes under `validate_with_control`; the default `validate` uses ordinary
unbounded control. The sealed schema retains those bytes alongside the literal
owners. Sorting, duplicate detection, mirror pairing and fingerprinting use
complete canonical bytes, never resident keys. Internal keys locate retained
bytes only and are neither serialized nor compared between owners.

Selections encode an actual Event cell once per tested field, then compare its
bytes against the admitted alternatives. Every complete, incremental, indexed,
unindexed and citation path uses the same fallible predicate. Cancellation or
encoding failure stops judgment; it never becomes a false match. Scalar cells
retain their ordinary equality.

The existing `bumbledb-schema-v6` stream is unchanged for previously admitted
schemas. The selected field already determines the literal type. For Event,
the literal extension is a u32 little-endian byte count followed by canonical
BEVT bytes; alternatives sort lexicographically by those bytes. No resident
identifier enters the fingerprint. Native diagnostics render complete BEVT hex.

## Transport and scope

Pure SDK selection authoring owns transport bytes without loading the addon.
The managed schema, draft, create/open, row-codec, change-set parse and snapshot
paths copy envelopes before dispatch and admit their graphs on a worker. Managed
log history create/open, command decode, tenant caches, administration (including
backup/restore) and transitions use the same stage. Operation registration
precedes copying. Pending schemas have no descriptor interpretation until the
worker resolves them; malformed Event graphs cannot reach storage or publication.
Existing shape/protocol diagnostics and typed work/resource refusals survive
the copy boundary. One grammar declaration generates both the concrete public
Rust authoring types and the generic pending-input forms. Public enums remain
actual enums, preserving variant imports and inference for literal-free values.
The fallible interpreter produces a concrete `SchemaSpec`; only that admitted
form has a descriptor interpretation. Returned schema metadata
captures Event encodings on the worker, so JavaScript delivery only copies
owned bytes. Generated SDK bindings reconstruct pure Event carriers and preserve
native schema fingerprints.

The historical synchronous descriptor and log-schema handles still use the
scalar-only bridge and explicitly refuse Event schema literals. All managed
core and log schema entry points use owned input admission. [Closed Event
rosters](event-ground.md) now also support exact selections. Event capacity
projection positions remain guarded. None of these guards narrow M0–M8.

## Evidence contract

Native tests cover independently allocated and reordered owners, selection-set
ordering, duplicate statements/literals, mirror pairing, distinct contexts,
supports and laws, empty and zero-mass values, selected capacity counts,
unclipped Event/interval coverage, typed macro literals and cancellation.
SDK tests cover immutable pure authoring, worker rejection, incremental target
replacement, atomic repairs, row/change-set codecs, original-runtime release,
reopen and compiled/imported generated bindings.
Log SDK tests additionally cover command replay after runtime release, cache
ownership, exact-selection enforcement, cold backup/restore, failed transition
judgment, retry, activation and abort. Raw bridge checks require operation
registration before semantic refusal, reject shared backing and oversized Event
envelopes, preserve existing protocol diagnostics, prove no storage side effects
for malformed schemas and check retained-handle reclamation. The log suite uses
compiled private copies of both SDKs and a private addon; it is not a packaged
release or hosted-storage qualification.

`SchemaSelections.lean` checks twelve denotational statements: exact singleton
selection, alternatives, ordering/duplicate semantics, retained context/support/
law, empty and zero-mass matching, overlap counterexample, codec-faithful matching
and projection preservation. Codec faithfulness is an explicit implementation
obligation. These reports do not prove the Rust codec, cancellation or allocator.
