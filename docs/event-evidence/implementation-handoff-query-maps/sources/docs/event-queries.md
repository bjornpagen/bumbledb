# Event query heads

The implementation branch supports constructive `Event(...)`, structural
`Test(...)`, captured readout maps and grouped Event `Pack(...)` heads in Rust
macros and pure-data IR. This is the complete-binding structural slice of M5.
Retained face imports, relational query
programs, probability/expectation heads and certified branch factoring remain
separate gates. These forms are not part of public v1.3.1.

```rust
let query = bumbledb::query!(Game {
    interior conditions(id, region: Event(a & !b), safe: Test(Subset(a, b))) |
        Claim(id, when: a, blocked: b);
    (id, region) | conditions(id, region, safe), safe == false;
});
```

`Game` and `Claim` above stand for application schema declarations. A head alias
is an output column; consuming it requires a later interior. Ordinary body
matching and anti-probing retain their relational meanings. The current macro
requires labels on Event/Test heads. Recursive heads still project variables.

## Constructive algebra

Supported syntax is `!a`, `a & b`, `a ^ b`, `a | b`, parentheses,
`Empty(anchor)`, `Full(anchor)`, `Ite(condition, high, low)`, and
`AtLeast(n, ...)`, `AtMost(n, ...)`, `Exactly(n, ...)`. Operator precedence is
`!`, `&`, `^`, `|`. Thresholds are nonnegative integer literals and the roster
must have at least one Event expression to establish its scope. A roster counts
positions: `Exactly(2, a, a)` means `a`, while `Exactly(1, a, a)` is empty.

The public `EventExpr` IR exposes all sixteen `BoolOp4` functions and an inclusive
cardinality range. `EventTest` supplies `IsEmpty`, `IsFull`, `Subset`, `Equal`,
`Disjoint`, and `Covers`. Tests concern entire legal regions; they are neither
probability comparisons nor a decision about one sampled world.

Every Event head constructs one value for its input binding. Empty is an owned
value and survives projection, storage, spill and staging. Distinctness can
merge equal projected values, as with ordinary relational projection. No binding
is dropped because its computed Event is empty. An input with no matching body
binding still produces no row.

## Grouped Event union

```rust
let query = bumbledb::query!(Game {
    interior claims(player, region: Event(a & !b)) |
        Claim(player, when: a, blocked: b);
    interior covered(player, region: Pack(region)) | claims(player, region);
    (player, missing: Event(!region)) | covered(player, region);
});
```

`Pack` unions a body-bound Event, grouping by every other head value. Computed
head values can be group keys. It emits one Event per present group; disconnected
regions remain one value. Overlap and duplicate derivations are counted once.
A group containing only empty Events emits empty. No inputs means no group,
including a Pack without grouping columns; explicit empty seeds establish presence.
The existing one-Pack-per-head and no-mixing-with-numeric-folds rules remain.

Each group requires one named space and original support. Its expected context
is the lexicographically least canonical BEVT full-space descriptor among its
participating claims. This only selects deterministic diagnostics: successful
groups have one context, and foreign spaces are never merged. Empty and full
claims both participate. All mismatching claims retain their written rule and
variable identities; exact repeated descriptors are deduplicated as in Event heads.
The aggregate's sole input has operand index zero. Identical fault descriptors
across bindings/groups denote one fault, rather than an occurrence count.

If a computed group key fails Event admission, that binding supplies its head
faults and supplies no Pack group. Every later binding with valid keys still
contributes, even after earlier head faults. Group faults and head faults are
collected together before stage publication. Scalar and operational refusals
retain their existing immediate-failure contract.

The implementation keeps an exact scratch-backed claim relation keyed by group
token, Event registry key, written rule and variable. Separate maps retain group
keys and canonical context minima. Numeric tokens are execution-local and never
appear in diagnostics. Claims stay inline for ordered group traversal; arbitrarily
wide group keys use exact lookup. The claim table spills at 4,096 distinct claims
or 4 MiB of accounted payload/entry estimates. This is a spill trigger, not a total
process-memory budget: the registry still owns its Events until generation release.
Union and output materialization occur at stage sealing, after the context minima
are known. Keeping claims prevents saturation or deduplication from losing faults.

