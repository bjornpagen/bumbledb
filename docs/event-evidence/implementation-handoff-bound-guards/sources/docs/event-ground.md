# Closed Event relations

Closed relations now admit Event-valued ground axioms. Their values participate
in the same pointwise keys, union coverage, contextual full projections, exact
selections and query algebra as stored Events. Schema sealing rejects refuted
ground laws before a database is created. This completes the closed-roster slice
of M3; Event capacity projection positions remain a separate open gate.

```ts
const Cases = closed("Cases", ["A", "Otherwise"], { group: u64, when: event }, {
  A: { group: 7n, when: a },
  Otherwise: { group: 7n, when: complementOfA }
})
const Claim = relation("Claim", { group: u64, when: event })
const theory = schema("Ground", { Cases, Claim }, [
  key(Cases, ["group", "when"]),
  contained(on(Claim, ["group", "when"]), on(Cases, ["group", "when"]))
])
```

Here `a` and `complementOfA` are owned Events in compatible world contexts.
No weights describe the partition. An independently declared full containment
can require its completeness. Empty Events remain ground facts, and a possible
Event of zero probability still overlaps and requires coverage normally.

## Portable identity and execution

`SealedRow.fact` is portable schema identity. Existing scalar-only rows retain
their exact fixed-width bytes. In a row containing Event, each Event field is a
u32 little-endian length followed by canonical BEVT bytes; ordinary fields keep
their previous encoding. The schema already fixes the types and field order.
This extends the existing schema-v6 fingerprint stream without changing bytes
for any previously admitted schema. Resident resolver keys never enter it.

An Event row also retains owned values and field boundaries. Its Event cells
are registered together when sealed, making same-row field equality safe for
ground folding. Independent rows remain independently owned. Read cursors use
the retained values; relation images register them in the execution generation
before producing the two-word physical fields. Ordinary rows, parameters,
literals and closed rows therefore use the same semantic equality in Free Join.
Owned answers retain their Events after cache, snapshot and database release.

Point reads compare complete canonical Event values across owners. An empty
Event key still requires an independently sufficient scalar/full key before a
single-result lookup is allowed. The synthetic id can provide that guarantee
when included. All closed-row lookup surfaces propagate work/encoding failures.

## Ground laws and context

Closed Event keys check disjointness within each scalar determinant. Two closed
sides are judged during sealing: target contributions form an aligned union,
and each source must be included. Full/full requires actual roster presence.
Closed/open dependencies use the transaction judge and allow atomic repairs.
A closed source with unmet initial demands requires a populated, admitted
instance; ordinary empty database creation correctly rejects it.

Ground and transaction coverage use the same helper. A missing target or a
contextual full target learns the first source Event's context, even if that
source is empty. Every later Event in the group must align to it. Context
includes named coordinates, support and law. Different scalar groups can have
different contexts. Operational or context errors never become false coverage.

Event selections compare complete portable values and keep whole rows. Scalar
member-set containments and capacities can select on an Event field. Scalar
weights, interval durations and dependent bounds retain their field positions
even after variable-size Event cells. Event capacity projections and closed
interval-containment projections retain their existing explicit refusals.

## Authoring and transport

Rust descriptors carry `Value::Event`. The Rust `schema!` byte-literal surface
also admits closed Event axioms. Its closed handle enums remain `Copy`; Event
column accessors reconstruct owned values at runtime, while scalar accessors
remain `const`. Declared-key query lookup remains available through dynamic
statement ids; closed enum handles keep their existing id accessors.

The SDK admits explicit trailing-Event keys on closed relations. Its static and
runtime target-key checks resolve the exact declared key, including permuted
containment projections. Portable carrier equality identifies independently
constructed copies of the same declaration. Native-checked schema snapshots
generate Event-valued axioms and compile back to the same fingerprint. Managed
log schemas retain these axioms through history recovery, caches, backup/restore
and schema transitions. Synchronous legacy schema handles keep their Event guard.

## Verification boundary

Native tests compare keys and full coverage against 768 explicit finite-world
cases, including restricted supports. Additional tests cover empty and zero-mass
Events, incompatible contexts, reordered owners, exact selections, scalar
capacity offsets, work refusal, actual ground-fold selection, point reads,
Free Join images/cursors, literals/parameters, reopen, retained outputs, mixed
dependencies and atomic repair. SDK tests include pure authoring, worker
admission, runtime replacement, generated bindings and managed log lifecycle.

`Ground.lean` adds eleven denotational reports for context retention, foreign
context refusal, empty/full coverage and representation independence under
explicit binding/codec faithfulness premises. It does not prove Rust refinement,
codec correctness, resource behavior or performance. The complete M0–M8 goal,
parameterized sources, remaining query forms and M8 qualification remain open.
