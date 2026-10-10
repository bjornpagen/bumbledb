# Proposal 0002: probability as stored data and capacity laws

Status: **research first, nothing to implement.** No syntax below compiles.

## 1. Scope

bumbledb stores data and relations and runs fast operators over them. This
proposal brings probability into bumbledb on those terms only:

- **Worlds, events and weights are facts.** A world is a fact. An event is a
  set of worlds. A weight is a stored integer.
- **Distributions are laws.** "These weights sum to N" is a capacity law that
  the judge checks against stored facts at commit, like any other law.
- **Probabilities are query results.** `P(A)` is the sum of the stored weights
  of A's worlds, computed by ordinary operators.

**Out of scope, permanently:**
- unknown or partially specified weights;
- searching for a distribution consistent with stated bounds;
- consistency proofs over quantities that are not stored;
- inference, independence assumptions, sampling, fitting.

bumbledb never solves for anything. It checks stated laws against stored facts and computes sums.

## 2. A distribution is already a capacity law

A conditional distribution says that for each conditioning group, the outcome
weights sum to exactly one. With integer weights out of a denominator N, that
is an exact capacity window:

```math
\forall x:\ \sum_{y} w(y \mid x) = N
\qquad\Longleftrightarrow\qquad
X(id) \le_{[w]}\{N..N\}\ P(x)
```

- A single distribution is the one-group case, and a Markov kernel is the
  same statement grouped by the current state.
- Duration weights already measure time. "Fraction of the span" is the uniform
  measure on an interval, so the cookbook's calendar capacity recipe is a
  measure bounded by a total measure.
- An FD is the unit window `{0..1}`; a unit window `{0..hi}` caps each group
  at `hi` outcomes.

## 3. The model

```text
relation Universe { id: u64 }
relation World    { universe: u64, id: u64, weight: u64 }   // weight out of N
relation Member   { event: u64, universe: u64, world: u64 }

World(universe, id) -> World;
World(universe) <= Universe(id);
Member(event, universe, world) -> Member;
Member(universe, world) <= World(universe, id);

// the weights of each universe sum to exactly N
Universe(id) <=_[weight] {N..N} World(universe)
```

**The probability of an event is a query.** It is the sum of `World.weight` over the event's `Member` facts:
- divided by N at the output boundary;
- or returned as the exact pair (numerator, N).

**Conditioning is a query too.** It is the ratio of two such sums, each exact as an integer.

**Event partitions are laws.** A key on `Member(universe, world)` per partition family makes the events of a family disjoint. Containment in both directions makes the family cover the universe.

**Scoring stays queries.**
- Resolution is a fact naming the realized world.
- Brier and log scores are aggregate queries over resolved facts.
- Per-relation versions give point-in-time reads of what was forecast before resolution.

## 4. Fast operators

Every probability operation is an operator bumbledb already has, or a small kernel of the same family:

| Operation | Relational form | Dense form (an event as a bitmap over world indices) |
| --- | --- | --- |
| `P(A)` | join `Member` to `World` and fold-sum `weight` | masked sum of a weight column: the exact carry-counted u64 fold under a bitmap mask |
| `A ∩ B`, `A ∪ B`, `U \ A` | join, union and anti-join over `Member` | bitwise AND, OR and AND-NOT |
| `|A|` | count | popcount |
| Partition check | pointwise key | disjointness: AND equals zero per pair; cover: OR of the family equals full |

The dense form is the reason to consider an Event value at all. Its operators
are branch-free, vectorize under fearless_simd, and need no join. The
relational form needs no new type.

## 5. Gaps in today's language

1. **Dependent exact windows (ruling C6).** Marginal consistency needs each group's sum to equal a field on the target:

   ```math
   \sum_{y} w(x, y) = w(x)
   ```

   That is an exact window whose floor is a target field. C6 refuses dependent floors, so today bumbledb can state "at most the marginal" but not "equal to the marginal". A per-universe denominator is the same gap: `{N..N}` works when N is a literal, but not when N is a field of `Universe`.
2. **Weights carried along a path.** Path weights are refused. A membership fact that must carry its world's weight uses the pinned-copy containment idiom from the cookbook today.
3. **Event representation.** An event is either a relation of `Member` facts or a field holding a dense bitmap value over a named, fixed-size universe.

## 6. Research questions

1. **Lifting C6.** Why were dependent floors refused, and what breaks if exact dependent windows are allowed? The judge already computes each group's sum. What changes is the comparison against a target field, plus re-judging a group when that target field changes in the delta. Marginal consistency and per-universe denominators are the first consumers.
2. **Pinned weights.** Is the pinned-copy idiom acceptable for weights, or should a capacity weight be allowed to name a field one containment away?
3. **Event as relation or value.** Compare the two forms on:
   - law expressiveness (partitions as pointwise keys, coverage as pointwise containment);
   - operator speed (bitmap kernels against joins);
   - storage size;
   - the cost of changing one event.

   If a value type is chosen, it must be a fixed-size bitmap whose universe identity is part of the value. Operators across different universes are a typed refusal, never an implicit alignment.
4. **Weight domain.** Use u64 numerators over a declared denominator. Sums stay exact on the integer kernels, and "sums to exactly N" is a plain equality. F64 weights cannot be required to sum to exactly 1.0 in any useful way, so they stay ordinary data, not law operands.
5. **Output form.** Should a probability leave the engine as an exact (numerator, denominator) pair, as an F64 rounded once, or as both?

## 7. Prior work

- Lenzerini and Santucci (1983), cardinality constraints in the entity-relationship model.
- Ross, Srivastava, Stuckey and Sudarshan (1998), foundations of aggregation constraints.
- The cookbook's calendar capacity recipe (duration as a measure) and its pinned containment recipe (weights carried by copy).

## 8. Decision gates

1. **On paper first.** Write the model above, a Markov chain and a two-variable marginal in today's language. Record every statement that refuses and why. That list is the gap, measured instead of guessed.
2. **Decide C6 by itself.** It is small and useful without any of the rest.
3. **Decide the event representation** with a benchmark of the dense kernels against the relational form at realistic universe sizes.
4. **The scope gate holds for every later change.** A requirement that needs bumbledb to search for weights, infer a distribution, or prove anything about quantities it does not store is rejected at design time.
