# Event values and dependency semantics

**Proposed contract.** All Event syntax in this directory is new; it does not
compile on the baseline. Storage, query behavior and implementation gates have
[their own documents](README.md). This document owns the value and schema laws.

## The value

An Event is `(U, A)`, where `U` is an explicitly identified, inhabited finite
universe and `A ⊆ U`. Its points are caller-defined possibilities. The type is
`event` / `Event`, without a model, probability, or world-count type parameter.

For the initial carrier, `U = (name: UUID, world_count: nonzero u64)` and the
points are indices `0 .. world_count`. **The complete descriptor is identity.**
Equal names with different counts are different, incompatible descriptors.
Neither field is database-issued. Reordering or changing the meaning of worlds
requires a fresh name, even when the count stays constant. The database cannot
validate the caller's interpretation of an index.

There is no global registry enforcing name-to-count functionality. That would
create hidden cross-table constraints and write effects. Applications that need
one common scope anchor it through ordinary parent rows and Event containments,
as in [Coup](coup.md). Standalone well-formed values remain portable between
stores. Import preserves the descriptor; it does not mint a new identity.

The caller supplies membership. It can enumerate worlds or export them from a
solver. A single solver witness is one world, not an implicit representation of
all solutions. A marginal AI score is not enough to specify a region or how two
regions share worlds. No probability law, normalization equation, strategy,
source API credential, or executable expression is stored inside an Event.

## Algebra and equality

| Operation | Meaning |
| --- | --- |
| Empty / full | `∅` / `U`, retaining U |
| Complement | `U \ A` |
| Intersection | `A ∩ B` |
| Union | `A ∪ B` |
| Difference | `A \ B` |
| Pack | Union of an explicit group of Event values |
| Membership | Whether the operand's world index belongs to A |
| Pair relation | One of fifteen occupancy cases, or different universe |

Value equality is total: descriptor equality plus member equality. Hashes,
handles, addresses and insertion order are not identity. Different universes
are unequal even for empty/full. There is no public numeric or lexicographic
order on Event values; internal sorting is an implementation detail.

Binary construction and Pack require a common descriptor. They return a typed
mismatch on disagreement, including behind empty/full shortcuts. No operation
implicitly aligns worlds, creates a product universe, or assumes independence.
Complement always uses U, not the currently observed evidence or a parent row's
narrower region. Intersect with that evidence explicitly when appropriate.

Every construction returns one value, including empty. A present fact carrying
empty differs from an absent fact: scalar joins and counts can observe the row;
complement produces full. Ordinary projection still deduplicates equal output
rows. No query or write drops empty-valued rows implicitly.

The [query contract](queries.md) distinguishes these partial constructors from
total pair filters. This distinction is necessary for native predicate placement.

## The schema language

```rust
relation Slot { game: u64, seat: u64, slot: u64, scope: event }
relation Holding { game: u64, seat: u64, slot: u64, card: u64, when: event }

Slot(game, seat, slot) -> Slot;
Slot(game, seat, slot, scope) -> Slot;
Holding(game, seat, slot, when) -> Holding;
Holding(game, seat, slot, when) <= Slot(game, seat, slot, scope);
```

A functional dependency (FD), written `->`, is a key. An inclusion dependency
(IND), written `<=`, is coverage by matching target facts. For Event positions,
read both point by point, just as for interval positions:

- `Holding(game, seat, slot, when) -> Holding`: distinct facts with that scalar
  prefix must have disjoint Events. At each world the slot identifies one fact.
- `Holding(..., when) <= Slot(..., scope)`: each selected source region must be
  included in the union of selected target regions with matching scalar fields.
- Reversing coverage requires the other direction. Existing `==` expands to
  both INDs. Combined with a pointwise key, it gives an exact partition.

The compiler still requires the exact projected target key. A scalar key on
`(game, seat, slot)` does not substitute for the explicitly declared key on
`(game, seat, slot, scope)`. Existing selection rules and statement ordering
remain applicable. Schema syntax `<=` is not query scalar ordering.

