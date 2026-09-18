# TypeSafe judgments that remain usable in queries

**Revision 0.5; source-construction and application proposal.** No TypeSafe
requests have been executed for these examples. Noul and Choice below refer to
the provider primitives; `event` remains the database type.

The purpose is to retain a judgment's alternatives and their connections until
a particular application decision needs a committed answer. A threshold at
every intermediate step loses those alternatives. Merely saving the numeric
probability also loses the overlap structure between different propositions.

## 1. What the provider actually supplies

The current [primitives](https://docs.typesafe.ai/primitives),
[Noul](https://docs.typesafe.ai/primitives/noul), and
[Choice](https://docs.typesafe.ai/primitives/choice) documentation specify:

- Noul is one probability of yes to a defined binary question. It has no
  separate confidence output. It is not a fuzzy degree of skill or membership.
- Choice includes the selected option and the full normalized option
  distribution. The distribution is the input to our partition constructor;
  selecting its maximum is an application decision.
- Score includes a distribution over described levels. Those levels can be
  retained as categorical event branches; interpreting their numbers as utility
  or distance is an additional application choice.
- Questions in one request see the supplied state and are evaluated separately.
  One answer is not hidden context for another. This does not assert statistical
  independence of the propositions or of model errors.
- Ask focused judgments and batch questions that use the same state. A dependent
  follow-up is appropriate when its input or options genuinely require an earlier
  result. The database algebra does not require a model call per derived fact.

Our [documentation snapshots and manifest](research/event-algebra/sources.json)
record the reading boundary. A typed probability is neither a calibration
certificate nor a joint distribution over every question.

## 2. Adopt one explicit meaning for each import

| Import intent | What is supplied | Chosen operation |
| --- | --- | --- |
| A new named binary/categorical judgment | Probability vector, question meaning, information case, source identity | Construct a normalized outcome partition; retain the whole response |
| A forecast conditional on existing cases | One option distribution for each case | Extend the joint source with the named conditional outcome |
| Constraints on an existing event | Bounds/equalities the application elects to impose | Restrict the admitted law family; report infeasibility, never silently repair |
| A replacement posterior assessment | Target probabilities on an existing partition, plus the commitment to preserve within-cell conditionals | Explicit Jeffrey revision into a new source view |
| A likelihood of a distinct observation | `P(observation | case)` and that observation's identity | Add the observation channel, then condition on its event; an explicit likelihood reweighting view is equivalent for stated queries |

There is **no default `Event::from(noul)` that silently makes the judgment
independent of existing facts**. An isolated binary judgment can have a local
space; combining it with other spaces requires a checked coupling. If the
question already names `bob_duke`, allocating an unrelated coin called
`bob_duke` would describe a different proposition.

For a finite family, several marginal constraints are combined simultaneously.
The exact feasible joint family is retained. Two marginals of 3/5 allow overlap
`t in [1/5,3/5]`; the event `A & !B` has mass `3/5-t`. It is not automatically
6/25. Shared constraints remain shared when this result is reused.

Source constraints may conflict with one another or with hard card conservation.
The result is a source conflict with an explanatory certificate where supported.
Dropping the less convenient assertion, rescaling the probabilities, or inventing
a confidence-based fusion rule would be a different operation.

## 3. Posterior probability and likelihood are different data

This question is now resolved using Jacobs (2019), §§4–5, and Jacobs–Stein
(2023), §§4–6. Both updates are available at the source layer; neither is
overloaded onto Event intersection.

For a normalized prior law mu and nonnegative likelihood L:

```text
z = sum_w mu(w) L(w)
mu_L(w) = mu(w) L(w) / z, where z>0
```

This is likelihood/Pearl updating. Sequential likelihoods multiply, and their
order commutes when they are fixed functions on the same context. Their product
asserts the appropriate joint observation likelihood; separate model outputs do
not justify that product. Reusing the same observation is not another likelihood
factor. The safest normal path keeps a named observation event and intersects
it once: `T & T = T`.

For a partition `{C_i}` and a replacement distribution `q_i`, Jeffrey revision
preserves each old within-cell conditional while adopting the new cell masses:

```text
mu_J(w) = sum_i q_i * mu(w | C_i)
```

Every positive q_i requires positive prior mass on C_i. Zero q_i contributes zero
without evaluating an undefined conditional. This operation does not create a
new independent draw or retain the old marginal as an additional constraint.
It is the declared modeling commitment that the new assessment changes only
the relative masses of the partition cells.

For one fixed prior, binary P(A)=1/5 and replacement q(A)=4/5:

```text
Jeffrey revision:                      new P(A) = 4/5
Treating (4/5,1/5) as likelihoods:       new P(A) = 1/2
```

That difference is large enough to change a Coup decision. A Noul answer is
documented as a posterior-style probability for its question, not a pair of
observation likelihoods. It must not silently take the second path.

Repeating the same Jeffrey revision on the same partition is idempotent under
the support conditions. Revisions on different overlapping partitions generally
do not commute. There is no join-order-dependent implicit revision policy.
Model questions that reuse the same underlying evidence are not independent
confirmations of one another.

For a law family, either operation is performed **fiber by fiber using the same
source parameters**. Retain the exact domain where all required denominators
are positive. Partial definedness is visible; do not invent a prior over the
parameters. A Jeffrey operation has no intrinsic evidence probability: retain
its old cell masses, targets, and revision receipt, rather than claiming that
an arbitrary virtual-evidence scale measures a real observation frequency.

An application that trusts a target posterior but cannot justify preserving old
within-cell conditionals can instead construct a replacement family constrained
by the target and the explicit retained structural assumptions. There is no
uniquely warranted joint posterior from the marginal alone.

## 4. Scope and support after revision

`Probability(E,G)` is a read-only observation in the original space; it does not
shrink Event's universe. A materialized conditioned/reweighted/revised source
is a new designated context, with its law and valid parameter domain recomputed.
Structural admissibility stays explicit. An identity-on-worlds map translates
old events when the structural world domain is unchanged. Restricting to evidence
or positive posterior support is an additional checked view; only that structural
restriction can merge previously distinct regions. Old keys retain their meaning.

With likelihood zero or Jeffrey target zero, those worlds receive zero posterior
mass but remain structurally possible unless explicitly excluded. Neither update
can assign positive mass to a cell impossible under its required prior
conditional. Revising that probabilistic zero needs an explicit replacement law,
not an epsilon hidden in the adapter. A model assigning zero to a legal move is
not a proof that the move is structurally impossible. See the
[admissibility decision](world-relations.md).

Named repeat draws remain distinct from copied judgments. Two future decisions
may share an unknown player tendency while having different action outcomes.
Retaining that shared parameter is essential when composing repeated decisions;
convexifying separate output summaries can erase it.

## 5. Coup: local inference, global consequences

The existing schema already has the necessary shape:

```rust
relation NextMove {
    decision: u64 as DecisionId,
    option: u64 as OptionId,
    when: event,
}
NextMove(decision, when) -> NextMove;
Decision(id, occurs) == NextMove(decision, when);
NextMove(decision, option, when) <= Available(decision, option, when);
```

The pointwise key and mirror make next-move branches a disjoint complete
partition of `occurs`. Normalization comes from the source constructor; the
schema proves branch structure. It does not certify the imported numeric law.

`PolicyCase(decision,id,input_digest,when)` partitions the reachable worlds by
the information the actor could use. `Forecast` retains each response. Construct
each option event by conditional source extension within those cases, then union
the matching option across cases. No threshold or winner selection is needed.

Forecast the next action from **pre-action** context. A player's private hand,
public history, and relevant memory belong in the case. Other players' actual
hidden cards do not. Cases the actor cannot distinguish must use the same
conditional policy; case IDs and execution order cannot reveal hidden worlds.
This factorization is a source contract, not something FD/IND syntax infers.

For the existing teaching example, Alice holds Duke and Assassin. The fair deck
gives Bob and Cleo each a Duke chance of `23/78`. Adopt the simplified conditional
Tax forecasts `4/5` with Duke and `1/5` without. After observing Bob declare Tax:

```rust
Probability(bob_duke, given & tax)  // 92/147, about 62.59%
Probability(cleo_duke, given & tax) // 5/21, about 23.81%
```

The second result requires no fresh model assessment of Cleo. A local behavior
judgment has consequences through shared physical cards. The returned events
can be intersected with a steal window, compared across positions, or inspected
through an information partition before choosing a move.

Other complete examples are in [Coup algebra applications](coup/algebra-applications.md):
uncertain target/certain expenditure; multiple simultaneous threats; what an
information case resolves; guaranteed continuation; and the value of a possible
observation under an explicit payoff table.

## 6. Concrete adapter receipts

Retain these as ordinary source metadata/relations, not Event generic parameters:

1. The named proposition or decision and its version; option/level identities.
2. Input state digest, question text/version and criteria, resolved model, raw
   response, and the information case. Question IDs are for code; the full
   question must be in the provider instructions.
3. The import intent from §2, predecessor source, and any posterior/likelihood
   interpretation assumptions. Record actual observation IDs separately from
   forecast request IDs.
4. The checked source extension or revision and its event translation map.
5. Reported confidence metadata where the primitive supplies it. No derived
   confidence interval or invented prior follows from that metadata.

Same request content can share cached provider output. It does not decide whether
two game decisions are the same random outcome. Conversely, reusing a response
for the same proposition must not create another independent piece of evidence.

## 7. First empirical slice

Keep one fixed Coup position, a finite roster of actor information cases, and
focused pre-action forecasts. Import all branches. Record one actual declaration
and query a different player's holding plus a compound event. Compare four
explicit baselines: thresholded answers, retained marginals with unknown coupling,
the adopted joint event source, and observed game outcomes when available.

Evaluate forecast quality on held-out decisions and inspect the consequences
of the import assumptions. Exact algebra can preserve and explain an imperfect
forecast; it cannot calibrate it by construction. API calls, credential setup,
data collection, and native implementation are separate work. This revision
specifies the experiment and its semantics without executing those actions.
