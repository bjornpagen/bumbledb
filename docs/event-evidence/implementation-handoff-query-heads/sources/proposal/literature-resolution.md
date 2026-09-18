# Literature resolution: names, repeated sampling, and the Allen analogy

> Source-process research from draft 0.3. Revision 0.8 defines the current Event
> contract and implementation baseline in [proposal.md](proposal.md).
> [The round-5 adjudication](research/algebra-resolution.md) added KAD, information
> abstraction, finite closure, explicit admissibility, and inference updates.
> The choices below concern source laws and historical alternatives, not another
> public field or a superseding limit on the Event operators.

**Research round 4; proposal draft 0.3.** The recommendation changes: the first-class object should retain named unknown laws and support repeated sampling. A finite convex set of output distributions is a useful view, but is insufficient as the reusable object. A finite qualitative relation table is not an adoption requirement. The required counterpart to Allen is an exact compositional semantics with equations, symmetries, and effective equality.

This is a research adjudication, not an implementation. The [active proposal](proposal.md) states the resulting design. Source PDFs, extracted text, and search evidence are retained locally; [the download manifest](review-evidence/literature-round-downloads.json) records URLs and SHA256 digests. The versions below are the downloaded versions, which sometimes differ from the year in the local filename.

## 1. The evidence that changes the carrier

### Named uncertainty is observable under composition