A partition of full needs no weights. Any normalized probability measure a
caller later chooses assigns that partition total mass one. The schema checks
partition structure without choosing or storing that measure.

## Complete admission rules

For a pointwise key, each scalar determinant group has one descriptor, including
empty Events. For an IND, all selected source and target Events within the same
scalar group must align. Source-only groups still validate their descriptors.
Different scalar groups can use different universes. These are statement-local
conditions, not restrictions on unrelated fields or the whole database.

Let `T` be the union of matching target regions. The source check is `A ⊆ T`.
If no target exists, T is empty in the checked source universe. An empty source
therefore satisfies that pointwise IND, but a separate scalar IND can still
require a target row. A nonempty source fails. A foreign target is a mismatch,
not a coverage witness. Row selections apply before gathering participants;
excluded rows do not become inputs to that IND.

Whole-fact set identity runs first: reinserting a fact is idempotent. Empty
facts with different payload fields remain different facts. Empty contributions
have no overlap and provide no coverage, but they still obey ordinary scalar
keys, scalar INDs, row counts and applicable capacity statements.

Writes are checked against the transaction's final state. Removing and replacing
a partition in one transaction is legal when that final state satisfies the
schema. There is no transient enforcement between individual draft operations.
Use the existing affected-group and final-state judge, with an Event-specific
closed case in compiled region metadata. Do not implement a separate validator
or generalize interval sweeps into a new plugin framework.

A group summary can use two Events, `covered` and `conflict`. For each distinct
fact region A, fold:

```text
conflict = conflict ∪ (covered ∩ A)
covered  = covered ∪ A
```

The key holds iff conflict is empty. Merge two summaries by unioning their
conflicts plus the intersection of their coverages. This is an order-independent
summary of zero, one, or multiple contributors at each point. Pack needs only
coverage; it does not impose a stored key.

For the first implementation, recompute each affected final group using the
existing scalar-prefix index. Do not maintain a persistent Event summary tree.
Deletes must read surviving contributors; subtracting deleted coverage from a
union is generally wrong. Retain the existing full-scan/reference judge path
and compare its verdict with the incremental path.

Constraint refusals use existing statement/group/row diagnostics. A key-overlap
citation must name actual participating conflicting facts, not all rows in the
group. A second pass against the conflict region can select them without storing
an Event-to-fact provenance graph. Universe disagreement is a typed statement
failure with contributing rows. Follow existing bounded citation ordering; no
new collect-every-fault subsystem is required.

## Where interval assumptions must change

A complete nonempty interval key can establish row uniqueness. An Event key
cannot do so unconditionally: two distinct empty-valued rows may share it.
Initially, **emit no complete-Event-key uniqueness witness**. Continue using
ordinary scalar-key and whole-fact witnesses. Nonemptiness-based refinements
can be a later proven optimization. Point membership is not exact value binding.

The compiled type/statement table must distinguish Event from interval, even
when their resident widths match. Event regions do not have start/end endpoints,
interval duration, or an endpoint sweep. Index routing still uses the scalar
prefix; the row retains the region value.

## Deliberate surface boundaries

| Surface | Initial rule |
| --- | --- |
| Ordinary relation fields | Event permitted; several Event fields may exist in one row |
| FD/IND region projection | At most one region position, last; Event-to-Event only |
| Mixed or multiple Event/interval positions | Reject at schema validation |
| Exact row identity, query equality, grouping | Complete Event equality |
| Closed relation Event field | Reject, following the current closed-relation restriction on variable-sized text |
| Statement selection on an unprojected Event field | Exact canonical value equality/set membership; use the ordinary literal machinery |
| Event in capacity projection, weight or duration | Reject; scalar-group row capacity with Event payload elsewhere is unchanged |
| Probability, automatic coordinate transport | Caller responsibility; no built-in operator |

There is no special Event literal language in `schema!` in this slice. Dynamic
schema descriptors carry checked Event literals where selections require them.
The Rust field type and TypeScript field descriptor remain structurally `event`;
optional host nominal wrappers do not create different runtime universes.
