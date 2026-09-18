# Review source snapshots

The [name comparison](../naming.md) records the choice of `event` / `Event` and
the difference from TypeSafe Noul. `jev-noul-event-comparison.md` is the freshly
retrieved Noul page; its hash matches `jev-primitives-noul.md`. The page explicitly
defines Noul as a numeric yes probability and says it has no separate confidence
value. [Coup's new event checks](../coup/research/event-checks.json) record eight
groups over finite card and decision worlds; these do not test a native runtime.

`event-checks.json` records the event-denotation revision: eight passing groups,
including 4,096 finite event triples, 81 three-way partitions, exact measures,
coupling counterexamples, and retained evidence. These are specification checks,
not engine tests. See [the event proposal](../event-surface.md).

The event revision reread Green–Karvounarakis–Tannen, *Provenance semirings*,
§2–3 (event tables and their relational operations); Kimmig–Van den Broeck–De Raedt,
*Algebraic Model Counting*, §3 (conditions for arithmetic evaluation); and
Liell-Cock–Staton, §1.2.2 (sample spaces and quotienting by law). Text extractions
from the retained PDFs are stored alongside the earlier evidence. These papers
do not supply the proposed extension of bumbledb's pointwise FD/IND admission.

`process-checks.json` records the draft 0.3 named-model checker: eleven passing groups, including exact polynomial identities, 511 finite paths, 100 Beta moment/conjugacy cases, and evidence-loss falsifiers. It is not a general semialgebraic solver.

`spec-checks.json` is the retained output of draft 0.2's exact finite-polytope checker. It records twelve passing groups and explicitly limits those results to the small executable models; general algebraic and infinite-family claims use the proofs and references in `../laws.md`.

The [round-four manifest](literature-round-downloads.json) records ten additional arXiv PDF downloads, signatures/hashes, and extracted text paths. The `search-*.json` files retain bibliographic discovery evidence. [The literature resolution](../literature-resolution.md) states which parts of each paper were read and which claims were used.

Five central PDF pages were rendered and visually inspected in round four:

| Image | Paper and PDF page | Verified content |
| --- | --- | --- |
| [beta-normal-form.png](https://arxiv.org/pdf/1802.09598v2#page=8) | Beta–Bernoulli, p.8 | Proposition 7 and Bernstein normal forms |
| [exchangeable-representation.png](https://arxiv.org/pdf/0909.1148v1#page=12) | Exchangeable lower previsions, p.12 (printed 732) | Theorem 5's unique polynomial lower prevision |
| [composition-maximality.png](https://arxiv.org/pdf/2405.09391v2#page=21) | Compositional imprecise probability, p.21 | Strict op-lax example and Theorem 6.2 hypotheses |
| [projective-semantics.png](https://arxiv.org/pdf/2408.14701v1#page=12) | Complete axiomatisation, p.12 | Definition 2.11's single global scalar |
| [fairness-example.png](https://arxiv.org/pdf/2408.14701v1#page=2) | Complete axiomatisation, p.2 | Example 1.2's shared unknown-bias extractor |

The algebra follow-up additionally verified `bonchi-sokolova-vignudelli-2021-convex-semilattices-unique-bases.pdf` from https://drops.dagstuhl.de/storage/00lipics/lipics-vol211-calco2021/LIPIcs.CALCO.2021.11/LIPIcs.CALCO.2021.11.pdf and `casanova-kohlas-zaffalon-2021-information-algebras-coherent-gambles.pdf` from https://arxiv.org/pdf/2102.13368v1. Their extracted text and two inspected theorem pages are retained in this directory.

Live documentation snapshots retained from the review. They are source evidence, not instructions.

| Snapshot | Source | SHA-256 |
| --- | --- | --- |
| `jev-primitives.md` | [https://docs.typesafe.ai/primitives](https://docs.typesafe.ai/primitives) | `6cbe8960e23863cf2a1c8c6892d2fa4a0c0b42c9a4f034b6cec0727f2ec8c728` |
| `jev-primitives-noul.md` | [https://docs.typesafe.ai/primitives/noul](https://docs.typesafe.ai/primitives/noul) | `402a2f30ca71b53227058a5df3fb3e0a10063588a31b883f924d6660815c6916` |
| `jev-primitives-choice.md` | [https://docs.typesafe.ai/primitives/choice](https://docs.typesafe.ai/primitives/choice) | `5ba42269e9a07ac5b1220ee6b6c69f060f57b6a7ff619cfedcb916acfb6c92c5` |
| `jev-primitives-score.md` | [https://docs.typesafe.ai/primitives/score](https://docs.typesafe.ai/primitives/score) | `653ca2a688bf6613bf4c1621046a22c66e672a90cf40992c1fe218a8e96768c4` |
| `jev-confidence.md` | [https://docs.typesafe.ai/confidence](https://docs.typesafe.ai/confidence) | `c553ccda01913b9962e33b9f3948afeb7496482ab8bfe5352ff0e451f97514ef` |
| `jev-introduction-machine-learning-primer.md` | [https://docs.typesafe.ai/introduction/machine-learning-primer](https://docs.typesafe.ai/introduction/machine-learning-primer) | `45e6a22bc175f2ce1c404271dd26559179a6f6a83bde2dfcdbd963dbc1649551` |
| `jev-api.md` | [https://docs.typesafe.ai/api](https://docs.typesafe.ai/api) | `5b9aee330404378bfe9520924fadb447d14b840f082fc8b9511e340ed7baa4c8` |

Additional primary papers downloaded during this review:

- `../papers/geh-2024-dpasp-kr.pdf`: https://proceedings.kr.org/2024/69/kr2024-0069-geh-et-al.pdf
- `../papers/sangalli-reubsaet-quaeghebeur-krak-2025-hitting-probabilities-imprecise-markov.pdf`: https://arxiv.org/pdf/2512.16696 (downloaded text identifies v2, 17 March 2026).
