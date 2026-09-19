# Contextual Event projections

The native schema language supports a typed full Event in the final projection
position. This is an implemented structural M3 slice, also valid under designated fixed laws; it does not close the full
Event implementation plan. See [the ledger](event-implementation.md).

```rust
bumbledb::schema! {
    pub Partitions;
    relation Roster { group: u64 }
    relation Branch { group: u64, choice: u64, when: event }

    Roster(group) -> Roster;
    Roster(group, true) -> Roster;
    Branch(group, when) -> Branch;
    Branch(group) <= Roster(group);
    Roster(group, true) == Branch(group, when);
}
```

For each group, the Branch facts partition the legal worlds. The key forbids
overlap between distinct whole facts; the mirror requires their union to be
full. Empty branch values remain ordinary stored facts. The independent scalar
containment requires a roster row even for an empty branch. A probability law,
when added, measures this partition; no weight column defines it.

## Meaning and context

A stored Event owns a named nonempty world space. Projection `true` has no
stored owner: it denotes full in the checked context of its statement group.
The target's Event operands establish that context, or the first source Event
does so when the target is contextual full or absent. Every subsequent Event
operand must align to it, including empty operands. Different scalar groups may
use different spaces. The native finite-space constructor and decoder both refuse
empty support.

| Source region | Target contributors | Coverage condition |
| --- | --- | --- |
| Stored Event | Stored Events | Source is included in their aligned union |
| Full | Stored Events | Their aligned union is full |
| Stored Event | Present full target | Compatible source context; coverage holds |
| Full | Present full target | Coverage holds |
| Full | No target | Reject |
| Empty Event | No target | Coverage holds; its context is still checked |

A contextual full key has the same uniqueness consequence as a scalar key,
because every admitted space contains a world. `Config(true) -> Config` is a
global singleton key. The authored full law remains distinct from an ordinary
scalar key: exact target-key resolution never silently substitutes one for the
other. `RosterByGroupTrue { group }` and `ConfigByTrue {}` supply stored fields
only. An independently sufficient full key also permits keyed lookup of an
empty stored Event.

Closed scalar rosters can participate through full projections with explicitly
declared matching keys. Full/full ground laws are checked while sealing the
schema. Event-valued closed rosters, Event selections and Event capacity
positions remain outside this implemented slice.

## Structural representation and transport

The shared theory uses `Projection<F>`, with `Fields(Box<[F]>)` and
`EventFull(Box<[F]>)` variants. `F` is `FieldId` in resolved descriptors and
`Box<str>` in named specifications. `fields()` returns only stored fields;
`arity()` includes the contextual constant. There is deliberately no implicit
slice conversion that could erase the constant. Existing field lists convert
with `.into()`.

Only the full constant is constrained to the trailing logical position, and its
prefix must be scalar. Stored Event keys retain the one trailing-region rule;
field-only containments may reorder their paired positions as before.

The schema-file JSON spelling is `[0,{"event":"full"}]`; named JavaScript
specifications use `["group",{event:"full"}]`. Sealed JavaScript descriptors
return numeric field positions and the same marker. Duplicate, nonfinal,
unknown and multi-arm markers refuse. A Boolean `true` cell is not a projection
constant in this wire grammar. High-level SDK authoring uses the trailing
Boolean token `true`: `key(Roster, ["group", true])` and
`on(Roster, ["group", true])`. A singleton uses `key(Config, [true])`.
The authoring layer lowers that token to the typed wire marker; it never mints
a stored Event or a field named `true`. A stored column with that name is the
string `"true"` and remains distinct in rendering, key resolution and transport.

SDK faces pair full with the Event field type, while class inference pairs only
stored columns. True contributes no fabricated join-class coordinate. Exact
target-key matching retains the full term; a scalar key with the same physical
prefix cannot replace it. `QueryReader.get(fullKey, { group })` supplies only the
stored prefix; a full-only key takes `{}`. Closed scalar rosters may declare
additional full keys through the same `key` API. Their default id key remains
implicit; other explicit field-only closed keys remain outside the SDK surface.

Generated TypeScript bindings now preserve full projections, closed full keys,
schema identity and statement order. SDK tests execute measured partitions,
missing coverage, overlap, empty orphan rows, singleton lookups and reopened
Pack queries. Ground full/full laws over selected closed scalar rosters still
run through native admission.

Field-only JSON and v6 schema fingerprint bytes are unchanged. In the fingerprint
stream, a full projection contributes its logical tuple length, physical field
IDs, and trailing little-endian `0xffff`. This is a wire opcode only, never a
fabricated `FieldId`. An admitted relation has at most 65,535 stored fields,
numbered from zero, so no valid field collides with it. The BEVT value codec is
unchanged: a contextual projection constant is not a persisted Event value.

Sealed keys keep physical field IDs plus `KeyForm::EventFull`. The compiler
assigns scalar uniqueness to that form under the nonempty-space contract.
Event coverage continues to use the existing conservative query witnesses;
empty Event coverage cannot authorize ordinary join elimination or a target-row
cardinality bound. Unsupported scalar/interval benchmark oracles explicitly
refuse Event theory instead of silently interpreting it as scalar equality.

## Proof and implementation boundary

[Admission.lean](../crates/bumbledb-event/semantics/Admission.lean) proves:

- Full keys are equivalent to scalar uniqueness, assuming a nonempty world type.
- Full coverage is universal coverage by the contributor union.
- A pointwise key plus full coverage gives exactly one distinct fact per world.
- An absent target cannot cover full in a nonempty space.
- Full/full containment is actual target presence.
- Without nonemptiness, the key and presence conclusions fail.

These are denotational theorems, not verification of Rust or the byte parser.
[Native tests](../crates/bumbledb/tests/event_full.rs) exercise 768 finite partition
cases, scalar uniqueness, contextual alignment, ground laws, rollback, deletion,
reopening, key lookup and projection permutations. Schema-file tests cover marker
roundtrip/refusal and identity separation. The qualification runner loads the
just-built Node addon to check the actual N-API grammar; it replaces no installed
package. Earlier evidence remains unchanged.
