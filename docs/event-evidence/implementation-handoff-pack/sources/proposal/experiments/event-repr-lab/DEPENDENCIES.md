# Event belongs in the dependency language

The decision order is native synergy, algebraic elegance, then performance.
The real Free Join benchmark establishes a useful execution path. It does not
establish Event schema validation, admission, ownership or persistence. This
review records the production meanings a native implementation must preserve.

## What the source actually means

| Production source | Meaning to preserve | Event implementation consequence |
| --- | --- | --- |
| `crates/bumbledb-theory/src/schema.rs`, `ValueType` and `Side` | Structural value types; a containment side is a selected projection of one relation | Add Event as a structural value type. A typed full-space constant is a real projection extension, with checked scope resolution |
| `crates/bumbledb/src/schema/validate.rs`, `validate_functionality` | One interval position, last in a pointwise determinant | Generalize the recognized pointwise domain; do not interpret Event as scalar handle equality in this rule |
| Same file, `resolve_target_key` | The target projection must match an explicitly declared key's field set | Do not infer arbitrary superkeys or turn a composite closed-catalog projection into a legal target |
| `crates/bumbledb/src/schema/judge.rs`, `CandidateFacts` | Final-state input consists of distinct whole facts | Deduplicate exact full facts before counting competing contributors; equality of projected payloads is insufficient |
| Same file, `key_pointwise` and `key_group_pointwise` | Distinct facts with equal scalar prefix must have disjoint regions | Overlap is a structural violation even when its probability is zero |
| Same file, `containment_pointwise` | Each source region must be covered by the matching target union | A gap is `source & !covered`; coverage need not come from one target fact |
| Same file, `judge_complete` and `judge_incremental` | Complete final-state proof differs from checking affected groups under a lawful-parent premise | Judge the transaction's final candidate; an intermediate gap may be repaired within the transaction |
| `crates/bumbledb/src/plan/planner/tests.rs` | A pointwise key retains general fanout under a scalar prefix | A card can have many location facts across different worlds; do not license a scalar unique-row probe |
| `crates/bumbledb/src/plan/fj/tests/distinct_proof.rs` | A complete nonempty interval key can identify a fact; a point-membership filter is not a complete interval binding | Require complete Event binding plus nonemptiness evidence before transferring that witness; two distinct empty-valued facts may have equal regions |

The current implementations use sorted interval endpoints, adjacent-overlap
checks and maximal-run coverage. Arbitrary events have no such endpoint order.
The reusable contract is disjointness and coverage, plus contributor identity,
final-state discipline and diagnostics. Those contracts do not require a
probability law.

## The schema-level consequence

```rust
At(position, card, when) -> At;
PositionCard(position, card, true) -> PositionCard;
PositionCard(position, card, true) == At(position, card, when);
```

For a fixed card, the first statement forbids two distinct location facts from
holding in the same world. The mirror supplies every world. Therefore there is
exactly one location fact at every world. A normalized law subsequently measures
that partition and its masses sum to one.

This is proposed syntax. Today's `Side` contains field IDs, so `true` is not an
existing constant-projection feature. Its explicit key declaration must be
validated as a pointwise full-space face; the full space must be resolved from
a checked owner/alignment contract even when matching target rows are absent.
An absent roster cannot establish a universe by inference. Scope mismatches
must fail before any empty/full shortcut. The declaration is not permission
to bypass the engine's exact-target-key rule.

## One compositional summary for keys and coverage

For a set of distinct contributing facts, retain two Event regions:

```text
covered  = worlds owned by at least one contributor
conflict = worlds owned by at least two contributors
```

For disjoint contributor collections A and B, their summary combines as:

```text
covered(A+B)  = covered(A) | covered(B)
conflict(A+B) = conflict(A) | conflict(B) | (covered(A) & covered(B))
```

A leaf for one fact E is `(E, empty)`. The empty collection is
`(empty, empty)`. At each world this is addition of contributor counts saturated
at two, encoded as two nested regions. Consequently the merge is associative
and commutative. It is intentionally not idempotent: merging the same leaf
twice counts two contributors, so fact-set normalization precedes this summary.

This gives a compact internal algebra whose operands and answers are all
Events. A key holds exactly when `conflict` is empty. A containment holds when
every source event is included in the target's `covered` region. A balanced
tree can update either summary by recomputing a root path after insertion or
deletion. Removing E from a union by `covered & !E` is unsound: another fact may
still cover the same worlds. Rebuilding ancestors preserves that contribution.

Diagnostics retain contributor leaves. Intersect a leaf with `conflict` to
identify participating facts; descend only through subtrees whose coverage
intersects it. A containment gap belongs to its original source fact. Native
top-k citations still use canonical fact bytes, not arena IDs or encounter order.
This tree is an experimental candidate for admission, not a selected persistent
index or a claim that arbitrary Event operators have constant cost.

## Executable evidence and the remaining native boundary