## Captured readout maps

`EventImport` retains a canonical BEDC descriptor and its checked native owners.
Construct it with `capture` from an `AdmittedDescriptor`, `admit` from plain
`Descriptor` data, or `from_bytes` from BEDC bytes. Reconstruction rechecks every
certificate. Equality compares the exact canonical descriptor bytes; the hash
in its bounded Debug output is only a display fingerprint.

```rust
let observation = bumbledb::EventImport::capture(
    &bumbledb::event::AdmittedDescriptor::Map(readout),
    bumbledb::event::DescriptorLimits::default(),
    &work,
)?;
let query = bumbledb::query!(Game {
    use map observed = &observation;
    (id, possible: Event(Possible(a, observed)),
         certain: Event(Guaranteed(a, observed))) | Claim(id, when: a);
});
```

`readout` is a previously admitted `CoordinateMap`; `work` implements Event's
`Control` trait. Imports are retained data, with no per-row host callbacks.
Map and template imports share lexical duplicate-name checks. An ordinary
template named `map` still uses `use map = &template;`.

For a map from S to T, each operator takes `(event_expression, imported_map)`:

| Expression | Input → output | Meaning |
| --- | --- | --- |
| `Pullback(b, f)` | T → S | Source worlds whose readout satisfies b |
| `Image(a, f)` | S → T | Readouts reached by at least one world in a |
| `UniversalImage(a, f)` | S → T | Readouts whose every source world satisfies a, including absent fibres |
| `NonvacuousImage(a, f)` | S → T | Universal image restricted to reachable readouts |
| `Possible(a, f)` | S → S | Worlds sharing a readout with some world in a |
| `Guaranteed(a, f)` | S → S | Worlds whose entire observation cell satisfies a |

Both `Map` and `Surjective` descriptors are accepted. Other checked descriptor
kinds refuse as `ImportKind`; a checked fibre product is not a readout. Operators
nest inside Boolean expressions, tests and cardinality rosters, and their results
use ordinary interior staging, Event storage and Pack.

## Scope admission and failure

Boolean components require one common context. A captured map fixes the contexts
on both sides of its boundary; its declared output also constrains ordinary
sibling leaves. Incompatible declared map outputs refuse during shape admission,
even if the query body would be empty. Components without an imported context
retain the first syntactic leaf as their context anchor.

All leaf occurrences are checked against their required contexts before evaluation, including
`Empty`/`Full` anchors, ignored truth-function operands, both ITE branches and
every cardinality position. Equivalent decoded owners align through the checked
execution registry. Different named spaces or original legal supports refuse.
No product, law or source renaming is inferred. One variable used on both sides
of a map boundary has two separate demands. For example, `Image(a, f) & Full(a)`
requires a in f's source at the first occurrence and in its target at the second;
distinct endpoint contexts yield a fault even if a is empty.

The computed sink reads complete bindings and never licenses a suffix skip or
fused scan. Operand validation is independent of algebraic simplification.
Unmatched rows and unused columns are outside expression participation.

All context faults in the first failing stage are collected before that stage
can publish results. `Error::EventFaults` owns a sorted, duplicate-free array of
`EventOperandFault` records:

- Logical interior index (or main), written rule, head position, syntactic leaf
  position and variable; repeated leaf positions retain separate identities.
- Error category and canonical BEVT bytes of the offending value and expected
  full-space Event. Resident keys, allocation order and physical row order are
  absent from the descriptor.

DNF collapse retains every written-rule stamp. Cross-rule subsumption cannot
erase an Event program or Event Pack until a participation-preserving certificate exists.
Different valid scalar groups may retain different Event spaces.

Diagnostics have an explicit stage limit of 65,536 distinct records and 16 MiB
of accounted record/canonical payload bytes. This bounds diagnostic payload,
not total allocator overhead or serialization workspace. Capacity, allocation,
cancellation, registry corruption and scalar failures return their own errors;
they never label a partial diagnostic set complete. Existing scalar first-error
semantics are unchanged. In a mixed head, Event admission precedes interval
construction; an interval producer's empty output cannot hide a participating
Event fault. Scalar evaluation retains its existing interval-first order.

