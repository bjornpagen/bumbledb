# Exact probability query heads

The implementation branch now supports final `Probability(event, given)` heads
in Rust macros, native IR, raw Node queries and the TypeScript SDK. This is an
M5/M6 integration slice, not a release or completion of observation queries.

```rust
let query = bumbledb::query!(Game {
    (player, chance: Probability(duke & alive, evidence)) |
        Claim(player, duke, alive), Seen(player: player, evidence);
});
```

Both arguments accept the full Event expression grammar, including captured
maps and relation modalities. The names above stand for application relations
and Event columns. Event-producing interiors and Pack stages can feed a final
probability head. A probability is an owned query observation, not a stored
schema scalar and not an `f64`.

## Result contract

`AnswerValue::Probability(&ProbabilityAnswer)` retains `event()`, `given()` and
`value()`. Its value is one of:

- `ProbabilityValue::Fixed`: original `ProbabilityObservation`, exact numerator,
  evidence mass and an optional exact rational conditional value.
- `ProbabilityValue::Parameter`: original `ParameterProbabilityObservation`,
  exact numerator and evidence-mass functions, partial conditional function and
  its exact defined domain.

Zero evidence mass produces an impossible observation, including when the
evidence Event is nonempty. It does not erase the answer row or manufacture
zero. A parameter observation can be defined on only part of the ambient domain.
`is_impossible()` means it is defined nowhere, not merely that one endpoint is
excluded. A missing law, incompatible context or operational refusal is an error.

For two draws sharing an unknown bias `p`, observing their conjunction given the
first draw retains numerator `p²`, evidence mass `p`, and conditional function
`p` on `p > 0`. The hole at zero survives cancellation of the polynomial factor.
Neither a prior over `p` nor marginal independence is inferred.

## Relational execution and ownership

The computed head carries two registered Event keys: four ordinary binding
words in total. Free Join still enumerates matching relational bindings.
Projection, grouping and deduplication use the complete Event/evidence pair.
Two different propositions with probability `1/2` remain different observations;
duplicate derivations of one pair collapse under relational projection. Count
grouped by a probability head groups on its source pair, not its numerical value.

Every participating operand is checked before projection, including operands of
constant expressions. Both trees share one shape/descriptor budget. Missing-law
admission happens before other output producers can remove a binding. Unmatched
rows and unused Event columns do not become observation operands.

After pair deduplication, one exact-arithmetic budget covers contraction of all
final observations. Results are published only after every observation succeeds;
failure clears a fresh answer buffer or rolls an internal append back to its
already-initialized prefix. Owned answers retain their sources after query,
snapshot and database release. Sealed result paging copies or visits those owned
answers, without repeating database execution.

The Node worker serializes the existing BEVT/BERA/BESC/BEPR transports. One
arithmetic budget and a cumulative 16 MiB observation-payload limit apply per
delivery operation: a collection or one page. A failed delivery publishes no
partial batch and does not advance the page cursor. These limits do not claim a
complete engine retained-memory policy or a budget across separate page requests.

## TypeScript

```ts
const observed = query(Game).rule((r) => {
  const claim = v(Claim)
  return r.match(Claim, claim).find({
    player: claim.player,
    chance: probability(claim.duke, claim.evidence)
  })
})
const restored = queryFromDescription(Game, describeQuery(observed), {
  player: u64,
  chance: probabilityResult
})
```

`probability` accepts Event variables or `EventExpr` programs. `ProbabilityAnswer`
is discriminated by `law: "fixed" | "parameter"`. Fixed values have an exact
rational `value` or `null`. Parameter values have a partial `ParameterFunction`
and a `ParameterRegion` named `defined`. Both retain `event`, `given`, `numerator`
and `evidenceMass`. Existing exact APIs inspect and compose these owned values.
`probabilityResult` describes an imported query output; it is not a schema field.
Pure query construction and description import do not load the native addon.

## Evidence and remaining gates

Native tests enumerate all 256 pairs of four-world regions with a nonempty
zero-mass world on resident and cursor join paths. Shared-family queries persist,
reopen and retain their exact functions after owners are dropped. Other tests
check source-pair identity, repeated arms, aggregate grouping, participating
context errors, unused and unmatched operands, missing laws, atomic buffer reuse,
forced spill and append rollback, combined shape bounds and the current interior
refusal. SDK tests cover owned
description roundtrips, reopened family sources, retained results and sealed pages.

`QueryProbability.lean` adds fourteen checked reference reports for source-pair
retention, relational projection/union, complement mass partition, impossible
evidence and concrete shared-parameter counterexamples. These are reference
semantics, not a proof of the Rust compiler, registry, contraction or transport.

[Native observation stages](event-observation-stages.md) now retain probability
outputs in Rust interiors. The typed SDK still refuses observation interiors;
its query-variable domain must expand. Query arithmetic over observations,
general parameter solving and certified branch factoring remain open.
Exact rational/function payoff slots are implemented in
[Expectation heads](event-query-expectation.md). There is no new performance
claim or independent scalar-SQL oracle for observation heads.

[Final integer-payoff Expectation heads](event-query-expectation.md) now share the
execution and delivery budgets with Probability, retaining checked payoff rosters.
