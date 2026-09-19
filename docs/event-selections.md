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
paths copy envelopes before dispatch and admit their graphs on a worker. The
same generic schema grammar carries pending envelopes and admitted values;
only admitted values have a descriptor interpretation. Returned schema metadata
captures Event encodings on the worker, so JavaScript delivery only copies
owned bytes. Generated SDK bindings reconstruct pure Event carriers and preserve
native schema fingerprints.

The historical synchronous descriptor conversion and managed log administration,
transition/cache/history schema conversion still use the scalar-only bridge.
They explicitly refuse Event schema literals; migrating those consumers to the
owned schema input stage remains open. This boundary does not affect the managed
core database paths above. Event-valued closed rosters and Event capacity
projection positions also remain guarded. None of these guards narrow M0–M8.

## Evidence contract

Native tests cover independently allocated and reordered owners, selection-set
ordering, duplicate statements/literals, mirror pairing, distinct contexts,
supports and laws, empty and zero-mass values, selected capacity counts,
unclipped Event/interval coverage, typed macro literals and cancellation.
SDK tests cover immutable pure authoring, worker rejection, incremental target
replacement, atomic repairs, row/change-set codecs, original-runtime release,
reopen and compiled/imported generated bindings.

`SchemaSelections.lean` checks twelve denotational statements: exact singleton
selection, alternatives, ordering/duplicate semantics, retained context/support/
law, empty and zero-mass matching, overlap counterexample, codec-faithful matching
and projection preservation. Codec faithfulness is an explicit implementation
obligation. These reports do not prove the Rust codec, cancellation or allocator.