[`dependencies.rs`](src/dependencies.rs) implements that summary and a balanced
array tree for all fourteen carriers. [The shared checker](results/dependency-initial-check.json)
passes 5,062 fact-set cases per candidate: every triple of four-world regions,
restricted supports, separate scalar groups, exact duplicates, overlapping
distinct facts with equal payload, absent branches and 64-bit boundaries.
It additionally checks 4,096 summary triples and 512 mixed leaf updates per
candidate. A direct per-world ownership oracle supplies the expected answers.
The deletion counterexample explicitly shows why subtracting the removed Event
from the old union loses still-covered worlds.

[`dependency_native.rs`](src/dependency_native.rs) provides the native bridge.
It uses the real `schema!` macro to declare two interval-valued relations, their
pointwise keys and a mirror. Each finite world w maps to `[w,w+1)`; adjacent
worlds coalesce only inside one original fact. A fixture origin rank outside
the projected key retains that whole fact's identity across all its runs.
The mapping preserves point ownership and coverage, so the expected complete
violated-statement set is unchanged. Native cited facts are checked against
the original offending contributor sets. The native judge may return a witness
subset; we do not mistake it for enumeration of every overlapping fact.

This bridge exercises real declaration, validation and enforcement. It also
checks that replacing the declared target pointwise key with a scalar key is
refused. It does not test the proposed typed `true` face, a native Event field,
Event object ownership or persistence. The shared checker does not compile
the native bridge. [The separate optimized native run](results/dependency-native-verification.json)
passes all 5,062 cases and validates 20,644 native citations against their
original offending facts. The source snapshot and build hashes are retained
in `results/dependency-src` and `results/build-dependency.json`.

Only after those checks should native Event storage and admission be wired in.
The join stage can reuse `covered` for Pack, but a Pack union does not establish
a pointwise key: its deliberate deduplication of worlds discards multiplicity.
The database should preserve exactly the information each proof needs.

The [storage-closure contract](../../event-storage.md) now admits empty Events
as stored fields. Such facts contribute neither coverage nor conflict, but
ordinary scalar constraints and queries still see them. The interval bridge
above cannot verify those row semantics: an empty Event expands to zero runs.
Ten [Lean reports](lean/EventStorage.lean) prove the denotational distinction
and the additional nonemptiness premise needed for complete-key distinctness.
Native Event persistence and planner integration remain acceptance obligations.

## The larger algebraic connection

`covered` and `conflict` are the first two threshold regions of a pointwise
contributor count. More generally let `C_k` mean “at least k distinct facts
hold here,” with `C_0 = full`. For disjoint contributor collections:

```text
C_k(A+B) = union_{i=0..k} (C_i(A) & C_(k-i)(B))
```

The identity follows pointwise from addition of nonnegative integer counts.
Our two-region summary is its saturation-at-two instance. Keeping additional
thresholds gives a possible implementation of the proposal's existing
cardinality operators: “two players could both block,” “exactly one legal
continuation,” or an Event-valued explanation of overcapacity. These answers
remain manipulable regions and can subsequently be measured under a chosen law.
This extension is derived, not benchmarked here.

## Admission is a compiled Event query

For one scalar determinant, let `I(world, fact)` mean that a distinct whole fact
holds at that world. This is a useful semantic view, not a request to materialize
worlds as ordinary rows. Coverage is `Domain(I)`. A pointwise key holds precisely
when `Converse(I);I` is included in the identity relation on **whole fact IDs**.
It says that two distinct facts cannot share a world.

The exact obstruction remains an Event:

```text
conflict(world) = exists fact_a, fact_b.
    I(world, fact_a) & I(world, fact_b) & fact_a != fact_b

gap(source_fact) = source_event & !union(matching_target_events)
```

The conflict can be computed by an ordinary self-join on the scalar determinant,
intersecting the two bound Events for distinct facts, then Pack-unioning the
results. The two-region tree above is a factorized evaluation of that query:
it avoids enumerating all contributor pairs. A containment uses the same grouped
union as Pack and returns the uncovered region. This connects the dependency
language, query algebra and optimized admission summary through one denotation.

Retain the world coordinate until the obstruction has been constructed.
`Converse(I);I` alone can prove whether the key holds, but loses where a conflict
occurred. If fact A holds at worlds 0 and 1 and fact B only at world 1, the
collision relation identifies A/B. Following it backward through A incorrectly
marks world 0 as conflicting. The precise conflict Event contains only world 1.
This is another reason to keep named faces and quantifier order in the plan.

[The independent finite checker](dependency_queries.py) verifies the incidence
criterion, self-join query and factorized summary over 4,096 triples, plus 65,536
source/target gap cases. It retains the lost-context counterexample and exact
fact deduplication cases in [raw evidence](results/dependency-query-checks.json).
These are semantic derivations; the native planner does not yet compile Event
admission queries. A proposed optimization must preserve the obstruction Event,
not only the final valid/invalid flag.

Ordinary native capacity statements still count or measure their declared
facts; they do not silently become pointwise Event capacities. Likewise, a
Free Join derivation occurring twice is not automatically two distinct source
facts. Contributor identity and the intended aggregation must be specified
before using a threshold summary. Pack remains idempotent union.
