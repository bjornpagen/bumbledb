# Research adjudication: the Event operator and inference algebra

**Round 5, 17 September 2026.** The outcome is revision 0.5 of the proposal.
New PDFs and extracted text are retained under `papers/` and
`research/event-algebra/`; [the manifest](event-algebra/sources.json) records
versions, URLs, and hashes. Reading below is section-specific. Finite checks
validate examples and falsifiers; they are not theorem proofs or benchmarks.

## 1. The Boolean representation is not the algebra's capability boundary

Desharnais, Möller, Struth, *Kleene Algebra with Domain*,
[arXiv:cs/0310054v1](https://arxiv.org/abs/cs/0310054v1), October 2003;
later published in ACM TOCL 7(4), 2006.
[PDF](../papers/desharnais-moeller-struth-2003-kleene-algebra-domain.pdf).
Read the introduction and concrete-model framing, inspected §6's image/preimage
definitions and laws, and read §7's star/preimage and fixed-point arguments.

Struth, *On the Expressive Power of Kleene Algebra with Domain*,
[arXiv:1507.07246v2](https://arxiv.org/abs/1507.07246v2), 2 August 2015.
[PDF](../papers/struth-2015-expressive-power-domain.pdf).
Read §§1–4, especially Proposition 1's relational model, Theorem 1's distinction
from Kleene algebra with tests, and §4's weakest liberal preconditions.

Domain maps a relation/action back into its enabling predicate. Together with
tests, composition, and antidomain, it constructs modal preconditions rather
than requiring every useful intermediate assertion to be supplied externally.
The 2015 expressiveness theorem is about the stated abstract axiom systems;
it is not a complexity or completeness theorem for our implementation.

**Decision:** use the concrete relational model of KAD as the closest
program-algebra lineage. Include typed faces, converse, full relational
complement, and residuals as additional concrete structure. Include exact Star
and monotone fixed points on sealed finite stable state presentations. Boolean
combinations are one fragment, and ROBDDs are their shared region encoding.
Our All/Must distinction, partial-transition falsifier, and finite game
predecessors are derived from the explicit relational definitions. A finite
four-bit pair signature is useful, but is not the capability boundary or an
asserted strong qualitative composition table.

The concrete product contract is essential: endpoint supports form a full
fiber product over shared parameters, while transition restrictions are the
relation's region. A checked two-state example shows that clipping every
composition to an arbitrary restricted ambient relation can break associativity.
Another falsifier shows why left and right witnesses cannot select different
values of a supposedly shared parameter. These are representation obligations,
not consequences supplied automatically by calling a Boolean library.

**Admissibility decision:** a world is possible because it satisfies the
declared structural constraints, not because a model assigns it positive mass.
Revision 0.4's automatic positive-support quotient would collapse legal but
zero-probability moves and compromise structural guarantees. Revision 0.5
therefore uses explicit H(theta,omega), optional designated measurement, and an
explicit Supported(law) view. This is a design consequence of combining concrete
relational reasoning with measurement, not a theorem attributed to the KAD papers.

## 2. Choose a compilation language with known transformation costs

Darwiche and Marquis, *A Knowledge Compilation Map*, JAIR 17 (2002), 229–264;
[arXiv:1106.1819v1](https://arxiv.org/abs/1106.1819v1), uploaded 2011.
[PDF](../papers/darwiche-marquis-2002-knowledge-compilation-map.pdf).
Read the representation definitions, query taxonomy, and §5 transformation
definitions and results, especially Definitions 5.3–5.7 and Proposition 5.1.

The paper distinguishes consistency, validity, entailment, equivalence, model
enumeration, conditioning/substitution, and forgetting. Its fixed-order OBDD
language supports negation and bounded binary conjunction/disjunction; the
discussion following Table 7 explicitly warns that many-variable forgetting
does not have a general polynomial bound in the input OBDD size. Repeated
polynomial binary operations need not have a polynomial total output size.

**Revision 0.5 decision, physically reopened by the [0.6 Rust lab](../experiments/event-repr-lab/REPORT.md):**
retain the complement-pair ROBDD identity baseline. Add ITE,
structural tests, cardinality, checked maps, and quantification to the operator
contract. Do not interpret the paper's word “conditioning” (Boolean variable
substitution) as probabilistic normalization. Dense kernels are derived views.
This 2002 map is evidence for its listed languages, not a verdict about later SDDs.

## 3. Information abstraction has a relational definition

Kohlas, Casanova, Zaffalon, *Information Algebras of Coherent Sets of Gambles*,
[arXiv:2102.13368v1](https://arxiv.org/abs/2102.13368v1).
[PDF](../papers/casanova-kohlas-zaffalon-2021-information-algebras-coherent-gambles.pdf).
Re-read §6's set algebra, saturation definition and Theorem 5; inspected the
relational connection around Theorem 12. The claim used here is elementary set
saturation by observation fibres, not the full coherent-gamble representation.

Their operator groups worlds agreeing on selected information and saturates a
set across those groups. Our `May_O` is that saturation, with `Must_O` its
Boolean dual. The best observable upper/lower approximation laws follow from
whole-cell inclusion. Ordinary event partitions supply those cells.

**Decision:** expose same-space information abstraction through relations and
Boolean stages. Define evidence-relative reachability explicitly. We derived
and checked a three-world noncommutation counterexample on constrained support;
do not claim all coordinate-saturation laws of an unconstrained product space.
An epistemic reading also requires an appropriate observer universe and memory.

## 4. Noul is not automatically a likelihood

Bart Jacobs, *The Mathematics of Changing one's Mind, via Jeffrey's or via
Pearl's update rule*, [arXiv:1807.05609v3](https://arxiv.org/abs/1807.05609v3),
29 June 2019. [PDF](../papers/jacobs-2019-changing-ones-mind.pdf).
Read §§1–5, especially channel/state/predicate transformations in §4, Lemma 4.1,
the inversion support qualification in §5.1, and Definition 5.2/Proposition 5.3.

Bart Jacobs and Dario Stein, *Pearl's and Jeffrey's Update as Modes of Learning
in Probabilistic Programming*, [arXiv:2309.07053v2](https://arxiv.org/abs/2309.07053v2),
18 November 2023, doi:10.46298/entics.12281.
[PDF](../papers/jacobs-stein-2023-pearl-jeffrey-update.pdf).
Read the motivating program distinctions and §§4–6: the conditioning update,
dagger-based Jeffrey update, and their different operational sampling meanings.
No variational-inference theorem from §7 is required by this proposal.

A normalized posterior assessment and an observation likelihood have different
semantics. The former can support Jeffrey revision if retaining within-cell
conditionals is an explicit commitment; the latter reweights a prior and
retains observation likelihood. Neither import is justified by a scalar alone.
Zero-probability inverse-channel cases need guards, not fabricated conditionals.

**Decision:** distinguish new outcomes, conditional forecasts, constraints,
posterior revision, and likelihood observations. No generic “fuse” or implicit
independent coin import. Jeffrey same-partition idempotence is a derived finite
law; revisions on different partitions need not commute. Fixed likelihoods
commute but do not generally become idempotent. Evidence identity remains part
of the constructor. Neither paper proves model calibration.

## 5. Exact symbolic composition precedes numerical interpretation

Kimmig, Van den Broeck, De Raedt, *Algebraic Model Counting*,
[arXiv:1211.4475](https://arxiv.org/abs/1211.4475).
[PDF](../papers/kimmig-vandenbroeck-deraedt-2012-algebraic-model-counting.pdf).
Re-read Definitions 1–2, Table 1 and its sensitivity/gradient equations, circuit
properties, and Theorems 2–4 with their proof conditions.

The AMC problem sums over **models**, not arbitrary proof paths. The circuit's
decomposability, deterministic alternatives, and smoothness or neutral-label
conditions determine when an algebraic traversal is sound. Its gradient algebra
is a concrete route to exact numerical sensitivity through a valid contraction.

**Decision:** Boolean Event union handles overlapping derivations. Probability
and expectation are law contractions, with actual joint factors and coordinate
roles. Numerical differentiation applies to retained rational/polynomial source
functions on valid domains. It is not differentiation of the model's text API.

## 6. Free Join synergy is an execution opportunity with proof obligations

Abo Khamis, Ngo, Rudra, *FAQ: Questions Asked Frequently*,
[arXiv:1504.04044v7](https://arxiv.org/abs/1504.04044v7), 23 December 2023;
extended abstract PODS 2016. [PDF](../papers/abo-khamis-ngo-rudra-2016-faq.pdf).
Re-read §1's definitions, mixed aggregates and InsideOut motivation, with the
variable-order qualification. The local filename uses the conference year.

Wang, Willsey, Suciu, *Free Join: Unifying Worst-Case Optimal and Traditional
Joins*, SIGMOD 2023. [PDF](../papers/wang-willsey-suciu-2023-free-join.pdf).
Re-read the plan/factorization and COLT execution descriptions alongside native
`plan/fj.rs`, `exec/run.rs`, `api/prepared/computed.rs`, and `exec/sink.rs`.

FAQ gives the broader mixed-elimination perspective. Free Join's actual native
executor matches bindings; it is not already an arbitrary FAQ/semiring solver.
The current computed sink requires full bindings and forbids suffix skipping
and fused leaf scans. Event integration must preserve those error/coverage
requirements until stronger proofs permit more aggressive plans.

**Decision:** add explicit event stages first. Then exploit distributivity,
constant operands, union trees and source factorization with proofs. An ordinary
existence query can stop at one witness; an event union generally cannot. A
mixed sum/max/min plan cannot exchange its eliminations merely because each
individual operation has an algebraic implementation.

## 7. Preserve source identity and the causal boundary

Revisited Liell-Cock–Staton [2405.09391v2](https://arxiv.org/abs/2405.09391v2)
on named composition, Staton et al. [1802.09598v2](https://arxiv.org/abs/1802.09598v2)
on repeated shared-rate sampling, and Piedeleu et al.
[2408.14701v1](https://arxiv.org/abs/2408.14701v1) on conditioned computations.
The section/theorem qualifications remain in [the earlier adjudication](../literature-resolution.md).
Ibeling–Icard [2001.02889v5](https://arxiv.org/abs/2001.02889v5) supplies the
association/intervention/counterfactual distinction, not an automatic causal
meaning for a source channel.

**Decision:** maps, conditional observations, fresh draws, law revisions, and
interventions are distinct. Keep shared unknown parameters through expectation,
observation planning, and repeated decisions. Finite relational closure has
exact semantics under its stable-quotient contract. Arbitrary continuous state
transformations, general infinite-horizon stochastic recursion, and unrestricted
continuous integration require other constructors/theory. They are not silently
approximated by finite closure or graph traversal.

## 8. Resolved questions and remaining evidence

| Question | Resolution |
| --- | --- |
| Is Event just a preserved Noul number? | No: its denotation retains a region and source relationships; the adapter must supply those relationships |
| Which Boolean operations belong? | All sixteen binary operations, ITE, and derived cardinality; one canonical core |
| Is that the whole algebra? | No: relations on typed faces, composition, converse, domain, residuals, modalities, and finite fixed points |
| Does probability zero prove impossibility? | No: explicit admissibility is independent of mass; a supported-law restriction is an explicit view |
| Can a query construct a plan's missing conditions? | Yes: domain/preconditions and residuals compute regions and relations satisfying containment constraints |
| Can every hidden state choose its own action? | Only in a fully observed arena; partial-observation decisions must be uniform per admitted information case |
| Can queries prove something despite uncertain inputs? | Yes: exact structural tests, partition mapping, and nonvacuous Must |
| What does hiding information mean? | Explicit same-space May/Must or a checked change-of-space map; law marginalization is separate |
| How do model judgments update old events? | Explicit import intent; Jeffrey revision and likelihood conditioning are different |
| Are payoffs another probabilistic primitive? | No: ordinary value/event partitions, measured by exact expectation |
| Can we identify which uncertainty matters? | Unresolved information regions, parameter sensitivity, and observation value each have specified, different semantics |
| Does a conditional query model an intervention? | No; action semantics require a supplied transition/causal construction |
| Is the entire algebra a new proved-complete calculus? | No such claim: the finite semantic definitions and derived laws are ours, with cited subtheories and exact finite falsifiers |

Implementation and empirical questions remain: source-solver coverage, arena
growth, elimination order, observation-partition size, integration costs, and
forecast quality. They have explicit acceptance experiments in
[the plan](../implementation-plan.md); none requires leaving operator meaning
ambiguous. This revision adds no native implementation or measured inference
quality claim.
