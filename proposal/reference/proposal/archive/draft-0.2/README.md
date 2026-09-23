# A first-class probability algebra for bumbledb

**Active draft: [proposal.md](/Users/bjorn/Documents/bumbledb/proposal/proposal.md).** The current candidate is an information algebra of finite rational credal models with convex-semilattice operations for chance and unresolved choice. Probability intervals are an exact special case. The deciding criterion is algebraic unity, following the role of Allen's theory in the existing engine.

This directory is campaign scaffolding. Nothing here is a standing repository rule, and no engine implementation has begun. Keep the original research as evidence of the alternatives and their review; its earlier recommendations are not the active specification.

## Read in this order

| Document | Purpose |
| --- | --- |
| [proposal.md](/Users/bjorn/Documents/bumbledb/proposal/proposal.md) | Complete draft: denotation, constructors, operations, examples, relational type surface, admission, execution contract, alternatives, and adoption gates |
| [laws.md](/Users/bjorn/Documents/bumbledb/proposal/laws.md) | Law registry with proofs, theorem references, scope conditions, and twelve counterexamples |
| [decisions.md](/Users/bjorn/Documents/bumbledb/proposal/decisions.md) | Provisional decisions, competing algebras, actual review rounds, and concrete tests for unresolved choices |
| [spec-checks.py](/Users/bjorn/Documents/bumbledb/proposal/spec-checks.py) | Exact finite-model checks and falsifiers supporting the active draft |

Draft 0.2 incorporates the closure review, failed-rewrite examples, and source-fit review. The important open decisions are whether the type describes joint-outcome information or sampling processes, whether the strict Allen analogy requires a finite complete relation table, and how a law-bound model becomes first-class in the existing structural language.

## Validation

Run from the repository:

```sh
python3 proposal/spec-checks.py
```

The current run passes twelve groups, including 216 convex-choice triples, marginal-coupling checks against an extended constraint formulation, posterior checks against an independent half-space description, robust-decision examples, and explicit failed rewrites. All computations use exact rational arithmetic. Infinite-extreme, noncompactness, and universal-law claims are supported by written proofs/citations, not proved by finite enumeration. These are proposal checks, not engine tests or performance results.

The [saved check output](/Users/bjorn/Documents/bumbledb/proposal/review-evidence/spec-checks.json) records the individual groups and their limits.

## Supporting work

| Path | Status and contents |
| --- | --- |
| `algebra-analogy.md` | Conceptual pivot from practical interval boxes to the current algebraic criterion; developed and qualified by the active proposal |
| `review-astra.md` | Seven-section review reconciling the original synthesis with primary sources and source code; its practical-first recommendation was subsequently superseded |
| `review-checks.py` | Earlier exact checks for the review, including interval-box formulas and semiring counterexamples |
| `question.md` | Original brief and fixed engine constraints; read with the later clarification about algebraic beauty and the active proposal's explicit exposure decisions |
| `synthesis.md` | Original interval-first synthesis, retained for provenance; superseded as the working design |
| `lineages/01-imprecise-probability.md` | Credal models, lower previsions, boxes, capacities, and expectations |
| `lineages/02-belief-functions-subjective-logic.md` | Random sets, belief functions, opinions, and evidence models |
| `lineages/03-interval-deductive-databases.md` | Probabilistic logic, LP bounds, interval annotations, and bilattices |
| `lineages/04-semirings-provenance-datalog.md` | Annotation algebras, provenance, and conditional convergence results |
| `lineages/05-probabilistic-databases-compilation.md` | Dependency, lineage, compilation, credal circuits, and process semantics |
| `lineages/06-dependence-bounds-arithmetic.md` | Fréchet bounds, uncertain arithmetic, p-boxes, and dependence assumptions |
| `lineages/07-upstream-calibration.md` | Forecasts, confidence, external calibration, and prediction guarantees |
| `sources.md` | Bibliography and local paper filenames, including sources not fetched |
| `papers/` | Retained primary papers |
| `review-evidence/` | Live jev documentation snapshots, extracted theorem text, and inspected theorem pages |

The review and proposal do not silently promote an unavailable or unchecked reference into evidence. Source-specific qualifications remain in the review and bibliography.
