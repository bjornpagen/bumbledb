# Event query heads

The implementation branch supports constructive `Event(...)` and structural
`Test(...)` heads in Rust macros and pure-data IR. This is the complete-binding
structural slice of M5. Event `Pack`, retained face imports, relational query
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
erase an Event program until a participation-preserving certificate exists.
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

The [query reference](../crates/bumbledb-event/semantics/Query.lean) adds 22 checked
reports for participation, empty values, cardinality and fault collection. The
schedule laws require equal participating binding/program sets and retained
written provenance; they do not certify arbitrary optimizer rewrites. A separate
proof shows that an omitted participating fault invalidates a claimed complete
report. These are reference denotations, not Rust extraction.

Native tests compare complete and coupled four-world supports against explicit
bitset answers on resident and cursor paths. They exercise all sixteen truth
functions, staging/lifetime, repeated roster positions, ignored branches,
duplicate written rules, DNF collapse, nonparticipating rows/columns, canonical
fault ordering, spill, cancellation and diagnostic capacity. The N-API fixture
admits 28 constructor/test cases and rejects nine malformed or cyclic programs
using a temporary addon and store. No installed package is replaced.
