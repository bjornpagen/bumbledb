# Coup: construct what can happen, then decide what to believe

**Revision 0.8; schemas, query semantics, and exact finite fixtures.** The existing
[Coup schema](schema.rs) remains the base. [Advanced query templates](algebra-queries.rs)
use its card, decision, option, and information-case relations. The final example
also specifies a small companion relation input for hypothetical game-rule
transitions. These are proposed APIs; no native query or TypeSafe call is executed.

The useful loop is: retain admissible alternatives, construct their consequences,
eliminate behavior that violates a chosen requirement, then use inference to
assess the remaining alternatives. Probability is one observation of that work.

## 1. An uncertain choice can have a certain consequence

Alice has ten coins. Bob and Cleo each have one live influence. Alice must Coup,
and both rivals are legal targets. Suppose an illustrative Choice distribution
favors Bob with 0.63 and Cleo with 0.37. Preserve both option events:

```text
B = Alice Coups Bob
C = Alice Coups Cleo
B & C = empty
B | C = occurs
```

The existing decision key and coverage mirror establish the last two equations.
The normalized conditional source supplies their masses. The game-rule mapping
maps **both** options to `coins_spent=7` and `influence_removed=1`. Pack equal
values, and the resulting observable has one branch covering occurs. Conditional
on the decision occurring, both consequences are guaranteed, even if the target
forecast is poorly calibrated. If occurs is proper, neither consequence is
asserted outside it.

This is stronger than storing a winner or averaging two targets. An ordinary
derived relation contains the certainty:

```rust
relation ActionCost { kind: u64 as ActionKindId, coins: u64 }
// Checked game-rule catalog includes (CoupAction, 7).
// Output relation: (decision, coins, when: event).
// NextMove ⋈ MoveOption ⋈ ActionCost → Pack by decision/coins.
```

Do not infer this from the action name alone; the cost catalog and transition
semantics are supplied game rules. For this fixture Coup is unchallengeable and
unblockable, its cost is paid, and each target has exactly one influence.

## 2. Every world has a winning action; Alice may still lack one

Continue the fixture. Exactly one rival holds the last live Assassin, and the
other holds a Duke. Alice knows those two roles remain but not who holds which.
Her goal is to remove the last live Assassin with this Coup.

| Admissible hidden position | Coup Bob | Coup Cleo |
| --- | --- | --- |
| Bob has Assassin; Cleo has Duke | Goal achieved | Goal missed |
| Bob has Duke; Cleo has Assassin | Goal missed | Goal achieved |

For each target a, a supplied rule relation `R_a: Before→After` contains all
its legal outcomes. Let E be the after-state Event “no live Assassin remains.”

```text
safe_a = Must(R_a,E)
```

That returns the before-state Event where this action is enabled and guarantees
E. Both safe events are nonempty and their union covers Alice's information case.
Yet neither one covers the entire case. There is no uniform action guaranteed
to achieve this goal. The exact distinction is:

```text
forall hidden s, exists action a: safe_a(s)      true
exists action a, forall hidden s: safe_a(s)      false
```

No amount of high-confidence Choice output changes that structural result.
If an observation distinguishes the two cases, each finer case admits a safe
target. The algebra can construct the missing information region and the
case/action relation, rather than ask the model to improvise whether a guarantee
exists. The fixture and quantifier-order counterexample are checked in
[algebra-checks.py](../algebra-checks.py).

## 3. Construct a complete permission relation

Let `I: Observation→Before` relate each visible information case to its
admissible hidden states. Let `Good: Before→Action` contain `(s,a)` exactly when
the rule action is enabled and every outcome meets E. Converse(I) runs from
hidden states to observations. Then:

```text
Allowed = LeftResidual(Converse(I), Good)   // Observation → Action
Permitted = Allowed restricted to Domain(I)
```

The residual computes the largest permission relation satisfying
`Converse(I);Allowed <= Good`. It therefore returns every action safe in every
hidden state compatible with that observation. Restricting to Domain(I)
excludes observations with no admissible state. Since Good already includes
enabledness, no unavailable action is accepted by vacuity.

This is the constructive counterpart of containment. It produces a relation
that can become an input to a Choice question. The model chooses among derived
permissions using context and preferences; the database owns the guarantee.
If there are no permissions, the output says the chosen requirement cannot be
guaranteed from that information. It does not secretly relax the goal.

For ordinary stored cell rows, the same query is `Subset(case & given, safe_a)`
plus a nonempty `case & given` test. [The templates](algebra-queries.rs) use that
staged spelling. It packs whole qualifying cases to make each action's Event.
Uniformity needs an observer-appropriate universe; Alice's private assumptions
cannot be reused as Bob's entire epistemic universe.

## 4. Multiple threats, information, and explanations

Pack the live holding events first. Then:

```rust
Event(Exactly(1, bob_assassin, cleo_assassin, dana_assassin))
Event(AtLeast(2, bob_captain, cleo_captain, dana_captain))
Event(Ite(bob_duke, bob_captain, cleo_captain))
```

