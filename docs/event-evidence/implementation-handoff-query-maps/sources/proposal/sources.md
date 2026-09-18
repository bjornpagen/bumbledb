# Sources

## Constructive Event algebra and inference, revision 0.5

See [the adjudication](research/algebra-resolution.md) for section-specific reading,
theorem boundaries, and the decisions drawn from each source. The
[manifest](research/event-algebra/sources.json) retains URLs, versions and SHA256s.

- Desharnais, Möller, Struth. *Kleene Algebra with Domain.* [arXiv:cs/0310054v1](https://arxiv.org/abs/cs/0310054v1), 2003; later ACM TOCL 2006. [Paper](https://arxiv.org/pdf/cs/0310054v1). Concrete relations, domain, preimages, and star.
- Struth. *On the Expressive Power of Kleene Algebra with Domain.* [arXiv:1507.07246v2](https://arxiv.org/abs/1507.07246v2), 2015. [Paper](https://arxiv.org/pdf/1507.07246v2). Expressiveness relative to KAT and construction of weakest liberal preconditions.
- Darwiche, Marquis. *A Knowledge Compilation Map.* JAIR 2002; [arXiv:1106.1819v1](https://arxiv.org/abs/1106.1819v1), uploaded 2011. [Paper](https://arxiv.org/pdf/1106.1819v1). Bounded operations versus many-variable forgetting on OBDDs.
- Jacobs. *The Mathematics of Changing one's Mind, via Jeffrey's or via Pearl's update rule.* [arXiv:1807.05609v3](https://arxiv.org/abs/1807.05609v3), 2019. [Paper](https://arxiv.org/pdf/1807.05609v3). Channels, predicates, state revision, and positive-support guards for inversion.
- Jacobs, Stein. *Pearl's and Jeffrey's Update as Modes of Learning in Probabilistic Programming.* [arXiv:2309.07053v2](https://arxiv.org/abs/2309.07053v2), 2023. [Paper](https://arxiv.org/pdf/2309.07053v2). Distinct update modes and sampling meanings.

Revisited: Kohlas–Casanova–Zaffalon's set saturation, Kimmig et al.'s AMC circuit
conditions, the FAQ mixed-elimination definitions, Free Join's execution model,
and the named-source papers below. Current TypeSafe
[primitives](research/event-algebra/typesafe-primitives.md),
[Noul](research/event-algebra/typesafe-primitives-noul.md),
[Choice](research/event-algebra/typesafe-primitives-choice.md), and
[Score](research/event-algebra/typesafe-primitives-score.md) snapshots document
the provider boundary; they are not evidence that the proposed adapter was run.

## Representation and machine-code review, revision 0.4

See [representation notes](research/representation-notes.md) for exact reading scope,
local primary-source snapshots, compiler evidence, and derived-versus-checked claims.
The [source manifest](research/representation/sources.json) records CUDD internals,
CUDD Boolean implementation, and ARM ACLE Advanced SIMD documentation. The
[supplied brief](research/representation-brief.md) grounds the data-first design.

Every item consulted. Local PDFs are in `papers/`. Items marked *(not fetched)*
were paywalled, certificate-blocked, or dead links; their citations are given so
they can be pulled by hand.

## Named probability and exact constraint research, draft 0.3

Reading scope, theorem claims, and version-specific qualifications are in [literature-resolution.md](literature-resolution.md). The [download manifest](review-evidence/literature-round-downloads.json) records URLs and hashes; filenames retain discovery/publication years even when the downloaded version is newer.

- Liell-Cock, Staton. *Compositional imprecise probability: A solution from graded monads and Markov categories.* [arXiv:2405.09391v2](https://arxiv.org/abs/2405.09391v2), 29 October 2024; POPL 2025, doi:10.1145/3704890 → [paper](https://arxiv.org/pdf/2405.09391v2). Theorems 5.6 and 6.2; the urn model is explicitly one-use.
- Staton, Stein, Yang, Ackerman, Freer, Roy. *The Beta-Bernoulli process and algebraic effects.* [arXiv:1802.09598v2](https://arxiv.org/abs/1802.09598v2), 15 May 2018; ICALP 2018, doi:10.4230/LIPIcs.ICALP.2018.141 → [paper](https://arxiv.org/pdf/1802.09598v2). Proposition 7 and Theorem 9; free parameters, Bernstein normal forms, exchangeability and conjugacy.
- de Cooman, Quaeghebeur, Miranda. *Exchangeable lower previsions.* [arXiv:0909.1148v1](https://arxiv.org/abs/0909.1148v1), 7 September 2009; Bernoulli 15(3):721–735, doi:10.3150/09-BEJ182 → [paper](https://arxiv.org/pdf/0909.1148v1). Theorem 5's unique polynomial lower-prevision representation assumes time-consistent exchangeability.
- Piedeleu, Torres-Ruiz, Silva, Zanasi. *A Complete Axiomatisation of Equivalence for Discrete Probabilistic Programming.* [arXiv:2408.14701v1](https://arxiv.org/abs/2408.14701v1), 27 August 2024; published 2025, doi:10.1007/978-3-031-91121-7_9 → [paper](https://arxiv.org/pdf/2408.14701v1). Conditioned equality is global projective equality; it is not raw-kernel equality.
- Sarkis, Zanasi. *Graded String Diagrams for Imprecise Probability and Causal Intervention.* [arXiv:2501.18404v4](https://arxiv.org/abs/2501.18404v4), 8 January 2026 → [paper](https://arxiv.org/pdf/2501.18404v4). Expanded from CALCO 2025, doi:10.4230/LIPIcs.CALCO.2025.5; Theorems 4.5 and 5.5, and the limitations in §6.
- Gürtler, Kaminski. *noDice: Inference for Discrete Probabilistic Programs with Nondeterminism and Conditioning.* [arXiv:2602.20049v1](https://arxiv.org/abs/2602.20049v1), 23 February 2026; doi:10.1145/3798215 → [paper](https://arxiv.org/pdf/2602.20049v1). §3.2 supplies the order-sensitive adaptive alternative and unnormalized evidence semantics.
- Durand, Hannula, Kontinen, Meier, Virtema. *Probabilistic team semantics.* [arXiv:1803.02180v1](https://arxiv.org/abs/1803.02180v1), 6 March 2018 → [paper](https://arxiv.org/pdf/1803.02180v1). Finite assignment distributions, marginal identity, polynomial conditional independence; its rational-valued setting is not our real-closed-field universe.
- Ibeling, Icard. *Probabilistic Reasoning across the Causal Hierarchy.* [arXiv:2001.02889v5](https://arxiv.org/abs/2001.02889v5), 2 June 2021; AAAI 2020 → [paper](https://arxiv.org/pdf/2001.02889v5). Theorem 6 gives finitary weak completeness; Theorem 17's complexity belongs to its specific satisfiability languages.
- Dylla, Mossakowski, Schneider, Wolter. *Algebraic Properties of Qualitative Spatio-Temporal Calculi.* [arXiv:1305.7345v2](https://arxiv.org/abs/1305.7345v2), 13 September 2013 → [paper](https://arxiv.org/pdf/1305.7345v2). Strong versus weak qualitative composition and relation-algebra properties.
- Di Lavore, Román, Sobociński, Széles. *Order in Partial Markov Categories.* [arXiv:2507.19424](https://arxiv.org/abs/2507.19424); MFPS 2025, doi:10.46298/entics.16686 → [paper](https://arxiv.org/pdf/2507.19424). Read the introduction and core definitions; background for copy/discard/effects, not a claimed theorem transfer for the full proposal.

## Algebraic foundations added during the review

- Bonchi, Sokolova, Vignudelli. *Presenting Convex Sets of Probability Distributions by Convex Semilattices and Unique Bases.* CALCO 2021, LIPIcs 211, 11:1–11:18. doi:10.4230/LIPIcs.CALCO.2021.11 → `papers/bonchi-sokolova-vignudelli-2021-convex-semilattices-unique-bases.pdf`. Theorems 1 and 3 supply unique bases and the real-weight convex-semilattice presentation; the rational restriction is derived in `laws.md`, P02.
- Kohlas, Casanova, Zaffalon. *Information algebras of coherent sets of gambles.* arXiv:2102.13368v1, 2021 → `papers/casanova-kohlas-zaffalon-2021-information-algebras-coherent-gambles.pdf`. Theorem 3, §7, and Theorem 12 support the information-algebra and generalized relational-algebra connection. Use the coherent-lower-prevision/strictly-desirable formulation with the qualifications recorded in the review.

Extracted text and inspected theorem-page images are retained in `review-evidence/`.

## Imprecise probability (lineage 01)

- de Campos, Huete, Moral. *Probability intervals: a tool for uncertain reasoning.* IJUFKS 2(2):167–196, 1994. doi:10.1142/S0218488594000146. *(not fetched; summarized via Destercke et al. and secondary sources)* — https://www.worldscientific.com/doi/abs/10.1142/S0218488594000146
- de Campos, Huete, Moral. *Uncertainty management using probability intervals.* IPMU 1994. https://link.springer.com/chapter/10.1007/BFb0035950
- Walley. *Statistical Reasoning with Imprecise Probabilities.* Chapman & Hall, 1991. *(book)*
- Walley. *Inferences from multinomial data: learning about a bag of marbles.* JRSS-B 58(1), 1996. doi:10.1111/j.2517-6161.1996.tb02065.x *(not fetched)*
- Bernard. *An introduction to the imprecise Dirichlet model for multinomial data.* IJAR 2005. https://www.sciencedirect.com/science/article/pii/S0888613X04001069 *(not fetched)*; tutorial slides → `papers/bernard-2006-imprecise-dirichlet-model-tutorial-slides.pdf`
- Hutter. *Practical robust estimators for the imprecise Dirichlet model.* arXiv:0901.4137 → `papers/hutter-2009-idm-robust-estimators.pdf`
- Weichselberger. *The theory of interval-probability as a unifying concept for uncertainty.* IJAR 24:149–170, 2000. https://www.sciencedirect.com/science/article/pii/S0888613X00000323 *(not fetched)*
- Miranda. *A survey of the theory of coherent lower previsions.* IJAR 48(2), 2008. https://www.sciencedirect.com/science/article/pii/S0888613X07001867 *(not fetched)*
- Troffaes & de Cooman. *Lower Previsions.* Wiley 2014. *(book)*
- de Cooman, Troffaes, Miranda. *n-Monotone exact functionals.* arXiv:0801.1962 → `papers/decooman-troffaes-miranda-2008-n-monotone-exact-functionals.pdf`
- de Cooman, Quaeghebeur, Miranda. *Exchangeable lower previsions.* See the fetched Bernoulli version, arXiv:0909.1148v1, in the draft 0.3 references above.
- Pelessoni & Vicig. *2-coherent and 2-convex conditional lower previsions.* arXiv:1606.06043
- Destercke, Dubois, Chojnacki. *Unifying practical uncertainty representations I: generalized p-boxes.* IJAR 2008; arXiv:0808.2747 → `papers/destercke-dubois-chojnacki-2008-generalized-p-boxes.pdf`
- Troffaes & Destercke. *Probability boxes on totally preordered spaces for multivariate modelling.* arXiv:1103.1805
- Cuzzolin. *Uncertainty measures: the big picture.* arXiv:2104.06839 → `papers/cuzzolin-2021-uncertainty-measures-big-picture.pdf`
- Vu et al. *Upper entropy for 2-monotone lower probabilities.* arXiv:2603.23558 → `papers/upper-entropy-2-monotone-lower-probabilities-2026.pdf`
- *Propagation of 2-monotone lower probabilities on an undirected graph.* arXiv:1302.3569
- *Calculating uncertainty intervals from conditional convex sets of probabilities.* arXiv:1303.5418 → `papers/uncertainty-intervals-conditional-convex-sets.pdf`
- Wang & Klir. *Choquet integrals and natural extensions of lower probabilities.* IJAR 1996. https://www.sciencedirect.com/science/article/pii/S0888613X96000783 *(not fetched)*
- Cozman. *Credal sets tutorial.* http://sites.poli.usp.br/p/fabio.cozman/research/credalsetstutorial/
- *A domain-theoretic foundation for imprecise probability and credal sets.* arXiv:2604.09272 → `papers/domain-theoretic-foundation-imprecise-probability-2026.pdf`

## Belief functions and subjective logic (lineage 02)

- Jøsang. *Belief calculus.* arXiv:cs/0606029 → `papers/josang-2006-belief-calculus.pdf`
- Jøsang. *Subjective Logic: A Formalism for Reasoning Under Uncertainty.* Springer 2016. *(book)*
- Jøsang. UAI 2016 tutorial slides → `papers/josang-2016-subjective-logic-uai-tutorial-slides.pdf`; FUSION 2022 tutorial https://www.mn.uio.no/ifi/personer/vit/josang/sl/subjective-logic-fusion-2022.pdf
- Jøsang, Wang, Zhang. *Multi-source fusion in subjective logic.* FUSION 2017 → `papers/josang-wang-zhang-2017-multi-source-fusion.pdf`
- van der Heijden, Kopp, Kargl. *Multi-source fusion operations in subjective logic.* arXiv:1805.01388 → `papers/vanderheijden-kopp-kargl-2018-multi-source-fusion-subjective-logic.pdf`
- Sensoy, Kandemir, Kaplan. *Evidential deep learning to quantify classification uncertainty.* NeurIPS 2018; arXiv:1806.01768 → `papers/sensoy-kandemir-kaplan-2018-evidential-deep-learning.pdf`
- Ulmer et al. *Prior and posterior networks: a survey on evidential deep learning.* arXiv:2110.03051
- *Multimodal learning with uncertainty quantification based on discounted belief fusion.* arXiv:2412.18024
- Ferson, Kreinovich, Ginzburg, Myers, Sentz. *Constructing probability boxes and Dempster-Shafer structures.* SAND2002-4015 → `papers/ferson-kreinovich-2003-constructing-p-boxes-ds-structures-sandia.pdf`
- Mubashar, Manchingal, Cuzzolin. *Random-set large language models.* arXiv:2504.18085 → `papers/random-set-large-language-models-2025.pdf`

## Interval deductive databases and probabilistic logic (lineage 03)

- Hailperin. *Best possible inequalities for the probability of a logical function of events.* Amer. Math. Monthly 72, 1965. *(not fetched)*
- Hailperin. *Boole's Logic and Probability.* North-Holland 1986. *(book)*
- Nilsson. *Probabilistic logic.* Artif. Intell. 28(1):71–87, 1986. *(not fetched)*
- Georgakopoulos, Kavvadias, Papadimitriou. *Probabilistic satisfiability.* J. Complexity 4, 1988. *(not fetched)*
- Hansen & Jaumard. *Probabilistic satisfiability.* Handbook of Defeasible Reasoning vol. 5, 2000. https://link.springer.com/chapter/10.1007/978-94-017-1737-3_8 *(not fetched)*
- Finger & De Bona. *Probabilistic satisfiability: logic-based algorithms and phase transition.* IJCAI 2011 → `papers/finger-debona-2011-probabilistic-satisfiability-algorithms.pdf`
- Lukasiewicz. *Probabilistic deduction with conditional constraints over basic events.* JAIR 10:199–241, 1999; arXiv:1105.5461 → `papers/lukasiewicz-1999-conditional-constraints-basic-events.pdf`
- Lukasiewicz. *Probabilistic logic programming with conditional constraints.* ACM TOCL 2(3), 2001. https://dl.acm.org/doi/10.1145/377978.377983 *(not fetched)*
- Lukasiewicz. KR 2016 tutorial slides → `papers/lukasiewicz-2016-probabilistic-reasoning-kr-tutorial-slides.pdf`
- Ng & Subrahmanian. *Probabilistic logic programming.* Inf. & Comput. 101(2), 1992. *(not fetched)*
- Lakshmanan, Leone, Ross, Subrahmanian. *ProbView: a flexible probabilistic database system.* ACM TODS 22(3):419–469, 1997. https://dl.acm.org/doi/10.1145/261124.261131 *(not fetched)*
- Lakshmanan & Sadri. *On a theory of probabilistic deductive databases.* TPLP 2001; arXiv:cs/0312043 → `papers/lakshmanan-sadri-2001-probabilistic-deductive-databases.pdf`
- Ginsberg. *Multivalued logics: a uniform approach to inference in AI.* Comput. Intell. 4, 1988. *(not fetched)*
- Fitting. *Bilattices and the semantics of logic programming.* J. Logic Programming 11:91–116, 1991. *(not fetched)*
- Fitting. *Kleene's logic, generalized.* → `papers/fitting-kleenes-logic-generalized.pdf`
- Fitting. *Bilattice basics.* FLAP 7(6), 2020. *(not fetched)*
- Denecker, Marek, Truszczyński. *Approximation fixpoint theory.* 2000. *(not fetched)*
- Charalambidis et al. *Non-monotone fixpoint theory based on the structure of weak bilattices.* KR 2024.
- Dubois & Prade. *Possibility theory, probability theory and multiple-valued logics: a clarification.* AMAI 32:35–66, 2001. *(not fetched)*
- Bonissone. *Selecting uncertainty calculi and granularity.* arXiv:1304.3425

## Semirings, provenance, Datalog (lineage 04)

- Green, Karvounarakis, Tannen. *Provenance semirings.* PODS 2007 → `papers/green-karvounarakis-tannen-2007-provenance-semirings.pdf`. Reread §2–3 for the event revision: the event-table instance is `(P(Omega), union, intersection, empty, Omega)`, with projection by union and join by intersection. [Paper](https://doi.org/10.1145/1265530.1265535). This supplies relational event algebra, not bumbledb's proposed FD/IND extension.
- Karvounarakis & Green. *Semiring-annotated data: queries and provenance.* SIGMOD Record 41(3), 2012.
- Green & Tannen. *The semiring framework for database provenance.* PODS 2017.
- Grädel & Tannen. *Semiring provenance for first-order model checking.* arXiv:1712.01980 → `papers/gradel-tannen-2017-semiring-provenance-first-order.pdf`
- Bourgaux, Ozaki, Peñaloza, Predoiu. *Revisiting semiring provenance for Datalog.* KR 2022 → `papers/bourgaux-2022-revisiting-semiring-provenance-datalog.pdf`
- Badia, Kolaitis, Noguera. *Codd's theorem for databases over semirings.* PODS 2025; arXiv:2501.16543 → `papers/badia-kolaitis-noguera-2025-codd-theorem-semirings.pdf`
- Abo Khamis, Ngo, Rudra. *FAQ: questions asked frequently.* PODS 2016; arXiv:1504.04044 → `papers/abo-khamis-ngo-rudra-2016-faq.pdf`
- Abo Khamis, Ngo, Pichler, Suciu, Wang. *Convergence of Datalog over (pre-) semirings.* PODS 2022, JACM 2024; arXiv:2105.14435 → `papers/abo-khamis-ngo-pichler-suciu-wang-2022-convergence-datalog-semirings.pdf`
- Im, Moseley, Ngo, Pruhs. *On the convergence rate of linear Datalog° over stable semirings.* ICDT 2024; arXiv:2311.17664 → `papers/convergence-rate-linear-datalog-stable-semirings-2024.pdf`
- Fan, Koutris, Roy. *Circuits and formulas for Datalog over semirings.* PODS 2025; arXiv:2504.08914 → `papers/circuits-and-formulas-datalog-semirings-2025.pdf`
- Wang, Willsey, Suciu. *Free Join: unifying worst-case optimal and traditional joins.* SIGMOD 2023; arXiv:2301.10841 → `papers/wang-willsey-suciu-2023-free-join.pdf`
- Huang, Li, Chen, Samel, Naik, Song, Si. *Scallop: from probabilistic deductive databases to scalable differentiable reasoning.* NeurIPS 2021 → `papers/huang-li-2021-scallop-neurips.pdf`
- Li, Huang, Naik. *Scallop: a language for neurosymbolic programming.* PLDI 2023; arXiv:2304.04812 → `papers/li-huang-naik-2023-scallop-language.pdf`
- Kimmig, Van den Broeck, De Raedt. *Algebraic model counting.* arXiv:1211.4475; J. Applied Logic 2017 → `papers/kimmig-vandenbroeck-deraedt-2012-algebraic-model-counting.pdf`. Reread §3 for the conditions that justify arithmetic evaluation of event formulas; arbitrary union does not license addition of marginal probabilities. [Paper](https://arxiv.org/pdf/1211.4475).
- Derkinderen, Manhaeve, Zuidberg Dos Martires, De Raedt. *Semirings for probabilistic and neuro-symbolic logic programming.* arXiv:2402.13782 → `papers/derkinderen-2024-semirings-probabilistic-neurosymbolic-lp.pdf`
- Hardouin, Cottenceau, Lhommeau, Le Corronc. *Interval systems over idempotent semiring.* LAA 2009; arXiv:1306.1136 → `papers/interval-systems-idempotent-semirings-2009.pdf`
- Yorgey. *The network reliability problem and star semirings.* Blog, 2016. https://byorgey.wordpress.com/2016/04/05/the-network-reliability-problem-and-star-semirings/
- *Semiring provenance for lightweight description logics.* arXiv:2310.16472
- Ngo. FAQ/InsideOut slides. https://cse.buffalo.edu/~hungngo/papers/faq-insideout.pdf *(dead link, 404)*

## Probabilistic databases and compilation (lineage 05)

- Suciu, Olteanu, Ré, Koch. *Probabilistic Databases.* Synthesis Lectures on Data Management 16, 2011. *(book)*
- Van den Broeck & Suciu. *Query processing on probabilistic data: a survey.* FnT Databases 7(3–4), 2017. *(not fetched)*
- Dalvi & Suciu. *The dichotomy of conjunctive queries on probabilistic structures.* PODS 2007; arXiv:cs/0612102 → `papers/dalvi-suciu-2007-dichotomy-conjunctive-queries.pdf`
- Dalvi & Suciu. *Management of probabilistic data: foundations and challenges.* PODS 2007 keynote → `papers/dalvi-suciu-2007-management-of-probabilistic-data-pods-keynote.pdf`
- Dalvi & Suciu. *The dichotomy of probabilistic inference for unions of conjunctive queries.* JACM 59(6), 2012. *(not fetched)*
- Kenig & Suciu. *A dichotomy for the generalized model counting problem for unions of conjunctive queries.* arXiv:2008.00896
- Amarilli & Ceylan. *The dichotomy of evaluating homomorphism-closed queries on probabilistic graphs.* arXiv:1910.02048
- Jha & Suciu. *Knowledge compilation meets database theory.* ICDT 2011; ToCS 52(3), 2013. https://dl.acm.org/doi/10.1145/1938551.1938574 *(not fetched)*
- Jha & Suciu. *Probabilistic databases with MarkoViews.* arXiv:1208.0079
- Olteanu. *Probabilistic databases.* Tutorial slides, 2017 → `papers/olteanu-2017-probabilistic-databases-tutorial-slides.pdf`
- Cozman & Mauá. *On the semantics and complexity of probabilistic logic programs.* JAIR 60, 2017; arXiv:1701.09000 → `papers/cozman-maua-2017-semantics-complexity-plp.pdf`
- Cozman & Mauá. *The structure and complexity of credal semantics.* PLP@ILP 2016. http://sites.poli.usp.br/p/fabio.cozman/Publications/Article/cozman-maua-plp2016.pdf *(not fetched: certificate error)*
- Cozman & Mauá. *The joy of probabilistic answer set programming.* IJAR 125, 2020. http://sites.poli.usp.br/p/fabio.cozman/Publications/Journal/cozman-maua-ijar2020.pdf *(not fetched: certificate error)*
- Mauá & Cozman. *Complexity results for probabilistic answer set programming.* IJAR 118, 2020.
- Tuckey, Russo, Broda. *PASOCS: a parallel approximate solver for probabilistic logic programs under the credal semantics.* arXiv:2105.10908 → `papers/pasocs-2021-credal-semantics-approximate-solver.pdf`
- Geh et al. *dPASP.* arXiv:2308.02944 → `papers/dpasp-2023-differentiable-probabilistic-asp.pdf`
- Antonucci & Facchini. *A credal extension of independent choice logic.* arXiv:1806.08298 → `papers/credal-extension-independent-choice-logic-2018.pdf`
- Qian et al. *Logical credal networks.* arXiv:2109.12240 → `papers/logical-credal-networks-2021.pdf`
- Antonucci, Facchini, Mattei. *Credal sentential decision diagrams.* ISIPTA 2019 → `papers/antonucci-facchini-mattei-2019-credal-sdd-isipta.pdf`
- Mattei, Antonucci, Mauá, Facchini, Villanueva Llerena. *Tractable inference in credal sentential decision diagrams.* IJAR 125, 2020; arXiv:2008.08524 → `papers/mattei-antonucci-maua-facchini-2020-credal-sdd-tractable-inference.pdf`
- Liell-Cock & Staton. *Imprecise probabilistic programming, precisely: credal sets via graded monads, BDDs, and semiring-parametric inference.* ICFP 2026; arXiv:2607.20801 → `papers/imprecise-probabilistic-programming-precisely-2026.pdf`
- Liell-Cock & Staton. *Compositional imprecise probability: a solution from graded monads and Markov categories.* POPL 2025. doi:10.1145/3704890 *(not fetched)*

## Dependence, bounds, arithmetic (lineage 06)

- Gilio & Sanfilippo. *Compound conditionals, Fréchet-Hoeffding bounds, and Frank t-norms.* IJAR 2021; arXiv:2010.14382 → `papers/gilio-sanfilippo-2021-frechet-hoeffding-frank-tnorms.pdf`
- Williamson. *Probabilistic Arithmetic.* PhD thesis, Univ. of Queensland, 1987 → `papers/williamson-1987-probabilistic-arithmetic-thesis.pdf`
- Williamson & Downs. *Probabilistic arithmetic I: numerical methods for calculating convolutions and dependency bounds.* IJAR 4:89–158, 1990. https://www.sciencedirect.com/science/article/pii/0888613X9090022T *(not fetched)*
- Frank, Nelsen, Schweizer. *Best-possible bounds for the distribution of a sum — a problem of Kolmogorov.* PTRF 74, 1987. *(not fetched)*
- Berleant & Goodman-Strauss. *Bounding the results of arithmetic operations on random variables of unknown dependency using intervals.* Reliable Computing 1998. *(not fetched)*
- Lux & Papapantoleon. *Improved Fréchet–Hoeffding bounds on d-copulas.* arXiv:1602.08894
- *Probability bound analysis for dependence uncertainty in risk and decision models.* arXiv:2606.19086
- Bornholt, Mytkowicz, McKinley. *Uncertain<T>: a first-order type for uncertain data.* ASPLOS 2014 → `papers/bornholt-2014-uncertain-t-asplos.pdf`; code https://github.com/klipto/Uncertainty
- IEEE Std 1788-2015, *Interval Arithmetic.* *(standard; not fetched)*

## Upstream calibration (lineage 07)

- Vovk & Petej. *Venn–Abers predictors.* UAI 2014; arXiv:1211.0025 → `papers/vovk-petej-2014-venn-abers.pdf`
- Vovk, Petej, Fedorova. *Large-scale probabilistic predictors with and without guarantees of validity.* NeurIPS 2015; arXiv:1511.00213
- Petej & Vovk. *Inductive Venn–Abers and related regressors.* arXiv:2605.06646
- van der Laan & Alaa. *Generalized Venn and Venn-Abers calibration.* ICML 2025; arXiv:2502.05676
- Angelopoulos & Bates. *A gentle introduction to conformal prediction and distribution-free uncertainty quantification.* FnT ML 2023; arXiv:2107.07511 → `papers/angelopoulos-bates-2021-conformal-prediction-gentle-intro.pdf`
- Caprio. *Conformal prediction regions are imprecise highest density regions.* arXiv:2502.06331 → `papers/caprio-2025-conformal-regions-imprecise-hdr.pdf`
- Wang, Shariatmadar, Manchingal, Cuzzolin, Moens, Hallez. *CreINNs: credal-set interval neural networks.* arXiv:2401.05043 → `papers/creinns-2024-credal-set-interval-neural-networks.pdf`
- Wang et al. *Credal wrapper of model averaging.* ICLR 2025; arXiv:2405.15047
- Garces Arias, Rodemann et al. *The geometry of creative variability: how credal sets expose calibration gaps in language models.* arXiv:2509.23088 → `papers/credal-sets-llm-calibration-gaps-2025.pdf`
- *Credal concept bottleneck models for epistemic-aleatoric uncertainty decomposition.* arXiv:2604.24170
- Fröhlich & Williamson. *Scoring rules and calibration for imprecise probabilities.* arXiv:2410.23001 → `papers/frohlich-williamson-2024-scoring-rules-calibration-imprecise.pdf`
- Jürgens, Mortier, Hüllermeier, Bengs, Waegeman. *A calibration test for evaluating set-based epistemic uncertainty representations.* arXiv:2502.16299 → `papers/jurgens-2025-calibration-test-set-based-uncertainty.pdf`
- Singh, Chau, Muandet. *Truthful elicitation of imprecise forecasts.* arXiv:2503.16395
- *Integral imprecise probability metrics.* arXiv:2505.16156
- TypeSafe documentation (pasted): Primitives, Choice, Score, Noul, Advanced: structure. https://docs.typesafe.ai/