IR admission bounds each Event expression/test to 128 levels, 4,096 nodes and
16 MiB of imported descriptor bytes, counted by syntactic occurrence, before
normalization clones it. These are payload/shape limits, not a bound on all
retained arenas or decoding workspace. The current expression evaluator is bounded
recursive Rust over that admitted tree. These query programs have multiple
body inputs; the host `EventProgramBuilder` remains a separate single-input
typed DAG used by fixed points.

## Transport and proof boundary

Raw Node `FindTermIr` accepts `kind: "event"` or `"test"`, with a strict owned
expression descriptor. The TypeScript parser and native decoder both check
shape, depth, node count, variable ordinals and truth-function extent. Cardinality
bounds use u64 BigInts. Complete high-level TypeScript Event authoring and
structured transport of the Rust fault records are still pending. The existing
Node engine-error path currently carries the Event family and diagnostic text.
Event Pack uses the existing `{ kind: "pack", over: variableOrdinal }` wire form
and an aggregate/pack head signature; the schema establishes the Event domain.

Readout expressions use `{ kind: "map", op, descriptor, expr }`, with BEDC
`Uint8Array` bytes and one of `pullback`, `image`, `universalImage`,
`nonvacuousImage`, `possible` or `guaranteed`. The TypeScript parser copies bytes
after checking their aggregate extent and refuses shared buffers. It checks
shape; native BEDC reconstruction establishes semantic admission. Raw N-API
reconstruction currently occurs during marshalling with an uncancellable control;
execution kernels run under the query WorkContext. This boundary still needs
the complete SDK's asynchronous construction/resource policy.

The [query reference](../crates/bumbledb-event/semantics/Query.lean) adds 22 checked
reports for participation, empty values, cardinality and fault collection. The
schedule laws require equal participating binding/program sets and retained
written provenance; they do not certify arbitrary optimizer rewrites. A separate
proof shows that an omitted participating fault invalidates a claimed complete
report. The [Pack reference](../crates/bumbledb-event/semantics/Pack.lean) adds
19 reports: grouped union/presence, canonical minimum selection in a lawful total
order, traversal/duplicate invariance, context admission, retained provenance,
and participation after computed-key failures. Native canonical bytes must
instantiate that order, and scratch/registry/group extraction must preserve the
reference claim sets. These are reference denotations, not Rust extraction.

[QueryMaps](../crates/bumbledb-event/semantics/QueryMaps.lean) adds 18 reports,
11 axiom-free, over a scope-indexed expression reference. It proves that retained
demanded inputs determine the whole program, every occurrence keeps its expected
scope, ignored branches retain demands, and no occurrence faults exactly when
all demands are admitted. Map denotations preserve possible/guaranteed bounds and
distinguish universal from nonvacuous image. The Rust shape checker, canonical
marker comparison, input alignment and descriptor reconstruction must establish
the reference premises; their extraction is not kernel-verified.

Native tests compare complete and coupled four-world supports against explicit
bitset answers on resident and cursor paths. They exercise all sixteen truth
functions, staging/lifetime, repeated roster positions, ignored branches,
duplicate written rules, DNF collapse, nonparticipating rows/columns, canonical
fault ordering, spill, cancellation and diagnostic capacity. Pack tests compare
336 pairs of legal four-world regions, duplicate rules, empty/absent groups,
computed keys, owner lifetimes, context changes after saturation, wide keys,
forced scratch spill and rule-layout changes. The scalar/interval reference
oracle explicitly refuses Event claims; these tests use independent bitsets.
Five readout integration tests compare all six operations against explicit legal
fibres, including coupled support, independently owned imports, context changes,
repeated variable occurrences, static refusal on empty bodies, mixed template/map
imports and staging through Pack. The N-API fixture admits 35 forms, rejects
14 malformed or cyclic programs, and executes four Pack scenarios against an isolated store:
absence, complementary claims, a staged complement, and a foreign-context refusal.
Eight additional executions cover six readout operators, a changed output context
and a foreign input refusal; malformed readout arity is rejected at import.
No installed package is replaced. Certified factoring and performance
qualification remain separate gates.
