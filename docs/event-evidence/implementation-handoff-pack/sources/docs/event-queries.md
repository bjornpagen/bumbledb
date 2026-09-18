# Event query heads

The implementation branch supports constructive `Event(...)`, structural
`Test(...)` and grouped Event `Pack(...)` heads in Rust macros and pure-data IR.
This is the complete-binding structural slice of M5. Retained face imports, relational query
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

## Scope admission and failure

The first syntactic leaf supplies the expected space for a same-space expression
or test. All leaf occurrences are checked against it before evaluation, including
`Empty`/`Full` anchors, ignored truth-function operands, both ITE branches and
every cardinality position. Equivalent decoded owners align through the checked
execution registry. Different named spaces or original legal supports refuse.
No product, law or source renaming is inferred.

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

IR admission bounds each Event expression/test to 128 levels and 4,096 nodes,
before normalization clones it. The current expression evaluator is bounded
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

Native tests compare complete and coupled four-world supports against explicit
bitset answers on resident and cursor paths. They exercise all sixteen truth
functions, staging/lifetime, repeated roster positions, ignored branches,
duplicate written rules, DNF collapse, nonparticipating rows/columns, canonical
fault ordering, spill, cancellation and diagnostic capacity. Pack tests compare
336 pairs of legal four-world regions, duplicate rules, empty/absent groups,
computed keys, owner lifetimes, context changes after saturation, wide keys,
forced scratch spill and rule-layout changes. The scalar/interval reference
oracle explicitly refuses Event claims; these tests use independent bitsets.
The N-API fixture admits 29 constructor/test/Pack cases, rejects nine malformed
or cyclic programs, and executes four Pack scenarios against an isolated store:
absence, complementary claims, a staged complement, and a foreign-context refusal.
No installed package is replaced. Certified factoring and performance
qualification remain separate gates.