Jack Liell-Cock and Sam Staton, [*Compositional imprecise probability: A solution from graded monads and Markov categories*](https://arxiv.org/abs/2405.09391), arXiv **2405.09391v2**, 29 October 2024; POPL 2025, DOI **10.1145/3704890**. Read the central definitions and main arguments through §6, and the related-work discussion in §7. [Local PDF](papers/liell-cock-staton-2025-compositional-imprecise-probability.pdf).

Their `ImP` retains an uncertainty environment in the type of a computation. Its denotation is an open stochastic map. Two components can refer to the same uncertainty; their composition retains that correspondence.

**Theorem 5.6** says that passage to ordinary convex sets is op-lax: taking the credal view after composing can give a strict subset of composing the separate views. The latter allows choices to vary separately with intermediate outcomes. This is a semantic distinction, not a less precise numerical solver.

**Theorem 6.2** strengthens the result. A FinStochSurj-graded distributive Markov functor out of `ImP` that preserves the specified structure and permits recovery of its credal interpretation must be faithful. Within those hypotheses, further identification loses a compositional observation. The theorem is not a universal impossibility result about every conceivable probability language. It does directly refute the assumption that an output credal set is automatically a sufficient reusable denotation.

The paper's §1.1 deliberately uses a Knightian urn only once. §7.3 distinguishes its name semantics from the repeated-urn semantics of the Beta–Bernoulli work below. This qualification matters: the recent imprecise-programming lineage identifies the need for names, but does not alone settle what repeated use of one unknown rate means.

The local ICFP 2026 paper, [*Imprecise probabilistic programming, precisely*](https://arxiv.org/abs/2607.20801), makes that boundary explicit in its language: a Knightian name can be shared between exclusive branches, but duplicate use on one execution path is rejected. Its main semantics and restrictions were read in this round. We should retain the insight about named dependency and deliberately support a different operation, repeated draws from a named law.

**Decision:** source identity is part of the compositional interface. It cannot be treated solely as provenance or discarded when an intermediate interval happens to be unchanged.

### Repeated use already has an algebra, not just a statistical interpretation

Sam Staton, Dario Stein, Hongseok Yang, Nathanael Ackerman, Cameron Freer and Daniel Roy, [*The Beta-Bernoulli process and algebraic effects*](https://arxiv.org/abs/1802.09598), **1802.09598v2**, 15 May 2018; ICALP 2018, DOI **10.4230/LIPIcs.ICALP.2018.141**. Read the main paper, including the normal-form and completeness arguments. [Local PDF](papers/staton-et-al-2018-beta-bernoulli-algebraic-effects.pdf).

The interface distinguishes allocating a process from obtaining another result from it. Drawing one unknown bias from a Beta prior and repeatedly sampling it is observationally equivalent to a Pólya urn with updating counts. The paper gives exchangeability, discardability, convex-choice, and conjugacy equations; **Theorem 9** proves completeness for its measure semantics.

Crucially, the bias parameters may be **free**. The result is not restricted to choosing a Bayesian prior. **Proposition 7** identifies the finite-output, free-parameter fragment with unital maps whose coordinates are Bernstein polynomials with nonnegative rational coefficients. Repeated draws generate powers of the same variable. Independent allocations generate different variables. Their distinction survives normalization of the syntax.

For a Bernoulli rate `p`, the count basis is

```text
B[n,k](p) = choose(n,k) p^k (1-p)^(n-k).
```

Exchangeability groups paths by counts, and degree elevation moves different descriptions to a common basis. At a fixed degree those coefficients are unique. Degree elevation itself is an equality; there is no unique degree-free coefficient list without a further convention.

**Decision:** repeated sampling with a shared unknown law is normative. Bernstein coordinates and ordinary expanded polynomials provide reference forms for the finite sampling fragment. A supplied Beta prior is an additional probabilistic commitment, never the default meaning of ignorance.

### The polynomial observations have a representation theorem

Gert de Cooman, Erik Quaeghebeur and Enrique Miranda, [*Exchangeable lower previsions*](https://arxiv.org/abs/0909.1148), **0909.1148v1**; Bernoulli 15(3), 721–735, DOI **10.3150/09-BEJ182**. Read the main paper and its Bernstein appendix. [Local PDF](papers/decooman-quaeghebeur-miranda-2009-exchangeable-lower-previsions.pdf).

**Theorem 5**, printed p.732, represents a time-consistent family of exchangeable coherent lower previsions by a **unique coherent lower prevision on polynomial gambles on the categorical simplex**. Expectations of finite sampling experiments are precisely the polynomial observations on the unknown law. The representation also uniquely extends to continuous gambles; uniqueness is not claimed for all discontinuous gambles.

This supplies a deeper analogy than replacing two interval endpoints with a larger bound vector: the finite experiments generate the observation algebra of the unknown law.

The assumptions are material. Theorem 2 concerns finite exchangeability, represented through counts and sampling without replacement. Finite exchangeability alone does not justify an indefinitely reusable iid-given-a-law model. Our `repeat` constructor explicitly makes the latter assumption; arbitrary database observations do not acquire it from having the same marginal.

**Decision:** use polynomial experiments to explain the type's semantics, but do not claim that every coherent lower prevision has a finite description. The finite exact proposal below is a selected subtheory, not an encoding of all imprecise probability.

## 2. Conditioning and equality

Robin Piedeleu, Mateo Torres-Ruiz, Alexandra Silva and Fabio Zanasi, [*A Complete Axiomatisation of Equivalence for Discrete Probabilistic Programming*](https://arxiv.org/abs/2408.14701), **2408.14701v1**, 27 August 2024; published 2025, DOI **10.1007/978-3-031-91121-7_9**. Read the examples, semantic definitions, normal-form construction, and central completeness arguments. [Local PDF](papers/piedeleu-et-al-2025-complete-axiomatisation-discrete-probabilistic-programming.pdf).

**Example 1.2** is the decisive acceptance case. Two independent draws given the same `p`, conditioned on disagreement, return a fair first result for every `0<p<1`. Both surviving paths have weight `p(1-p)`. The guarantee follows from algebraic identity; no estimate of `p` is needed.

The **causal fragment**, Theorem 3.16 and Corollary 3.18 in this version, has a complete presentation for finite stochastic maps on Boolean powers. The **conditioned fragment**, Theorem 4.3, has a different equality: **Definition 2.11** and Proposition 2.13 use stochastic maps up to one global positive scalar, the category `FinProjStoch`. Remark 2.14 explicitly rules out using a separate scalar for every input.

We choose stricter equality because the database can query evidence mass. For example, a filter succeeds with probability `1` for a true input and `1/2` for a false input. Applied to an input with `P(true)=1/10`, the posterior input probability is `2/11`. Replacing each successful row by its normalized output first erases that likelihood ratio and yields `1/10`.

**Decision:** the reusable model keeps the full unnormalized kernel. A posterior view divides only after its input model has been supplied and preserves the evidence function alongside the posterior. The fair-coin extractor's posterior equals a fair coin; its reusable unnormalized model equals a fair coin multiplied by the success effect `2p(1-p)`. Those are different equality claims.

Ralph Sarkis and Fabio Zanasi, [*Graded String Diagrams for Imprecise Probability and Causal Intervention*](https://arxiv.org/abs/2501.18404), **2501.18404v4**, 8 January 2026, substantially extends the CALCO 2025 paper. Read the introduction, the relevant presentation statements, §5, and the discussions of composition and conditioning in §6. [Local PDF](papers/sarkis-zanasi-2025-graded-string-diagrams-imprecise-probability.pdf).

**Theorem 4.5** gives a modular presentation for the parameter construction; **Theorem 5.5** presents their Boolean graded imprecise category. This shows that named interfaces and complete equation theories are compatible. Two limits prevent citing it as a ready-made theorem for our entire design: §6.1 says the Boolean-power restriction lacks the coproducts needed for general if-hoisting; §6.2's account of conditioned ProbCirc does not preserve the original paper's distinction between raw substochastic and projective semantics. Our own matrix definitions, rather than an unqualified transfer of either theorem, own the proposed equalities.

Tobias Gürtler and Benjamin Lucien Kaminski, [*noDice: Inference for Discrete Probabilistic Programs with Nondeterminism and Conditioning*](https://arxiv.org/abs/2602.20049), **2602.20049v1**, 23 February 2026; DOI **10.1145/3798215**. Read the introduction, overview, and §3 syntax and semantics; this review does not rely on having checked the complete compilation proof or benchmarks. [Local PDF](papers/nodice-2026-nondeterminism-conditioning.pdf).

Its §3.2 supplies a concrete rival semantics. In a bind, the nondeterministic continuation can select a different distribution for each preceding result. Its `ExLet1` and `ExLet2` change from `[1/3,2/3]` to `[0,1]` when two choices change order. This is correct for an adaptive scheduler, but different from holding an unknown law fixed. The paper also retains unnormalized distributions before final conditioning.

**Decision:** no implicit adaptive scheduler. An outcome-dependent uncertain policy must appear as an explicitly indexed kernel or law table. Query evaluation order cannot silently grant uncertainty access to earlier draws.

## 3. Why the exact constraint envelope is semialgebraic

Arnaud Durand, Miika Hannula, Juha Kontinen, Arne Meier and Jonni Virtema, [*Probabilistic team semantics*](https://arxiv.org/abs/1803.02180). Read the introductory relational examples, the two-sorted arithmetic language, and §3.2's probability semantics and dependency atoms. [Local PDF](papers/durand-et-al-2018-probabilistic-team-semantics.pdf). Its distributions range over rational numbers; we do **not** transfer decidability of real arithmetic to that rational universe.

The fit to bumbledb is concrete. A probabilistic team is a distribution over assignments to a finite variable scope. Marginal identity is equality of corresponding marginal sums. Conditional independence is a polynomial constraint without division:

```text
P(x,y,z) P(x) = P(x,y) P(x,z), for every tuple.
```

This formula handles zero-mass conditioning rows without inventing their conditional distributions. Equal marginals, equal outcomes, and independence are different constraints. None can be inferred from ordinary row containment alone.

Duligur Ibeling and Thomas Icard, [*Probabilistic Reasoning across the Causal Hierarchy*](https://arxiv.org/abs/2001.02889), downloaded **2001.02889v5**, 2 June 2021; AAAI 2020. Read the framing, polynomial probability language, axiomatization, and central semialgebraic completeness argument. [Local PDF](papers/ibeling-icard-2020-probabilistic-causal-hierarchy.pdf).

**Theorem 6** gives sound and weakly complete finitary axiomatizations for its three languages. Its proof uses real polynomial feasibility and a Positivstellensatz. **Theorem 17** gives PSPACE decidability for its satisfiability languages; we do not attribute that complexity bound to our more general quantified model-equality problem. Only its association layer is needed here. A probability kernel does not by itself justify a causal intervention or counterfactual interpretation.

Our finite exact envelope is the following **synthesis**, not a theorem stated by these papers: parameter constraints and graphs of finite kernels are rationally defined semialgebraic sets. Products, finite sums, conjunction, disjunction, and real existential projection preserve that class. Normalization is expressed by polynomial equations with an explicit positive denominator guard. Totality, functionality, feasibility, inclusion, and equality are first-order questions over real closed fields. Quantifier elimination therefore gives complete decision specifications.

Unknown probabilities range over the **reals**, even though descriptions have rational coefficients. Restricting unknowns to rational solutions would invalidate this argument and exclude ordinary irrational probability laws. Exact answer endpoints may be real algebraic numbers. Open domains are permitted, and an infimum need not be attained.

General integration against an arbitrary continuous prior is not closed in this class. A restricted Beta moment constructor for polynomial experiments is exact; claiming a general integration operator would be a different proposal. This boundary follows from the mathematics, not performance.

## 4. The finite-table question is resolved

Frank Dylla, Till Mossakowski, Thomas Schneider and Diedrich Wolter, [*Algebraic Properties of Qualitative Spatio-Temporal Calculi*](https://arxiv.org/abs/1305.7345), **1305.7345v2**, 13 September 2013. Read the qualitative-calculus definitions and strong/weak composition distinction, with selective reading of the classification results. [Local PDF](papers/dylla-mossakowski-schneider-wolter-2013-algebraic-qualitative-calculi.pdf).

Definitions 3–7 distinguish exact relational composition from the smallest expressible qualitative overapproximation. A finite qualitative calculus chooses what distinctions to retain. A finite table is not the definition of a complete algebraic theory.

The proposal's five geometric comparison atoms already fail exact closure (`laws.md`, X08). This does not prove that every imaginable finite probability calculus is impossible. It establishes that a superficial partition is insufficient. More fundamentally, our chosen operations observe numeric identities such as `p(1-p)=p-p²` and quantify over constraints on continuously varying laws. A finite qualitative comparison table would be an additional abstraction of that theory, not its defining object.

**Decision:** stop searching for “the thirteen probability relations” as an acceptance condition. Require typed composition, a denotation that preserves dependencies, exact equality, canonical polynomial forms where applicable, and explicit complete decision procedures for the constraint envelope. A later qualitative abstraction must state whether its composition is strong or weak.

This follows the source's real Allen pattern: endpoint-order semantics determines the comparison signature and lookup implementation. The NEON lookup is a consequence of that particular theory. It does not prescribe the carrier of another theory.

## 5. Alternatives adjudicated

| Candidate | Verdict | Substantive reason |
| --- | --- | --- |
| Finite rational credal polytopes as the entire type | Reject as the reusable carrier; retain as an exact fragment | Shared-rate repetition has a curved family with infinitely many extreme points; output hulls erase live dependencies |
| Arbitrary closed convex output models | Retain as a mathematical observation theory | Removing the finite-basis restriction still does not recover named interfaces; conditioning can produce nonclosed sets |
| All coherent lower previsions on polynomial gambles | Semantic completion and explanation of repeated experiments | Natural and supported by an exchangeability theorem, but arbitrary members lack finite exact descriptions and a general decision procedure |
| Named polynomial kernels only | Use as the generative core and normal-form fragment | Excellent sampling equations; posterior and constraint projection require a larger exact envelope |
| Named semialgebraic kernels with scoped law constraints | **Choose** | Retains source sharing, closes under finite composition and guarded normalization, and gives exact containment and equality judgments |
| Adaptive convex nondeterminism, as in noDice | Separate semantics, not default | Decisions can depend on earlier draws; introduces intentional order sensitivity |
| Random-set/evidence algebra | Keep for explicit source-fusion constructors | Its convolution and Möbius duality are beautiful, but independent evidence fusion is not idempotent assertion or shared-law sampling |
| Finite qualitative probability table | Optional declared abstraction | Does not own the chosen quantitative observations and process identities |

## 6. What was actually verified

The theorem/equation pages for Beta–Bernoulli Proposition 7, exchangeable lower previsions Theorem 5, compositional maximality Theorem 6.2, projective semantics Definition 2.11, and the fair-coin example were rendered and visually inspected. Their PNGs are in `review-evidence/` as `beta-normal-form`, `exchangeable-representation`, `composition-maximality`, `projective-semantics`, and `fairness-example`.

The proposed envelope's closure and equality arguments are written in the updated law registry. The added exact checker covers polynomial identities, shared versus fresh rates, copied outcomes, evidence retention, Bernstein degree elevation, and the restricted Beta moment equations. These are reference checks and mathematical arguments, not a formally verified implementation of real quantifier elimination or a new completeness theorem for every future surface construct.
