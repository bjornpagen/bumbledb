# The same Event through queries and storage

An `event` field accepts every validated Event in its owner, including empty and
full. A computed result can be saved, loaded, complemented, transported and used
in another query without changing its type or silently dropping its row. This
supersedes the earlier proposed `NonEmptyEvent` storage boundary. The implementation
branch now persists unmeasured Events and checks pointwise field/full dependencies,
including the conservative planner rules below. Typed contextual full projections
have a [native contract](../docs/event-projections.md); the complete source, query
and consumer integration gates remain open. See the
[native ledger](../docs/event-implementation.md).

The empty Event says “this condition holds in no admitted world.” A missing row
says no such fact was supplied. Neither is a failed source request, an invalid
owner, a missing probability law, or an undefined conditional probability.
The owner still requires nonempty legal support; a measured owner still requires
a nonempty family of normalized laws. Admitting the empty subset does not admit
an inconsistent universe.

## Why this belongs in the algebra

Suppose a Coup rule query returns one region for every declared action. One
action has no legal continuation. Its result is empty. Saving that row preserves
the answer “this action is impossible,” including the option's identity. A later
complement produces full. Deleting the row produces no answer at all. Ordinary
joins, counts, anti-probes and explicit rosters can observe the difference.

Empty is already the identity of union, the absorbing element of intersection,
and the complement of full. Inverse image and existential projection preserve
it. Universal preimage deserves care: `All(R, Empty)` is the region with no
R-successor, while `Must(R, Empty)` is empty. These laws require an actual value
to keep composing. They do not authorize hiding validation behind a constant.

## The dependency meaning stays pointwise

Within one scalar determinant group, a fact contributes its Event's worlds.
An empty fact contributes zero worlds to both coverage and conflict. Therefore
adding it preserves the pointwise key and all existing coverage summaries.
A nonempty source cannot be covered solely by empty targets. An empty source's
pointwise inclusion is vacuous.

All Event contributions within a statement's scalar group must first align to
one context. This includes a source-only group containing empty Events: absence
of target rows does not authorize mixing source identities or legal supports.
The empty target union is interpreted in that checked context. Different scalar
groups need not share a context.

Scalar constraints still see the fact. For example:

```rust
At(position, card, when) -> At;
At(position, card) <= PositionCard(position, card);
```

The first statement concerns world ownership. Two distinct empty `At` facts can
share its scalar prefix without conflicting. The second is an ordinary roster
containment and still requires the referenced card even for an empty Event.
Empty cannot create an orphan fact by evading a scalar IND. Nor may it evade
owner, coordinate, wire-object or role validation.

## A precise change to the planner proof

Today's complete interval binding can help prove distinct results because a
stored interval is nonempty. If two distinct facts had the same nonempty region,
any point in that region would violate the pointwise key.

For Event the same proof needs the explicit premise `Possible(region)`. Two
different empty-valued facts have equal regions and no common point. Therefore:

```text
pointwise key + equal complete Event + nonempty Event => same whole fact
pointwise key + equal complete Event                  => insufficient
```

Production must not inherit the interval distinctness or unique-probe witness
unconditionally for an Event field. It can use an established nonempty constant,
a sound nonemptiness refinement, a separate ordinary scalar key, or full-fact
identity. Without such evidence it retains general fanout/deduplication. This is
an optimization premise, not a second public Event type.

## Storage and sparse views

The default branch iterator returns all declared alternatives with their Event
values. An application may explicitly filter to possible branches when it wants
a sparse relation. That operation changes row existence; it is not a transparent
storage optimization. Covering a parent Event does not require a row for every
empty alternative. A scalar roster mirror can require that stronger contract.

Saving an Event keeps its scoped canonical empty/full identity and owner just as
for any other region. `Pack` still needs an explicit seed to produce an absent
group; storing empty does not invent missing groups. All fact equality and scalar
capacity rules retain their ordinary meaning. The coverage/conflict tree may
omit a zero contribution internally while the fact store retains the fact.

## Evidence and integration obligations

[EventStorage.lean](experiments/event-repr-lab/lean/EventStorage.lean) formalizes
empty-row coverage and conflict, the nonempty complete-key theorem and its empty
counterexample, map/projection laws, universal dead ends, scalar-target absence,
row loss before complement, and the distinction between full and empty in a
nonempty owner. All ten reports pass on the pinned Lean 4.32.0 toolchain without
axioms; [the central verifier record](experiments/event-repr-lab/results/lean-check.json)
retains the source hashes and proof reports. These are denotational contracts,
not a verification of native Event persistence or planner code.

Native acceptance must save and reload empty results, retain their row identities
and complements, reject incompatible owners even at empty, enforce scalar INDs
on empty-valued rows, and retain duplicate projected empty keys unless another
proof establishes distinctness. Historical interval-bridge results do not prove
these obligations: encoding an empty Event as interval runs emits no run and
therefore loses precisely the stored fact being tested here.

[Admission.lean](../crates/bumbledb-event/semantics/Admission.lean) now proves the
Boolean summary recurrence, its correspondence with pointwise keys and union
coverage, order independence, and which facts qualify as conflict citations.
Canonical whole-fact deduplication and context alignment are explicit premises.
The native tests exercise those premises independently, including 768 bitset
partition cases; see the ledger for the qualification record and remaining gates.
