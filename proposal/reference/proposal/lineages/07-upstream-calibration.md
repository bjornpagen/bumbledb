# 07 — Upstream: what models emit, and how it becomes an interval honestly

The engine never calls a model. This lineage is about the boundary: what a
System One answer *is* statistically, and which transformations into a
first-class uncertain value carry a guarantee rather than a vibe.

## What TypeSafe emits (from the pasted docs)

- **Noul**: one number in `[0, 1]`, "the probability that the answer is yes."
  No separate confidence. `0.5` means yes and no are equiprobable, *not* medium.
- **Choice**: a full distribution `probabilities` over the supplied options
  (sums to 1), the argmax `choice`, and `confidence` = "a number from 0 to 1
  computed from how `probabilities` is spread." Options are yours; the model
  never returns a value outside them.
- **Score**: a distribution over ordered levels, `score` = probability-weighted
  mean of level numbers (0 x p0 + 1 x p1 + 2 x p2), `legend`, `confidence`.
  Explicitly: "Different distributions can produce the same score."
- Every question is evaluated independently; one answer is not context for
  another. Independence of *evaluation*, not of the *propositions*.

Nothing above is an interval. Every input is a precise distribution plus a
dispersion summary. Turning `confidence` into interval width is a design choice
with no statistical warrant on its own. The three lineages below supply
warrants.

## Venn–Abers predictors (Vovk & Petej, arXiv:1211.0025, UAI 2014)

The one method that outputs a probability *interval* for a binary label with a
distribution-free validity guarantee.

- A **multiprobabilistic predictor** "maps each sequence (z1,…,zl) to a subset
  of [0,1]." A **Venn taxonomy** assigns an equivalence relation on indices.
- Construction: "Try the two different labels, 0 and 1, for the test object x."
  Fit isotonic regression (PAVA) on the scorer's outputs with `(s(x), 0)`
  appended → `p0`; again with `(s(x), 1)` appended → `p1`. "The probability
  interval output by a Venn predictor is defined to be the convex hull
  conv(p0,p1)."
- Validity (Theorem 1): under IID/exchangeability, "There exists a selector S
  such that P_S is perfectly calibrated for Y" — the selector is the true label.
  Gloss: "at least one of the two probabilities output by the Venn predictor is
  perfectly calibrated." Corollary 1: `P(Y=1)` lies between the expectations of
  the lower and upper endpoints.
- Merge to a point when forced, minimax log loss: `p = p1 / (1 − p0 + p1)`.
  Interpret `(1−p0, p1)` as an unnormalized distribution on {0,1} and
  normalize. Lemma 2: never yields 0 or 1, so log loss is never infinite.
- **Simplified Venn–Abers**: train the scorer once, only the calibrator is
  refit twice. Not a Venn predictor (can violate validity), empirically as good.
- Width shrinks with data: isotonic regression is stable, so "the Venn-Abers
  prediction set converges to a point prediction as the sample size grows"
  (van der Laan & Alaa, arXiv:2502.05676, ICML 2025, generalizing to arbitrary
  losses).
- Limitation: binary only, with a 2026 extension to bounded/unbounded
  regression (Petej & Vovk, arXiv:2605.06646).

**Reading for the engine:** a Noul plus a calibration set yields `[p0, p1]`
with a real guarantee. This is the canonical honest source of a probability
interval for a Boolean proposition.

## Conformal prediction (Angelopoulos & Bates, arXiv:2107.07511)

- Produces **sets**, not intervals of probability: "sets that are guaranteed
  to contain the ground truth with a user-specified probability, such as 90%."
  Distribution-free, finite-sample, model-agnostic. Softmax scores are
  "heuristic"; conformal converts them into rigorous coverage.
- For a Choice over a closed roster, a conformal prediction set is a **subset of
  the roster with coverage 1−α**. That is a different first-class object from
  an interval vector: it is a set-valued answer.
- Bridge to imprecise probability: Caprio 2025 (arXiv:2502.06331) proves "a
  classical conformal prediction region coincides with the imprecise highest
  density regions (IHDR, the IP version of a credible interval) of the credal
  set." A 2026 follow-up (arXiv:2603.06826) decomposes interval width into an
  aleatoric core, an epistemic inflation term "induced by credalization," and
  conformal calibration slack.