The first counts **players who hold the role**, not physical role copies. A
different item roster counts physical cards. None of these formulas multiplies
marginal probabilities. ITE selects a condition by worlds; it does not execute
a model call inside either branch.

For an observation partition O and a threat E, `May_O(E)` selects cases where
the threat remains possible. `Must_O(E)` selects cases where it is guaranteed.
Their difference is the unresolved region. With evidence G, only cases meeting
G qualify, and tests are performed against `case & G`. This can drive a focused
follow-up: which observation would actually separate a useful unresolved case?

When a candidate guarantee fails, `witness(case & G & !safe_a)` gives a legal
counterexample. It might show a surviving Assassin at the other target. It is
an example of why the guarantee fails, not a sampled prediction. A sufficient
explanation is a separately checked condition included in safe_a.

PolicyCase is already a disjoint partition of Decision.occurs. The template's
use of it establishes what **that partition** resolves. Calling it a player's
knowledge additionally requires complete alternatives, correct visibility,
and retained memory in its construction. The macros cannot infer those contracts.

## 5. Local inference changes remote conclusions

The original three-player example still matters. With Alice holding Duke and
Assassin, a fair remaining deal gives Bob and Cleo each Duke probability 23/78.
The illustrative conditional forecasts are P(Tax|Duke)=4/5 and
P(Tax|no Duke)=1/5. After Bob's named Tax declaration:

```text
P(Bob Duke | Tax)  = 92/147
P(Cleo Duke | Tax) = 5/21
```

No model question about Cleo was needed. Card conservation and the shared joint
source transmit the consequences. Keep the Tax event, so further observations
and queries reuse the same evidence identity. The [adapter](../typesafe-inference.md)
also specifies how a Noul assessment about an existing event can revise a law
or constrain a family without minting an unrelated independent coin.

If all the adapter receives are two marginal Noul estimates, it cannot recover
their coupling. Retain an admitted coupling family or request/construct a
justified conditional relation. Porosity exposes the information that exists;
it does not manufacture information absent from the inputs.

## 6. Value information without granting secret knowledge

The hidden-Assassin fixture has a useful numerical version. Suppose the two
hidden positions are equally likely and utility is one for removing the last
Assassin, zero otherwise. Every fixed target has expected utility 1/2. A perfect
observation followed by a case-specific target gives utility one, so its value
before any observation cost is 1/2. A Choice forecast is not itself that perfect
observation. The model's confidence does not distinguish the hidden states.

Represent payoff as `(action,value,when)` rows, and pack equal values. Expectation
contracts those partitions with the retained joint law. The action maximum must
be taken per **visible case**, after integrating its hidden states. Taking a
maximum separately in each hidden state would incorrectly give Alice the answer
without the observation. With a law family, keep the same parameter assignment
through the whole comparison and state the robust policy criterion explicitly.

## 7. Legal sequences and strategies use the same carrier

Given a sealed finite transition presentation:

```text
May(Star(R),E)       some legal sequence reaches E
All(Star(R),E)       E holds now and throughout every legal sequence
μX. E | Must(R,X)    every maximal sequence reaches E
```

The last query rejects dead ends outside E and cycles that can avoid E. These
three questions differ. If Alice controls action selection while opponents
control responses, use the action-quantified predecessor specified in
[world-relations.md](../world-relations.md), not plain existential reachability.
If information is partial, a repeated strategy needs explicit information-state
updates and memory. A finite belief-set arena is possible; an unbounded transcript
or unbounded coin coordinate does not become a finite carrier by declaration.

This revision specifies closure semantics and a bounded fixture. It does not
claim to have solved all of Coup, invented a finite sufficient-memory quotient,
or measured an infinite-horizon stochastic winning probability.

## 8. Rule-view input contract

The advanced query file's final template uses an optional companion schema:

```rust
relation Scenario { id: u64, starts: event, goal: event }
relation RuleOption { scenario: u64, option: u64 }
relation RuleOutcome { scenario: u64, option: u64, when: event }
Scenario(id) -> Scenario;
RuleOption(scenario, option) -> RuleOption;
RuleOutcome(scenario, option) -> RuleOutcome;
RuleOption(scenario) <= Scenario(id);
RuleOutcome(scenario) <= Scenario(id);
RuleOption(scenario, option) == RuleOutcome(scenario, option);
```

`starts` is on the before presentation; `goal` is on the after presentation;
`when` is on their checked product. They therefore deliberately have different
spaces. A retained `step_faces` descriptor aligns their roles. One RuleOutcome
row per supplied option carries the full union of that option's outcomes.
An unavailable action stores empty. The scalar roster mirror requires one
outcome row per declared option, even when its Event is empty. Scenario starts
and goals also accept empty. The ordinary scalar key determines the outcome
row independently of the region's nonemptiness. A sparse variant may omit the
mirror and seed missing option groups explicitly in its query.

The rule constructor must certify completion over the declared options,
card/coin changes, and correspondence to the chosen game variant. The ordinary
keys/INDs enforce scalar identity and membership; they do not prove that the
transition relation implements Coup. Its product law is absent unless separately
supplied. No row weight or `Model<Outcome>` appears anywhere in this schema.
