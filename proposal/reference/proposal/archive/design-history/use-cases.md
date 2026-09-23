> Historical design research. The [active proposal](../../proposal.md) supersedes recommendations here.

# What this would let someone build with TypeSafe and bumbledb

**Historical process examples:** [nouls as events](../../event-surface.md) now supplies
the preferred field denotation and Rust surface. The examples below retain their
application assumptions; packed-model syntax is superseded.

Companion to draft 0.3. All probabilities, error rates, costs, and outputs below are **illustrative assumptions**, not measurements of TypeSafe. The schema example uses the current bumbledb authoring vocabulary; probability queries describe the proposed extension. No TypeSafe inference call or engine change was performed.

The live [TypeSafe primitives](https://docs.typesafe.ai/primitives) and [API](https://docs.typesafe.ai/api) were rechecked for this round. Choice supplies the entire option distribution, Score supplies the entire level distribution as well as its mean, and Noul supplies a yes/no probability. Several questions are evaluated separately against the same state. This does not make the underlying facts statistically independent. The snapshots and hashes are in [review-evidence/use-cases/](../../review-evidence/use-cases).

## A concrete operations schema

[operations-schema.ts](../../examples/operations-schema.ts) uses today's `relation`, `closed`, `closedId`, `key`, `contained`, and `interval(i64)` declarations. Its records are:

```text
Service(id, name, owner)
Incident(id, service, during)
Report(id, incident, origin, text)
Dependency(id, consumer, provider, active)
Source(id, model, promptRevision, evaluationSlice)
Assessment(id, incident, source, stateDigest, resolvedModel)
UsesReport(assessment, report, incident)
CauseMass(assessment, cause, reportedMass)
SeverityMass(assessment, severity, reportedMass)
OutageForecast(assessment, reportedProbability)
Audit(assessment, isWrong, randomSample)
ActionCost(action, cause, loss)
```

The closed Cause, Severity, and Action rosters type the corresponding coordinates. Report and assessment references are contained in their owners. Assessment IDs distinguish observations; source IDs identify declared shared reliability factors. Interval fields represent actual incident and dependency windows.

In this example `Audit.isWrong` audits the primary Choice classification, and its evaluation slice identifies that task. Reliability models for the separate Noul and Score answers require their own question/source bindings and audits; a common API request does not establish common accuracy or independent errors.

The `f64` fields retain the reported source outputs; they do not themselves enforce normalization, calibration, or a coherent joint model. The proposal adds an explicit model-construction/judgment stage. In particular, storing “the model returned this forecast” is not the same assertion as adopting that forecast as a hard constraint on reality.

A TypeSafe request could ask small, focused questions against a supplied incident packet:

```json
{
  "model": "jev-latest",
  "state": {
    "report": "Every payment attempt now fails; failures started after the API rollout.",
    "recent_changes": ["payments-api build 813 deployed"],
    "signals": ["database connections normal", "upstream gateway timeouts"]
  },
  "questions": {
    "failure_signature": {
      "type": "choice",
      "instructions": "Which listed failure signature best matches this packet?",
      "criteria": {
        "deploy": "A regression in the newly deployed application",
        "database": "Database access or saturation errors",
        "provider": "Failures at the external payment provider",
        "other": "Insufficient evidence or another failure signature"
      }
    },
    "reported_complete_blockage": {
      "type": "noul",
      "instructions": "Does the report say that every payment attempt fails?"
    },
    "reported_severity": {
      "type": "score",
      "instructions": "How severe is the issue described in the report?",
      "criteria": [
        "Cosmetic issue; normal operation remains available",
        "Degraded operation with a usable workaround",
        "Complete blockage of the reported operation"
      ]
    }
  }
}
```

This request is an example, not an actual response. A classifier of a report's wording and a predictor of the world's true state have different meanings. A production model must retain that distinction and, when needed, connect reported features to operational outcomes through validated likelihoods. `jev-latest` is a request alias; preserve the returned model identifier and the prompt/state provenance available from the API, and use a pinned model revision when the provider makes one available. An alias alone does not establish a permanently stable reliability source.

## 1. One verified mistake updates the risk of an entire batch

Consider a separate invoice-classification application with the same `Source`, `Assessment`, and `Audit` pattern. Add:

```text
Invoice(id, supplier, template, text)
Account(id, description, budgetOwner)                 closed roster
AccountMass(assessment, account, reportedMass)
AssessedInvoice(assessment, invoice)
```

TypeSafe Choice classifies invoices into the account roster. An externally evaluated reliability model says that one supplier template can be healthy or broken. Every invoice in this batch shares that hidden template state. Conditional on the state, classification errors are independent trials.

Use these illustrative values:

| Declared quantity | Value |
| --- | --- |
| Prior chance that this template is broken | 1% |
| Error probability when healthy | 0.1% |
| Error probability when broken | 90% |

Before any audit, each invoice has predicted error probability `0.999%`. A **randomly selected** invoice is audited against trusted ground truth and its classification is wrong. The shared template-fault posterior becomes `90.09%`; the predicted error probability for each other invoice using that template becomes `81.09%`.

Nothing has been rerun through the AI. The error-risk views change because the schema retains the common source of failure. A policy query can now return all unaudited invoices in that source group for review. It has not invented the correct replacement account labels.

If each invoice instead had its own independent failure source, the other invoices would retain their original `0.999%` error probability. That different result is a different declared model, not a switch chosen by an optimizer.

A duplicated join path to the same `Audit(assessment)` supplies one observation. Two distinct audited assessments supply two trials under the declared sampling model. Selection matters: a human who deliberately picks a suspicious invoice is not implementing this random-audit likelihood and needs a selection model.

**What earns the new type:** names preserve a shared latent law, conditioning revises that law, and every dependent query receives the revised consequences. Reliability is not guessed from TypeSafe's `confidence` field.

## 2. An uncertain classification can imply a certain action

Suppose an invoice's Choice distribution is:

```text
software_subscription: 34%
cloud_hosting:         33%
developer_tooling:     33%
```

The Account roster maps all three accounts to the same budget owner, `Engineering`. Ask for the owner distribution by joining to that roster and projecting the law onto `budgetOwner`. It is exactly:

```text
Engineering: 100%
```

The system can assign the review workflow to Engineering while retaining uncertainty about the eventual ledger category. It does not need an arbitrary confidence threshold on the winning category.

The strongest version uses a set of allowed distributions rather than one forecast: if every supported account maps to Engineering, the owner is certain throughout the allowed model. If an `other` option carries weight and maps elsewhere, the query preserves that possibility; the apparent certainty vanishes.

In the operations schema the same construction can map several plausible failure signatures or services to one on-call team or common first runbook step. The correct question is often “does uncertainty change this decision?” rather than “has the classifier picked a clear winner?”

**What earns the algebra:** deterministic pushforward and robust refinement. Existing closed-roster containment supplies the vocabulary and deterministic mapping.

## 3. The database chooses which diagnostic is worth running

Adopt an incident hypothesis distribution of `deploy=.55`, `database=.35`, `provider=.10`, `other=0` in this illustrative scenario. Store a total action-loss table. Loss is a common operational cost unit, not an ordinal severity index:

| True cause | Rollback loss | Failover loss |
| --- | ---: | ---: |
| Deploy | 2 | 20 |
| Database | 30 | 2 |
| Provider | 12 | 12 |
| Other | 30 | 30 |

Acting immediately gives expected losses `12.80` and `12.90` respectively.

There is a canary diagnostic: test the previous application build against the current environment. Its measured or otherwise explicitly adopted outcome model is:

```text
P(canary passes | deploy cause)   = .95
P(canary passes | database cause) = .05
P(canary passes | provider cause) = .05
P(canary passes | other cause)    = .50
cost of diagnostic               = 1
```

The database composes the diagnostic with the incident model, considers both outcomes, conditions on each, and compares the available follow-up actions. It returns the policy:

```text
run canary
  if it passes: rollback
  if it fails:  failover

expected loss including diagnostic: 4.985
expected improvement over acting now: 7.815
```

The expected cost is computed before the diagnostic is run. This is a finite decision query over data, not an LLM writing an unverified explanation of its favorite diagnostic. Other diagnostics can be compared using the same interface. With uncertain reliability parameters, use a declared robust decision criterion over their allowed values.

After a pass, the posterior deploy probability is `209/218≈95.87%`. The diagnostic's outcome likelihood is indispensable: simply asking TypeSafe to return a new normalized classification does not automatically provide that likelihood or a sound evidence-combination rule.

**What earns the algebra:** compositional kernels, retained evidence, branch-specific posterior decisions, and payoff evaluation. It makes an acquisition plan an inspectable mathematical object. Ordinary agent code executes the selected diagnostic only under the application's authorization policy.

## 4. Allen plus probability: a blast radius that changes over time

A report refers to “the gateway.” A scoped identity model says that this alias refers to Payments with probability `.60` and Identity with probability `.40`. Every reference to that alias in the incident shares one identity variable.

The known relational facts are:

```text
incident window:                     [10:20,10:40)
Checkout depends on Payments:        [10:00,10:30)
Checkout uses BackupPayments:        [10:30,11:00)
customer session:                    [10:25,10:35)
```

For this example the known dependency model gives no relevant Checkout→Identity path. The query asks about exposure to the named failed dependency, not whether exposure guarantees an observed request failure.

Allen intersection and the dependency graph derive the temporal pieces. The probability model evaluates whether the failed alias denotes a dependency active in each piece:

```text
[10:25,10:30)   P(session exposed to this incident) = .60
[10:30,10:35)   P(session exposed to this incident) = 0
```

The half-open boundary matters: the old provider is no longer active at 10:30. The uncertainty is shared across the first five minutes; it is not five independent 60% chances. The probability of any exposure is `.60`, while expected exposed duration is three minutes.

Join the same incident to 1,000 sessions. The database can derive each session's exposure and the distribution of a total affected-session count under the declared model. It must preserve the single identity/incident variable through that fan-out. Repeated copies of one assessment cannot become 1,000 pieces of supporting evidence.

If an operator later resolves the alias to Identity, all of these exposure views change together. Resolving it to Payments makes the first segment certain. This is a useful marriage of the existing temporal algebra and the proposed probability algebra: exact time boundaries and retained uncertainty identity in one query.

**What earns the algebra:** deterministic graph/temporal derivation plus pushforward of one shared uncertain assignment. Causal outage claims require an additional operational model; this example computes exposure.

## 5. Identical Score means can hide a 500-fold difference

Use TypeSafe Score levels `cosmetic=0`, `degraded=1`, `outage=2`. These distributions both have reported mean severity `1.0`:

| Report | Cosmetic | Degraded | Outage |
| --- | ---: | ---: | ---: |
| A | .50 | 0 | .50 |
| B | 0 | 1 | 0 |

Suppose the affected service has losses `0`, `100`, and `100000` respectively. Their expected losses are `50000` and `100`. A queue ordered only by the average severity treats them as equal. A query over the full probability distribution and the service's loss table does not.

The schema owns the utility mapping as payload data, so changing the service's business impact changes the decision without rewriting the TypeSafe question. If losses depend on incident duration or customer exposure, join the relevant interval-derived facts before taking expectation. The Score level number is a position in a rubric; it is not itself a monetary or operational loss.

This example needs the finite distribution/payoff fragment rather than the full shared-source machinery. It is still a compelling entry point because TypeSafe already returns the necessary distribution.

## 6. The database can expose incompatible AI assessments

On the same report and with aligned definitions, “the report describes complete blockage” implies “the report describes at least some impact.” Suppose one adopted Score forecast assigns `.70` probability to complete blockage, while a Noul forecast assigns `.10` to any impact. No joint distribution satisfies those two exact constraints and the implication.

Both raw forecasts can be stored as source outputs. The proposed coherence judgment rejects their promotion into one jointly asserted model and identifies the conflicting claims and support law. It must not secretly alter one forecast or multiply them as independent facts. An application can obtain more evidence, revise its calibration model, or explicitly retain alternative assessments.

**What earns the relational language:** support implications and probabilistic constraints participate in one exact consistency judgment. This is a database-level explanation of why the adopted claims cannot all hold, without pretending to know which AI answer is correct.

## What remains open

The proposal now takes definite positions on shared-law sampling and the finite-table criterion. That does not mean every design obligation is discharged.

1. **A satisfying canonical form for the full carrier.** The polynomial fragment has concrete normal forms. The semialgebraic envelope has an exact decidable equality judgment, but a natural canonical representation and the complete rewrite story for the combined surface remain to be specified. Calling that only a byte-layout problem would understate it.
2. **Concrete source binding and lifetime.** The specification must show how an immutable value captures source identities, how bindings survive joins and serialization, how model/prompt revisions create distinct sources, and how stronger source information is represented without silently mutating old value meanings. These belong in the next schema/change/query/log example.
3. **Per-application forecasting and evidence contracts.** A TypeSafe score does not supply a calibration prior, a likelihood table, or a dependency model. Each application must say which latent quantity a forecast describes and how its reliability/selection assumptions are obtained. These are model inputs with provenance, not missing algebra operators.

I would use the incident application as an implementation-driving vertical slice: three actual TypeSafe primitive shapes, a closed cause roster, source identity, evidence, decisions, temporal dependencies, and an inconsistency case. The invoice audit is a sharper isolated acceptance test for shared-source semantics. Neither requires implementing arbitrary model-valued recursion.