**Reading for the engine:** conformal sets are the set-semantics-native way to
store a Choice: insert every option in the set as a fact. Coverage is a
property of the procedure, not of any one row.

## Evidential deep learning (Sensoy, Kandemir, Kaplan, arXiv:1806.01768)

- Replaces softmax with a **Dirichlet output**: "we treat predictions of a
  neural net as subjective opinions and learn the function that collects the
  evidence." Concentration `α_c = 1 + e_c`, evidence `e_c ≥ 0` from a
  non-negative activation. Belief `b_c = e_c / S`, uncertainty `u = C / S`,
  `Σ b_c + u = 1`.
- This is Jøsang's multinomial opinion (see lineage 02) emitted directly by a
  network. Prior Networks (Malinin & Gales 2018) are the same parameterization
  with a different loss, and formalize epistemic vs aleatoric decomposition.
- Surveys: "Prior and Posterior Networks" (arXiv:2110.03051); plug-in losses
  (arXiv:2605.22746, 2026); density-aware EDL (arXiv:2409.08754).

**Reading for the engine:** if the upstream model is evidential, the honest
first-class value is the **evidence vector**, and the interval is derived
(`[b_c, b_c + u]`). A TypeSafe-style distribution plus confidence is a lossy
projection of this.

## Credal / imprecise outputs from modern models (2024–2026)

- **CreINNs** (arXiv:2401.05043): credal-set interval neural networks; apply
  de Campos's reachability correction to the emitted interval vector.
- **Credal wrapper of model averaging** (ICLR 2025, arXiv:2405.15047) and
  **credal deep ensembles** (NeurIPS 2024): ensembles → credal sets.
- **Credal sets expose calibration gaps in language models** (arXiv:2509.23088).
- **Random-set LLMs** (arXiv:2504.18085): Cuzzolin's random-set framework on
  LLM token distributions.
- **Credal concept bottleneck models** (arXiv:2604.24170, 2026): explicit
  epistemic/aleatoric decomposition via credal sets; primer: "Imprecise
  probability (Walley, 1991) instead assigns intervals [P̲(A), P̄(A)]."
- **Scoring and calibration for imprecise forecasts**: Fröhlich & Williamson
  (arXiv:2410.23001); Jürgens et al. calibration test for set-based epistemic
  uncertainty (arXiv:2502.16299); truthful elicitation of imprecise forecasts
  (arXiv:2503.16395). These define what it means for an emitted *interval* to
  be well-calibrated, which is the evaluation the boundary needs.

## Walley's Imprecise Dirichlet Model (JRSS-B 1996; Bernard IJAR 2005)

The classical statistical route from counts to intervals with no prior.

- Posterior over a multinomial chance θ described by a **set** of Dirichlets;
  inferences summarized as lower/upper probabilities.
- Predictive interval for category j after N observations with n_j hits:
  `[n_j / (N + s), (n_j + s) / (N + s)]`, hyperparameter `s` sets caution
  ("the larger the value of s, the more cautious the inference"; 1 ≤ s ≤ 2 is
  the argued range). Width is `s / (N + s)`, shrinking in N.
- **This is the same interval as Jøsang's `[b, b+u]`** with `W = s`. IDM and
  subjective logic agree on the evidence-to-interval map. See lineage 02.
- Hutter's robust estimators (arXiv:0901.4137); R package `imprecise101`.

## What this lineage decides

1. The **honest primitive at the boundary is evidence**, not probability: counts
   `(r, s)` or a Dirichlet evidence vector. Intervals are derived from evidence
   by a fixed rule (IDM/opinion), with width a decreasing function of total
   evidence.
2. When the source is a black-box scorer with a calibration set, **Venn–Abers**
   is the warranted interval for a Boolean question.
3. When the source is a Choice with coverage requirements, **conformal sets**
   are the warranted object, and they are set-valued, which the engine already
   models natively.
4. `confidence` as emitted by TypeSafe has no guarantee and should not be
   silently converted to width. It can be *stored* as an ordinary `f64` fact and
   used by application policy.
