> Historical design research. The [active proposal](../../proposal.md) supersedes recommendations here.

# Review: a probability algebra worth adding to bumbledb

**Follow-up:** The user's clarified criterion is algebraic unity. [The algebraic analogy to Allen](/Users/bjorn/Documents/bumbledb/proposal/algebra-analogy.md) revises the practical-first representation recommendation below and adds two primary sources on information algebras and the complete equational theory of convex semilattices. The mathematical claim checks in this review still apply.

Reviewed against the source tree, the seven research lineages, their primary papers, the supplied review brief, and the live TypeSafe documentation. This is a design review, not an implementation. Mathematical examples use exact rationals; the proposed stored representation is discussed separately.

**Recommendation: make a coherent distribution over a named, closed roster the central abstraction.** Add a checked `Prob` endpoint pair as its scalar building block. Represent the distribution relationally, with a declared law that binds assessment identity, outcome identity, and probability bounds. Its mathematical meaning is a **box intersected with the probability simplex**. Preserve that shared distribution while combining events and comparing actions; calculate interval answers at the boundary of the calculation.

The valuable operation is **robust decision comparison**: prove that one action has greater expected value under every distribution still allowed by the evidence. Allen comparisons cannot do this, and comparing the resulting expectation intervals cannot always do it either. Exact event mass, coherent refinement, deterministic regrouping of outcomes, and a later path to imprecise Markov models make this a substantive new type.

The synthesis contains much of the right mathematics, but its proposed unification overstates closure, conflates several meanings of independence, misstates Fitting's structure, and assumes execution machinery the current engine does not have. A scalar interval with selectable t-norms is insufficient as the central answer.

The unifying theory is **finite credal sets and coherent lower previsions**: the declared constraints define the admissible distributions, and query bounds are extrema over that shared set. The box–simplex is the tractable first representation. Fitting supplies useful information orders; Boolean event algebra preserves identity; semirings describe particular execution abstractions. Those supporting structures should not override the probability model they are meant to calculate.

## 1. Verdict table for claims a–h

| Claim | Verdict | Reason |
| --- | --- | --- |
| a. Three lower-bound dioids | **CONFIRMED** over exact arithmetic | `max` with Łukasiewicz, product, or minimum has the stated algebraic properties; rounded product does not retain associativity. |
| b. Proposed upper operations are not semirings | **CONFIRMED** | Both fail distributivity; a scalar semiring with conjunction unit 1 cannot supply a universally sound upper disjunction for arbitrary events. |
| c. Lower propagation through joins and recursion | **NEEDS-QUALIFICATION** | `max` is universally sound, but conjunction soundness requires the correct dependence model; repeated leaves are harmless for soundness of Fréchet bounds, not for tightness. |
| d. Read-once interval evaluation | **NEEDS-QUALIFICATION** | Exact with disjoint independent leaf sets and separately varying marginal parameters; stratification alone proves neither property. |
| e. Allen's thirteen relations, including point intervals | **REFUTED** | Point intervals make the ordinary basic predicates overlap; a prioritized classifier does not establish the old relation algebra. |
| f. Roster coherence and Choquet expectation | **CONFIRMED** for the stated model | Exact for a nonempty box–simplex credal set; arbitrary credal sets, signed naive sums, and the expectation result type require qualifications. |
| g. IDM/opinion bijection | **NEEDS-QUALIFICATION** | The interval formulas agree for a fixed positive prior strength, but finite evidence excludes point intervals and does not recover an opinion's base rate. |
| h. Fitting/THREE embeds existing engine negation | **REFUTED** as a combined claim | The three-value restriction is correct; consistent intervals are not a full bilattice, and Fitting's unknown is not the engine's closed-world absence. |

### a. Exact lower algebras and the convergence claim

Let `T` be any of `max(0,a+b−1)`, `ab`, or `min(a,b)`. Each is associative, commutative, monotone, has unit 1 and annihilator 0. Since the carrier is a chain,

```text
T(a, max(b,c)) = max(T(a,b), T(a,c)).
```

Together with idempotent addition `max`, these are commutative semirings. Since `T(a,b) ≤ a`, they satisfy absorption `max(a,T(a,b)) = a`, hence 0-stability. The natural order is ordinary `≤`; the exact-real carrier is a complete distributive chain. This matches the complete distributive dioid framework used in `abo-khamis-ngo-pichler-suciu-wang-2022-convergence-datalog-semirings.pdf`, Theorem 1.2 and §6.

The `N` bound concerns the number of ground IDB variables in the finite grounded system, under the theorem's semantics. It is not an unconditional bound of “the number of stored input facts,” a latency bound, or a proof that bumbledb's current set-presence recursion already implements annotation improvements. Seminaive evaluation also needs the specified algebraic and evaluation assumptions.

Do not transfer this exact-real proof directly to fixed-width product arithmetic. Section 4 gives an explicit failure at the proposed `2^63` scale. Łukasiewicz and minimum do remain exact on that grid.

### b. Upper disjunction, and the wrong field-wide conclusion

For bounded-sum disjunction and minimum conjunction, take `a=b=c=1/2`:

```text
min(a, min(1,b+c))                         = 1/2
min(1, min(a,b) + min(a,c))                = 1
```

For probabilistic-sum disjunction and product conjunction:

```text
a(b+c−bc)                                = 3/8
ab + ac − (ab)(ac)                        = 7/16
```

There is a stronger obstruction. Suppose the scalar carrier is `[0,1]`, 1 is the conjunction identity, and `⊕` is always a sound upper disjunction. Soundness implies `1⊕1=1`. Distributivity would then force

```text
a = a⊗(1⊕1) = (a⊗1)⊕(a⊗1) = a⊕a.
```

But two independent events of probability `a`, for `0<a<1`, have union probability `2a−a²>a`. Under unknown dependence, two disjoint half-probability events are an even simpler contradiction. A single scalar value cannot encode enough event identity to make these requirements compatible. The comonotone/nested-event case escapes because equal-probability events there coincide up to null sets.

**This does not mean upper bounds require lineage, or that lower inference is inherently easy while upper inference is inherently hard.** Syntax-directed Fréchet interval propagation gives both sound bounds cheaply. It lacks arbitrary semiring-based rewrite invariance and usually loses tightness. Exact lower and upper event inference are dual through `U(A)=1−L(Aᶜ)`; exact expectations through `UpperE(f)=−LowerE(−f)`. Some restricted query representations have a real one-sided complexity difference, but the synthesis extrapolates that to the whole field. Its cheap lower result is specifically a strongest-proof abstraction using `max`.

Nor are all interval semirings impossible. Nonnegative intervals with componentwise, **uncapped** addition and multiplication form a semiring over exact arithmetic. Their carrier is not closed unit probabilities. The `IntervalS` code in the ICFP paper uses precisely this distinction; a generic statement that interval arithmetic is never a semiring is too broad.

### c. Sound propagation, repetition, and the coherence correction

For marginal intervals `I=[l₁,u₁]`, `J=[l₂,u₂]`, without additional dependence information:

```text
not I       = [1−u₁, 1−l₁]
and(I,J)    = [max(0,l₁+l₂−1), min(u₁,u₂)]
or(I,J)     = [max(l₁,l₂), min(1,u₁+u₂)]
```

These are the tight bounds given only two unconstrained marginal intervals. They remain sound when composed over formulas, because each step encloses the probability of the corresponding event in every original model. Repeated occurrences of an event do not invalidate that argument. For example, the lower bound for `A∧A` can become `max(0,2l−1)` even though the exact answer is `P(A)`. That is lost precision, not an unsound bound.

The three lower conjunction choices are not interchangeable:

* Łukasiewicz works under arbitrary dependence.
* Product works for genuinely independent subevents. For positive formulas over mutually independent base Bernoulli variables, Harris/FKG positive association also makes product a sound lower bound despite shared leaves. This argument does not extend to negation or arbitrary correlated inputs.
* Minimum requires nested/comonotone events for the probability interpretation. Ordinary positive correlation is insufficient: two half-probability events with intersection `.3` are strictly positively correlated, but `.3` is still below `min(.5,.5)`.

For `A∧¬A`, product of marginal probabilities produces `p(1−p)`, although the answer is zero. A declaration about how model requests are evaluated cannot justify treating those events as independent.

**The synthesis misuses Gilio and Sanfilippo's counterexample.** In `gilio-sanfilippo-2021-frechet-hoeffding-frank-tnorms.pdf`, the marginals `.5,.6,.7` have pairwise Fréchet minima `.1,.2,.3` and triple minimum 0. Assigning those minima simultaneously as **exact** intersection probabilities implies union probability `1.2`, an impossibility. Keeping them as lower inequalities is entirely consistent: the independent joint distribution satisfies all of them. Sound consequences of a common nonempty model remain jointly feasible. Different lower extrema need not be attained by the same distribution; that is normal for coherent lower probabilities. The claim that iterating sound Fréchet bounds itself makes the stored inequalities incoherent should be removed.

### d. Read-once is sufficient, but the exactness story is broader

A Boolean formula is read-once when each base variable appears once. If its leaves refer to distinct independent random variables, each node combines disjoint independent supports. Under a rectangular set of marginal probabilities, interval endpoint evaluation is exact. For a negative literal, its interval is the complemented interval, with endpoints reversed. This does not license an independence claim merely because an anti-join is stratified.

There is an important underclaim: for **any positive Boolean lineage**, including repeated variables, probability under independent Bernoulli inputs is monotone in each base probability. Therefore

```text
LowerP(F) = WMC(F, all lower endpoints)
UpperP(F) = WMC(F, all upper endpoints).
```

Finding the extremizing parameter assignment is easy in this case. Computing either weighted model count can still be #P-hard. Unate formulas admit a similar endpoint argument according to each variable's polarity. Mixed-polarity formulas need more care. Read-once formulas make WMC itself cheap; they are not the sole setting in which endpoint assignments are exact.

Safe-query theorems depend on the query fragment, including self-joins, unions, and negation. The simple hierarchical, self-join-free conjunctive case is not a theorem for the entire current IR. Jha and Suciu's original paper remains unavailable locally; its broader characterization should be checked in the primary text before implementing a safety checker. The elementary read-once proof above does not depend on that unavailable citation.

In `imprecise-probabilistic-programming-precisely-2026.pdf`, §5.2, printed p. 300:15, Liell-Cock and Staton write: “the knowledge that w⊥ + w⊤ = 1 is lost. This is the dependency problem of interval arithmetic”. The adjacent code really does add/multiply interval endpoints componentwise. The page also explains how separate BDD occurrences can choose the same named uncertain parameter differently. The lesson is to retain complementary mass and choice identity, not simply to select a different t-norm. The paper's `Double` example is not a certification of outward-rounded binary64 arithmetic.

`mattei-antonucci-maua-facchini-2020-credal-sdd-tractable-inference.pdf`, §5, Theorem 3, gives exact marginal inference for the CSDD's separately specified local credal sets. It does **not** make arbitrary global shared-parameter models tractable merely by drawing them as a circuit. A compilation must preserve the model and its parameter sharing; compilation size and the change from global to independent local choices are unresolved costs. Conditional inference has additional restrictions in that paper.

### e. Point intervals break the claimed Allen classification

Let `A=[.5,.5]`, `B=[.5,.8]`. The ordinary endpoint predicates say both `A MEETS B` (`A.end=B.start`) and `A STARTS B` (equal starts, earlier end). Equal point intervals similarly satisfy equality and raw meeting predicates. Choosing one case first produces one implementation result, but no longer proves that the ordinary thirteen relations are pairwise disjoint or retain Allen's composition table.

The current classifier requires strict nondegeneracy: [allen.rs](/Users/bjorn/Documents/bumbledb/crates/bumbledb/src/allen.rs:28). Its half-open temporal behavior also differs operationally: closed probability intervals that meet share an admissible value. This is compatible with non-strict endpoint separation, not with a proof of strict preference.

For `Prob`, expose the comparisons users actually need:

```text
refines(I,J)          iff J.lo ≤ I.lo and I.hi ≤ J.hi
compatible(I,J)       iff max(I.lo,J.lo) ≤ min(I.hi,J.hi)
strictlyBelow(I,J)    iff I.hi < J.lo
componentwiseLE(I,J) iff I.lo ≤ J.lo and I.hi ≤ J.hi
```

Refinement includes equal intervals and intervals sharing one endpoint; Allen `DURING` alone is insufficient. Compatibility is not agreement, and neither numerical predicate establishes that two intervals describe the same proposition. An extended point/interval relation algebra is possible, but it would need its own partition and composition proof. It is unnecessary for the recommended v1.

### f. Exact roster formulas and their result type

For a nonempty finite outcome roster `X`, define

```text
C(l,u) = {p : Σᵢpᵢ=1 and lᵢ≤pᵢ≤uᵢ for every i}.
```

The set is nonempty iff `Σl≤1≤Σu`. Tightening each coordinate to

```text
lᵢ* = max(lᵢ, 1−Σⱼ≠ᵢuⱼ)
uᵢ* = min(uᵢ, 1−Σⱼ≠ᵢlⱼ)
```

preserves the feasible set and makes every coordinate endpoint attainable. The resulting envelope is canonical for this family. It does not repair an empty feasible set and does not establish statistical calibration.

For an event `A⊆X`:

```text
L(A) = max(Σᵢ∈A lᵢ, 1−Σᵢ∉A uᵢ)
U(A) = min(Σᵢ∈A uᵢ, 1−Σᵢ∉A lᵢ).
```

Sort the payoffs so `f₁≤…≤fₙ`, and let `Aⱼ={j,…,n}`. Then

```text
LowerE(f) = f₁ + Σⱼ₌₂ⁿ (fⱼ−fⱼ₋₁)L(Aⱼ)
UpperE(f) = f₁ + Σⱼ₌₂ⁿ (fⱼ−fⱼ₋₁)U(Aⱼ).
```

The tail sums are computable in one pass after sorting. Cost is `O(n log n)`, or `O(n)` when the declared rank order is already available. An equivalent implementation starts at all lower masses and allocates the remaining unit mass, up to each upper limit, to the cheapest outcomes for the lower expectation and the most expensive for the upper. This is also a direct constructive proof of the optimum.

The lower probability is 2-monotone and its Choquet integral is this natural extension. Verification sources: `destercke-dubois-chojnacki-2008-generalized-p-boxes.pdf`, pp. 8–9, and `decooman-troffaes-miranda-2008-n-monotone-exact-functionals.pdf`, Theorem 11. The original de Campos/Huete/Moral 1994 paper was not fetched; the formulas additionally have the direct finite-dimensional proof and independent checks described below.

For nonnegative ranks this is at least as tight as `[Σfᵢlᵢ,Σfᵢuᵢ]`, sometimes equal. That naive formula is not a valid general signed interval sum. For general payoffs, the expectation may be negative or exceed 1. **It must not return `Prob` unconditionally.** In v1, return separately typed lower/upper numeric results, with explicit outward rounding and an exact internal comparison for robust preference.

These formulas are exact for `C(l,u)`, not necessarily for another credal set whose coordinate bounds happen to be `l,u`. Lower expectations are generally superadditive, not additive. Expected values under a single distribution are linear regardless of statistical dependence; claiming all exact sums require independence confuses this with attainability of extrema over an uncertain family.

### g. Evidence, without the notation collision

Use `r` for positive observations, `q` for negative observations, and `W>0` for prior strength:

```text
b = r/(r+q+W)
uncertainty = W/(r+q+W)
I = [r/(r+q+W), (r+W)/(r+q+W)].
```

Walley's IDM predictive interval agrees with this after identifying `n=r`, `N=r+q`, and `s_IDM=W`. For an interval `[l,u]` of positive width and fixed `W`, the nonnegative real evidence inverse is

```text
r = Wl/(u−l),     q = W(1−u)/(u−l).
```

This is not a finite-count bijection onto all closed intervals: width zero requires a dogmatic/infinite-evidence case, and integer observations cover only a subset. The base rate `a`, which selects the subjective-logic point `b+a·uncertainty`, is not recovered from the interval. An IDM predictive range, a Beta credible interval, and a frequentist confidence interval are different objects.

Evidence addition is a useful monoid **when the terms are valid, nonoverlapping observations under the declared sampling model**. Running two models on the same message does not establish two independent observations of its truth. Neither the equations nor an evidential network's output magnitude supplies a calibration theorem. Store observation identity when fusion is meant to count evidence; set semantics alone cannot prevent double counting through different aggregate paths.

### h. Correct Fitting terminology and the engine boundary

In `fitting-kleenes-logic-generalized.pdf`, **§2, p. 4**, Fitting explicitly says the consistent interval structure “does not yield a bilattice in his sense.” The truth order is componentwise and is a complete lattice. Reverse inclusion gives the knowledge order and a complete meet-semilattice:

```text
knowledge meet = hull:         [min(l₁,l₂), max(u₁,u₂)]
knowledge join = intersection: [max(l₁,l₂), min(u₁,u₂)], when nonempty.
```

The synthesis reverses these names. There is no greatest consistent information element: contradictory intervals have no common knowledge upper bound. A larger bilattice can retain inconsistent information, but that is a deliberate expansion of the type, not an existing property of `[l,u]` with `l≤u`.

The `{0,1}` restriction gives false `[0,0]`, unknown `[0,1]`, and true `[1,1]`, as stated. That fact does not embed bumbledb's current closed-world relational execution into Fitting semantics. `abo-khamis-ngo-pichler-suciu-wang-2022-convergence-datalog-semirings.pdf`, §7.3, itself contrasts minimal-model semantics with Fitting semantics: `P(a) :- P(a)` is false in the former and unknown in the latter. Stratification does not erase this positive-recursion difference.

Keep absence, a stored uncertain assessment, and inconsistent evidence distinct. Complementing a stored probability assessment is not the same operation as an anti-join on missing rows.

## 2. The strongest alternative and whether it wins

### The demonstration that should determine the design

Suppose an assessment has two outcomes with `P(first)∈[.2,.8]` and the complementary probability on the second:

| Action | Payoff if first | Payoff if second | Possible expected payoff |
| --- | ---: | ---: | ---: |
| A | 10 | 100 | `[28,82]` |
| B | 0 | 101 | `[20.2,80.8]` |

The expectation intervals overlap substantially. Neither action wins in every outcome: B pays one more in the second outcome. Yet A is better in expectation under **every allowed distribution**, by at least 1.2:

```text
LowerE(A−B) = minₚ [10p − (1−p)] = 11·.2−1 = 1.2 > 0.
```

Allen relations between `[28,82]` and `[20.2,80.8]` cannot recover that conclusion. Separate interval subtraction gives `[-52.8,61.8]`, which cannot prove it either. The information doing the work is the common probability model and the aligned outcome identities.

This suggests a coherent family of useful operations:

* **Event algebra:** union, intersection, and complement of selections from the same roster, with exact final probability bounds. Repetition and complements work automatically: `A∨A=A`, `A∧¬A=∅`.
* **Robust preference:** `LowerE(payoffA−payoffB)>0`. Return the undominated actions when no single action is justified.
* **Worst-case choice:** maximize lower expected payoff, explicitly declaring a maximin policy. This is a preference rule, not the same as robust dominance.
* **Minimax regret:** for each action A, compute `max_B UpperE(payoffB−payoffA)`, then choose the smallest. With a finite action roster this is a series of exact expectation queries.
* **Monotone assurance:** refining the same model can only raise lower expectations and lower upper expectations. A strict robust-dominance result survives any further nonempty refinement.

Pairwise undominated actions need not all be optimal for some single distribution; the stronger E-admissibility question requires extra simultaneous constraints. Do not silently identify those decision criteria.

A second concrete example links TypeSafe-style Choice and Score to these operations:

| Outcome | Probability bounds | Rank | Automatic-action payoff |
| --- | ---: | ---: | ---: |
| Refund | `[.4,.6]` | 0 | 10 |
| Exchange | `[.2,.4]` | 1 | 10 |
| Other | `[.1,.3]` | 2 | −30 |

Normalization proves `P(Refund∨Exchange)∈[.7,.9]`. Expected rank is `[.5,.9]`; automatic payoff is `[-2,6]`. A review action with certain payoff 4 is favored by maximin, but is not universally better. Refine `Other` to `[.1,.1]`: the other lower bounds tighten to `.5` and `.3`, the event probability becomes `.9`, and automatic payoff becomes 6. Automatic action now robustly beats review. This is a useful explanation the engine can derive and reproduce.

### Alternatives worth considering

Here `K` is the explicit number of mutually exclusive outcomes. Complexity in `K` must not be confused with complexity in a succinct Boolean description of up to `2^m` joint worlds.

| Representation | What it stores and does particularly well | Main loss or cost | Where it wins |
| --- | --- | --- | --- |
| Bare probability interval | Two endpoints; complement, refinement, intersection, Fréchet enclosures | Loses event identity and shared probability constraints | Individual assessed propositions; scalar building block |
| Evidence / residual-mass model | Counts plus prior strength, or `p=(1−ε)q+εr` with arbitrary `r∈Δ`; additive valid evidence, very cheap expectation | Commits to a sampling/contamination model; arbitrary confidence is not evidence | Repeated labeled observations; deliberately calibrated contamination bounds |
| **Box–simplex roster** | `K` named outcomes and `2K` bounds with total mass 1; exact event mass, expectation, preference, refinement, regrouping | Cannot preserve arbitrary cross-outcome constraints or all posteriors | **Best first version for Choice/Score and robust finite decisions** |
| **Event-constrained or general linear credal relation** | Bounds on subsets and, when needed, arbitrary linear forms in the same probability vector | LP, richer schema, exact solver/certification, harder canonicalization | Conditional reasoning, overlapping policy constraints, correlated assessments |
| Finite vertices / model ensemble | Full probability vector per model; evaluate linear queries by scanning models | Intersections and independent products may grow rapidly; ensemble membership has no automatic coverage guarantee | Preserve correlations across several explicit models |
| Random sets / belief functions | Mass on outcome subsets; ignorance is assigned to sets rather than individual outcomes | Up to `2^K` focal sets; not all interval-box models are belief functions; fusion assumptions matter | Set-valued observations and source evidence naturally about subsets |
| P-box / bounded cumulative distribution | Lower/upper cumulative mass at ordered thresholds | Requires meaningful order; different expressiveness from singleton bounds | Scores, quantiles, threshold risk, stochastic-order comparisons |
| Possibility / necessity | One possibility contour; max/min qualitative reasoning | Cannot express general additive probability information | Ranking, fuzzy logic, best-path propagation |
| Boolean lineage / compiled circuits | Named events and logical dependence; evaluate later under a probability model | Lineage is not itself a probability model; WMC/compilation/credal inference can be expensive | Uncertain tuple existence and arbitrary logical reuse |

The evidence/residual-mass row deserves more than dismissal. For a multinomial IDM, `pᵢ=(nᵢ+Wrᵢ)/(N+W)` with `r∈Δ`, so

```text
LowerE(f) = [Σnᵢfᵢ + W·minᵢfᵢ]/(N+W)
UpperE(f) = [Σnᵢfᵢ + W·maxᵢfᵢ]/(N+W).
```

This elegant common-slack family is already contained in the box–simplex model. It makes an excellent constructor when its evidence assumptions are warranted. It is too restrictive as the universal stored semantics.

For random sets with focal masses `m(B)`:

```text
Bel(A) = ΣB⊆A m(B)              Pl(A) = ΣB∩A≠∅ m(B)
LowerE(f) = ΣB m(B) minᵢ∈B fᵢ  UpperE(f) = ΣB m(B) maxᵢ∈B fᵢ.
```

These are exact, useful operations, with coherent treatment of set-valued evidence. Belief functions are infinitely monotone; the box-induced lower probabilities need only be 2-monotone. The two compact representations should not be presented as one subsuming the other. Dempster's normalized combination is a particular evidence model, not a universally valid operation for pooling two classifiers; conflict normalization must be a visible choice.

### The strongest challenger: a finite constrained probability relation

The strongest alternative is a **finite credal set represented by constraints on named outcomes**, not a recursive opaque polytope value. Schematically:

```text
Outcome(outcome, rank, ...closed payload...)
Assessment(assessment, state, question, sourceVersion, ...)
Bound(assessment, outcome, probability)
EventMember(event, outcome)
EventBound(assessment, event, probability)
```

For example, one `EventBound` can say “Refund or Exchange has at least .9 total probability.” The semantics is an explicit linear constraint over the same `K` mass variables. Arbitrary linear constraints extend that language further. This fits bumbledb's relational vocabulary better than hiding a solver object inside `Value`.

It wins mathematically on information preservation. Consider the two line segments

```text
C₁ = conv{(.5,.5,0,0), (0,0,.5,.5)}
C₂ = conv{(.5,0,.5,0), (0,.5,0,.5)}.
```

Both have all singleton probability intervals `[0,.5]`. But the first two outcomes have total probability `[0,1]` in `C₁`, and exactly `.5` in `C₂`. No choice of canonical singleton endpoints can encode that distinction. Full vertices or appropriate constraints can.

General rational LP is polynomial in the size and bit length of an **explicit** system. It is wrong to reject a small, finite credal roster merely by citing NP-hard probabilistic satisfiability. The latter often concerns a succinct Boolean world space with exponentially many probability variables. There are, however, real costs: exact feasibility, reproducible optimization, explanation certificates, projection, and semantic equality of polytopes are much larger commitments than the two roster sums. Canonical bytes for a list of constraints do not give canonical bytes for the denoted polytope.

**Verdict:** general constrained credal relations beat the synthesis's scalar-centered answer on semantics and preserve its useful compact case. They do not beat box–simplex rosters as the first implementation for the stated upstream data. If v1 must preserve arbitrary subset assessments or repeated conditioning exactly, promote this alternative before implementation. Otherwise use the compact case and make the precision boundary explicit in the API.

Even general convex credal sets do not create a universal small algebra: independence introduces products and parameter-sharing obligations; taking convex hulls can preserve extrema of linear queries while losing an assertion that every retained distribution factorizes. No reviewed source establishes simultaneous unrestricted logical composition, exactness, fixed-size representation, and cheap execution.

### What is actually closed in the compact model

| Operation | Box–simplex result |
| --- | --- |
| Intersect two assessments of the same model | Exact coordinate intersection followed by tightening, or explicit conflict |
| Restrict/refine singleton bounds | Exact, with nonempty-model check |
| Rename or deterministically merge outcomes | Exact pushforward; add bounds within each disjoint group, clamp upper totals to 1, then tighten |
| Form events within one roster | Exact in the Boolean algebra of outcome selections; evaluate the final selection against the original distribution |
| Compute a linear payoff range | Exact scalar extrema, then numerical enclosure if the result is not representable |
| Convexly pool models | Coordinate hull is generally an outer approximation, not the exact convex hull |
| Condition on an event | Not generally closed; retain richer constraints or label a coordinate enclosure as an approximation |
| Apply a general stochastic transformation | Not generally closed; retain the pushforward set or label the enclosure |
| Combine separate assessments | Requires a coupling model; no numerical default restores the missing joint structure |

Deterministic regrouping is worth emphasizing: within each source group, any mass between the sum of its lowers and the sum of its uppers is attainable. Groups are disjoint, so the global unit-sum constraint completes the characterization. This gives exact category aggregation without a general solver.

Conditioning provides a small counterexample to overclaiming closure. Let three outcomes each have mass `[.1,.2]`, and a fourth `[.4,.7]`. Condition on one of the first three occurring. Every posterior singleton has tight bounds `[.2,.5]`, but the posterior still satisfies `q₁≤2q₂`. Its box enclosure admits `(.5,.2,.3)`, violating that constraint. Consequently the exact upper expectation of payoff `(1,−2,0)` is 0, whereas the box reports .1. A general linear/extended representation can preserve the posterior relation; three new intervals cannot.

### The recommended data structure in this engine

Use two levels with separate responsibilities:

```text
Scalar:       Prob { lo: u64, hi: u64 }, 0 ≤ lo ≤ hi ≤ 2^63
Relation:     Bound(assessment, outcome, probability)
Declaration:  a complete coherent distribution per assessment over Outcome
Query view:   Credal<Outcome>, tied to an assessment and its declared law
```

`Credal<Outcome>` here is a high-level structural view over relations, not a new recursive variant of `ValueType`. Outcome identity comes from the closed roster; ranks and utilities are ordinary payload values. A named event is a selection of outcome members, compiled where useful to the existing four-word closed-roster bitset shape. No array-valued database cell is required.

The law binds a parent/group key, the closed discriminator, the bound field, completeness, uniqueness per outcome, and coherence; a canonical constructor tightens before proposing the change. Existing containment and functionality statements supply part of the relational shape, but they do not already express all probability constraints. Queries that need the distribution consume the declared view, not an arbitrary bag of `(rank,prob)` pairs.

One additional consequence is attractive: the greedy expectation calculation can expose the extremizing probability vector as a deterministic **witness**. A robust decision can be explained as “even the worst admissible distribution for this comparison leaves A ahead by 1.2.” In the example that witness is `(.2,.8)`. Tie-break by explicit outcome identity for reproducible witnesses. That explanation belongs to the mathematical model; it does not claim the upstream model is calibrated.

## 3. Missing literature, citations, and consequences

The existing bibliography is broad. Its main omissions are work on annotated logic, relational numerical abstractions, and structured stochastic processes. The following distinguishes primary verification from bibliographic leads rather than pretending every source was retrieved.

| Source and verification status | What it changes |
| --- | --- |
| Kifer & Subrahmanian, *Theory of generalized annotated logic programming and its applications* (1992), [DOI](https://doi.org/10.1016/0743-1066(92)90007-P). Bibliographic record verified; full paper not fetched. | Lattice-valued annotations are an established alternative to semiring annotations. Check its exact interval lattice, negation, and fixpoint conditions before claiming semirings exhaust recursive uncertainty. A lattice annotation alone does not establish probabilistic coherence. |
| Cousot & Cousot, *Abstract interpretation: a unified lattice model for static analysis of programs by construction or approximation of fixpoints* (1977), [DOI](https://doi.org/10.1145/512950.512973). Foundational lead, not newly fetched. | Supplies the right account of interval collapse: an abstraction of a richer relational model. Soundness, tightness, closure, and rewrite invariance become separate obligations. Repeated variables are the classic reason to retain relations between quantities. |
| Harris (1960), *A lower bound for the critical probability in a certain percolation process*; Fortuin, Kasteleyn & Ginibre (1971), *Correlation inequalities on some partially ordered sets*. Foundational results, not newly fetched. | Justifies the positive-association qualification for products of positive formulas over independent leaves. It does not justify product across arbitrary model answers or through complements. |
| Kozine & Utkin, *Interval-Valued Finite Markov Chains* (2002), [DOI](https://doi.org/10.1023/A:1014745904458); Škulj, *Discrete time Markov chains with interval probabilities* (2009), [DOI](https://doi.org/10.1016/j.ijar.2009.06.007). Records verified; primary texts not fetched. | A row of transition probabilities is exactly a normalized uncertain roster. This is a natural specialized recursive use of lower/upper expectation. |
| de Cooman, Hermans & Quaeghebeur, *Imprecise Markov chains and their limit behavior* (2009), [DOI](https://doi.org/10.1017/S0269964809990039). Record verified; primary retrieval unsuccessful. | Lower transition operators and separately specified rows are central, rather than max-over-paths probability tags. Limit behavior requires its own assumptions. |
| Givan, Leach & Dean, *Bounded-parameter Markov decision processes* (2000), [DOI](https://doi.org/10.1016/S0004-3702(00)00047-3). Record verified; primary text not fetched. | Supports robust planning as a stronger product direction. The uncertainty model and Bellman semantics must be declared, particularly when parameters are shared. |
| Sangalli, Reubsaet, Quaeghebeur & Krak, *Computing Lower and Upper Hitting Probabilities for Imprecise Markov Chains*, [arXiv:2512.16696](https://arxiv.org/abs/2512.16696), v2 March 2026. Primary PDF added and inspected: `sangalli-reubsaet-quaeghebeur-krak-2025-hitting-probabilities-imprecise-markov.pdf`. | Gives a current concrete algorithmic route. Assumes nonempty compact convex transition sets with separately specified rows; analyzes different reachability notions, fixed-point uniqueness, and policy-style iteration. It does not establish a blanket polynomial iteration bound in the compact input size. |
| Geh et al., *dPASP: A Probabilistic Logic Programming Environment For Neurosymbolic Learning and Reasoning* (KR 2024), [DOI](https://doi.org/10.24963/kr.2024/69). Primary PDF added and inspected: `geh-2024-dpasp-kr.pdf`. | The repository has the earlier 2023 version. The full paper handles interval facts, neural predicates, several logic semantics, and decision-style examples; §7 makes the enumeration cost explicit. Evidence against assuming a cheap generic interval-logic engine already exists. |
| *Solving Decision Theory Problems with Probabilistic Answer Set Programming* (2025), [DOI](https://doi.org/10.1017/S1471068424000474). Metadata/abstract verified; full proof not checked. | Decisions, utilities, credal semantics, and layered algebraic model counting are a direct comparator for the proposed robust-decision feature. |
| Liell-Cock & Staton, *Compositional imprecise probability: a solution from graded monads and Markov categories* (POPL 2025), [DOI](https://doi.org/10.1145/3704890); ICFP 2026 follow-up already local and checked. | The 2026 paper's named choices and explicit independence are more useful architectural guidance than treating its approximate interval evaluator as the canonical type. The 2025 primary remains a follow-up verification target. |
| IEEE 1788-2015, *Interval Arithmetic*. Standard listed in the repository but not fetched. | Motivates distinguishing an empty result, an ill-formed input, and a weakened definedness guarantee. Decorations do not supply event provenance, calibration, or a probability conflict policy. Confirm exact names and propagation rules from the standard before claiming compliance. |

For 2024–2026, the repository already includes recent work on semiring convergence, circuit representations of Datalog provenance, credal calibration, and imprecise probabilistic programming. The added sources above strengthen that coverage. This review did not establish an accepted, small, universally closed “credal Datalog” algebra. That is a bounded search result, not a claim that no relevant paper exists.

### What the two 2026 papers actually warrant

The ICFP 2026 paper is an important confirmation that names and dependence survive composition only if the representation retains them. Its interval approximation deliberately drops constraints, including complementary branch mass. It does not refute exact nonnegative interval semirings, nor does it make their unit-probability interpretation automatic. Its numerical optimization section also explicitly discusses nonconvexity and local optimization; it is not a general exact polynomial optimizer.

`domain-theoretic-foundation-imprecise-probability-2026.pdf` studies observational event pairs under a measure as well as imprecise parameter models. Those are not interchangeable with a rectangular family of independent Bernoulli marginals. In particular:

* The §8 lower-factor/upper-Fréchet independence result refers to that paper's event-pair notion. It should not replace the product bounds for an ordinary independent pair of uncertain Bernoulli probabilities.
* The interval Bayes endpoints in §6, p. 10, follow the monotonicity of `xy/[xy+z(1−x)]` over a free parameter box, with well-defined denominators. They do not automatically remain sharp after adding couplings between prior and likelihood parameters. Zero-probability conditioning requires an explicit policy.
* The Cantor-system example in §9, p. 22, really gives an event-probability image `{0,1}`. It concerns a family of invariant measures that is not the finite convex credal set used here. The affine event image of a finite convex credal set is an interval. This example is not a counterexample to that basic fact.
* Appendix B identifies a harder bilinear problem for imprecise transition parameters. It cannot be cited as a ready-made closure or tractability result for arbitrary `rec` queries.

### A genuinely useful later recursion primitive

For an explicitly declared imprecise Markov kernel, let each state `s` have a coherent distribution `Cₛ` over successor states. With a target set `G`, finite-horizon robust reachability can use

```text
h₀(s) = 1_G(s)
hₜ₊₁(s) = 1                         if s∈G
hₜ₊₁(s) = LowerE_Cₛ(hₜ)             otherwise
```

and the upper-expectation dual. Under the separately specified, stepwise rectangular model this is exact dynamic programming, polynomial per explicit horizon in the state/transition representation. It reuses precisely the roster expectation primitive recommended here.

Do not equate this with a model in which one shared unknown parameter must remain fixed across all time steps; finite-horizon parameter reuse can change the optimization. Also do not equate it with random-edge network reliability, where the query asks about existence of a path in a sampled graph.

For infinite-horizon hitting, the new Sangalli et al. paper is more specific: §2.5 explains a setting in which homogeneous and inhomogeneous hitting envelopes coincide under its assumptions; §4.3 relates the calculation to MDP policy iteration; Corollary 5.2 bounds iterations by the number of extreme transition matrices and states that this bound can be tight. That number can be large relative to a compact row-constraint representation. Finite-horizon tractability, asymptotic fixed-point existence, exact infinite-horizon termination, and a certified numerical stopping rule are separate claims. A specialized future operator has a much better foundation here than silently changing existing `rec` semantics.

### Verification performed and what remains open

[review-checks.py](/Users/bjorn/Documents/bumbledb/proposal/review-checks.py) is a standalone exact-rational checker, independent of the engine. It verifies the three algebras on 125 rational triples each; tests 500 generated small boxes, including 306 feasible models, against exhaustive vertices for feasibility, tightening, every event, 2-monotonicity, and signed expectations; and checks the numerical associativity, decision, regrouping, information-loss, and conditioning examples. These checks support the proofs and expose counterexamples; they are not proofs of all real-valued algebra laws or an engine conformance test suite.

Primary-source verification remains necessary before adopting the unavailable annotated-program, original de Campos, broader safe-query, or IEEE standard results as implementation contracts. A general-polytope version additionally needs an exact solver prototype and model-preserving conditional/lineage compilation tests. No unresolved citation is needed to establish the finite box model, its greedy optimizer, or the counterexamples used in the recommendation.

## 4. Decisions 1–5 with recommendations

### Decision 1: fixed-point endpoints, with exact accumulation and an honest numerical contract

**Choose the `u64` pair with `M=2^63` representing 1 for this v1.** This gives deterministic exact complement, comparisons, refinement, roster coherence, tightening, event mass, and the Łukasiewicz/minimum algebras. The constructor accepts only `0≤lo≤hi≤M`, including points. Give it its own canonical tag and big-endian endpoint encoding.

| Calculation | Required implementation contract |
| --- | --- |
| Complement | `[M−hi,M−lo]`, exact |
| Łukasiewicz lower conjunction | `max(0,a+b−M)`, exact using a widened sum, or `a.saturating_sub(M−b)` |
| Bounded upper disjunction | `min(M,a+b)`, with widened addition or an equivalent overflow-free form |
| Independent lower product | `floor(a·b/M)` with exact `u128` product |
| Independent upper product | `ceil(a·b/M)` with exact `u128` product and explicit upward rounding |
| Roster sums | Widen before summing; 256 endpoint values fit comfortably in `u128` |
| Expectation | Choose the extremizing masses, accumulate the full weighted numerator exactly, round once at the requested numeric output |
| Robust comparison | Compare the exact numerator of `LowerE(f−g)` against zero before converting to a display number |

Do not use saturating `u64` addition followed by subtraction for Łukasiewicz: at `a=b=M`, saturating the overflowing sum first would return `M−1` instead of `M`. The widened or subtractive formulas avoid this edge case.

The existing [Rounding enum](/Users/bjorn/Documents/bumbledb/crates/bumbledb/src/scalar.rs:17) has `TowardZero` and two nearest modes, but no upward mode. Nonnegative probability products need floor/ceiling. Signed expectation results need rounding toward negative/positive infinity; truncation toward zero is not a lower enclosure for negative values.

For integer utility/rank payloads and dyadic bounds, the greedy optimizer's mass allocations are on the same grid. The expectation numerator can therefore be accumulated exactly. With `u64` payoffs the total weighted numerator is less than `2^127`; with `i64` payoffs a signed `i128` accumulator suffices. Form signed payoff differences in a widened type. Do not round each term of a Choquet sum or multiply every term through a lossy scalar output before folding it.

**Directed rounding preserves enclosure, not the semiring laws.** At the proposed scale, let

```text
M = 9223372036854775808
a = M/4 = 2305843009213693952
b = 5
c = 7M/8 = 8070450532247928832
mul(x,y) = floor(x·y/M)

mul(mul(a,b),c) = 0
mul(a,mul(b,c)) = 1.
```

Thus fixed-point product tags cannot rely on the associativity theorem in claim a. The failure is one grid unit, but it changes exact canonical results and can cross a threshold. Binary64 product has the same category of problem. Exact evaluation of an entire rational expression followed by one final rounding, or a fixed evaluation/grouping contract with restricted rewrites, are possible answers; neither is the existing scalar semiring unchanged. For recursive product expressions, exact rational denominators can grow, so this is not a trivial bounded accumulator fix.

Use one documented import boundary. If the source is a parsed JSON binary64 value, lower conversion rounds down and upper conversion rounds up relative to that **represented** number. If the intended source is the exact decimal token instead, parse that token exactly before conversion. Do not promise both meanings at once. A point forecast `[p,p]` becomes a point on this grid only when `pM` is integral; otherwise its numeric enclosure has width at most `1/M`. That width is quantization, not epistemic uncertainty.

The tradeoff is a uniform absolute resolution of about `1.0842×10⁻¹⁹`. Positive probabilities below that cannot retain a positive lower endpoint. Binary64 preserves much smaller magnitudes and may be preferable for extreme-tail workloads, but requires careful outward rounding for complement, multiplication, and sums; existing nearest-rounded numeric operations are insufficient. The current Choice/Score use case favors the simpler exact-grid laws. Verify rare-event requirements before fixing a public wire format.

The draft's claim that the whole existing `F64` type is NaN-free is inaccurate. Ordinary f64 values have a canonical NaN representation; interval endpoints impose stronger restrictions. `Prob` should reject NaN and infinities at its own constructor regardless of that existing behavior.

### Decision 2: unknown dependence for unmarked scalar connectives

**Choose Fréchet bounds for unmarked `and`/`or` of scalar assessments without a declared joint model, and spell out actual independence.** Prefer a name such as `andBound` when the operation is only an enclosure. The failure mode of product-by-default is a plausible-looking, unjustifiably narrow or even unsound result.

Inside the **same declared roster**, combine the outcome selections exactly first. Two different singleton outcomes are mutually exclusive; one event and its complement exhaust the roster. Neither generic Fréchet propagation nor independent product is the right substitute for these known relationships.

An explicit independent operation needs a declared probability model or a documented caller assumption, not a deduction from two different row IDs, separate requests, separate models, or low empirical correlation. Minimum/maximum should be named for a comonotone/nested-event model when interpreted probabilistically. A generic Frank parameter is not itself proof that all resulting dependence claims can coexist globally.

### Decision 3: store the interval; keep evidence as ordinary declared data when real

**Choose `Prob` as the scalar. Store identified evidence and its declared `W` in ordinary relations when the application actually has it.** Derive probability intervals through an explicit constructor/query. This accommodates raw model forecasts, externally calibrated bounds, elicited constraints, and count-based IDM inputs without fabricating observations.

Evidence-only storage cannot represent finite exact point assessments, arbitrary credal bounds, or unspecified model scores honestly. Storing both counts and derived intervals everywhere introduces synchronization and policy-version obligations without gaining information. If an application chooses both, it needs an explicit derivation/consistency rule, including rounding and `W` identity.

Separate three different combination operations: intersect compatible constraints about the same model; take a conservative outer envelope when retaining alternative models; accumulate valid nonoverlapping observations under an evidence model. None is a universal “fusion” of arbitrary model opinions.

### Decision 4: no generic annotation lane in v1

**Of the two offered options, choose the nonrecursive scope. Narrow it further to explicit scalar and law-bound roster operations initially.** These are enough to deliver the new capability while retaining the current set-semantic execution contract. Ordinary joins may carry stored `Prob` fields and assemble payoff tables; that is different from treating their rows as probabilistically present.

A subsequently added safe probabilistic-query fragment should reject or clearly downgrade unsupported lineage shapes at preparation, preserve base event identity, and have a proved evaluation contract. Do not ship an implicit “safe” label justified only by the presence of a nonrecursive plan.

If a lower-bound recursive prototype is separately desired, `max` with grid-exact Łukasiewicz is the defensible arbitrary-dependence starting point. It still needs an annotation-improvement evaluator, a bound interpretation, and differential tests. Minimum has the wrong universal probability meaning; rounded product has the associativity problem. None is a free switch at the Free Join sink.

The failure mode of putting recursive tags first is that a sophisticated new execution model ships before the feature's semantics are settled, while the user-facing operation remains a loose bound. The box–simplex expectation and decision primitive offers more immediate value for less architectural risk.

### Decision 5: explicit roster laws, enforced on every admitted final state

**Require a schema statement to declare a coherent distribution.** A `prob` field next to a closed discriminator does not necessarily denote a partition of mutually exclusive outcomes. It could be one probability per independent symptom, a transition likelihood under different conditions, or separate reviewers' assessments. Automatically imposing total mass 1 would silently change those schemas' meaning.

Once the statement exists, enforce its declared obligations deterministically at admission: parent/group identity, full roster, one row per member, `Σlo≤M≤Σhi`, and canonical reachable bounds if that is the declared representation. Recommend that the high-level canonical distribution constructor requires tightened bounds; perform tightening when building the proposed change, not as a hidden repair inside the judge.

Treat the missing-parent/missing-child cases explicitly. Grouping only rows that happen to exist cannot prove that every required parent has a complete distribution. Updates are judged in the proposed final state, so replacing an entire assessment can be atomic. No tolerance, model call, solver randomness, or order-dependent patching belongs in admission.

## 5. The jev boundary: honest rules

The provided brief describes the primitive contract accurately at a high level. The live [primitives](https://docs.typesafe.ai/primitives), [Noul](https://docs.typesafe.ai/primitives/noul), [Choice](https://docs.typesafe.ai/primitives/choice), [Score](https://docs.typesafe.ai/primitives/score), [confidence](https://docs.typesafe.ai/confidence), [machine-learning primer](https://docs.typesafe.ai/introduction/machine-learning-primer), and [API](https://docs.typesafe.ai/api) pages were also inspected. Saved snapshots are retained under `proposal/review-evidence/` for this review.

1. **Store what the service returns, with its assessment context.** Noul returns `noul`, a probability of yes, and no separate confidence. Choice returns a selected option, a full option probability distribution, and confidence. Score returns a full level distribution, its probability-weighted mean of numeric level indices, the legend, and confidence. Keep state identity, question/option definitions, source/model version when available, and response/assessment identity so the numbers retain a referent. Record a service-selected option separately if reproducing that selection matters; do not invent a tie-breaking contract.

2. **Do not say the docs make no calibration claim.** The current primer explicitly describes RLCD, reinforcement learning for calibrated decisions. It explains grouped frequency calibration and says: “These rates describe groups of predictions, not a guarantee about any single answer.” This corrects the repository's more categorical dismissal. The inspected pages do not provide a domain-specific error bound, a per-case probability interval theorem, a calibration dataset specification, or a quantified guarantee applicable to this application's data.

3. **Confidence is a dispersion statistic, not an interval width.** The confidence page says it is computed from the returned distribution and summarizes how peaked it is. The inspected documentation does not specify whether the formula is entropy, margin, or another statistic. Do not reverse-engineer a mathematical contract from a few examples. A calibrated point distribution can legitimately be diffuse; a confidently wrong forecast can be peaked. Store confidence as an ordinary finite `f64` fact, and let applications validate action/escalation thresholds against observed outcomes.

4. **A confidence-to-width rule needs an external, specified inferential target.** Labeled calibration data, assumptions about exchangeability or domain stability, and a proved/validated calibration procedure could make confidence one useful feature of that procedure. No monotone function of confidence alone gives a warranted per-case probability interval. State what is covered: a population calibration property, a parameter confidence set, a Bayesian posterior statement, or a robust modeling assumption. These are not interchangeable.

5. **Import Noul as a point forecast under the model, with numerical enclosure if needed.** Mathematically that is `[p,p]`. On the proposed grid, use the exact point when representable and outward endpoints otherwise. This bounds the supplied forecast value; it does not certify that the true conditional probability equals that forecast. Externally justified epistemic bounds can replace/augment the point under a separately identified method.

6. **Import Choice with explicit normalization and quantization policy.** The documented distribution sums to 1 mathematically, but independently parsed binary64/decimal payloads need not sum to exactly 1 under the engine's arithmetic. Validate all options, nonnegativity, finiteness, and positive total first. A defensible adapter retains the raw response and explicitly normalizes its exact interpreted numbers once, then outward-quantizes each coordinate and tightens the resulting feasible box. This encloses a documented normalized forecast model. A mass-conserving rounded point vector is another possible policy, but it perturbs the forecast rather than enclosing it. Never hide an epsilon in the judge.

7. **Tightening is not calibration or inconsistency repair.** If external widths are supplied, first check `Σlo≤1≤Σhi`. For a feasible box, de Campos-style tightening preserves the model while removing unattainable endpoints. If the box is empty, return a conflict; the correction formula cannot manufacture a coherent assessment. If the source supplies richer constraints or a full ensemble, converting it to singleton intervals must be labeled a loss of information.

8. **Import Score as the distribution plus explicit rank payload.** Compute a point mean or lower/upper expectation under the chosen forecast/credal model. Roster declaration order is not arithmetic semantics in this codebase; numeric rank must be a declared payload column. A score can be greater than 1, so its expectation result is not automatically `Prob`. Different distributions with the same mean must remain distinguishable.

9. **Repeated calls are not labeled evidence.** The inspected API does not document a logprob interface, a sampling/seed contract, a Venn taxonomy, or a `(p₀,p₁)` endpoint. Descriptive question criteria can help define a future calibration procedure, but do not themselves instantiate one. Variation between repeated responses measures variability of that calling procedure, not error against the truth. Even available logprobs would not by themselves provide an epistemic interval.

10. **Use Venn–Abers and conformal results with their actual guarantees.** In `vovk-petej-2014-venn-abers.pdf`, Theorem 1 establishes calibration for an appropriate selector from the multiprobability output under its exchangeability assumptions. It does not assert that the unknown individual conditional risk lies between the two returned values with a chosen coverage level. Full Venn–Abers and valid inductive variants have specified fitting/calibration procedures; feeding a score through arbitrary isotonic code is not enough. A fixed black-box scorer with suitable held-out labeled calibration data can be part of a valid inductive construction; that calibration data is the missing ingredient, not access to neural internals.

11. **A conformal prediction set is not a probability-1 support restriction.** Standard conformal coverage is a marginal guarantee of a procedure over future examples. It does not justify setting excluded labels to zero probability in this individual assessment, or treating included labels as a warranted credal set of conditional probabilities. Store the set and its procedure/coverage metadata as such unless an additional model proves the proposed conversion.

12. **Independent evaluation does not imply independent propositions.** TypeSafe explains that one answer is not hidden context for another and that questions see the same state. That is an evaluation contract. Answers about refund intent and exchange intent can be logically related or correlated. Unmarked scalar combination remains Fréchet; shared-roster events use their known exclusivity; actual cross-question independence needs its own justification.

13. **Batching and speculative fan-out do not determine schema shape.** Group facts by their meaning, state, question definition, and assessment/source version. A stable small product of questions may reasonably be fields on one relation; evolving or repeated questions may reasonably be keyed assessment rows. The transport batch and token budget are ordinary execution/provenance metadata, not a reason to encode every possible question as a field or to infer independence.

The engine can guarantee that calculations are exact or sound **relative to the stored model**. Its deterministic judge cannot establish that the external model contains the truth. Keep that boundary in the data semantics rather than trying to encode a calibration promise into `Prob`'s two endpoint bits.

## 6. Engine-fit risks, ranked by late-discovery rework

### 1. Confusing assessed values with uncertain tuple existence

This is the largest architectural risk. All the rows of an assessment distribution are stored facts: they report bounds for the outcomes. They are not independent probabilistic claims that each row exists. A Choice's outcomes form one mutually exclusive variable, not `K` independent Bernoulli tuple tags. Existing keys and containment constrain the stored assessment table; possible-world constraints would constrain the worlds described by that table. Those are different models.

Introducing annotations into Free Join changes how duplicate derivations, existential projection, aggregation, and already-present recursive tuples are handled. A tuple whose annotation improves has to re-enter propagation even when its set membership is unchanged. The plan, recursive frontier, materialization, delta handling, and naive oracle must all agree on that behavior. Probability-of-existence semantics also needs a source model for keys, exclusivity, and shared events. Treating a field as a tag at the sink cannot supply those requirements.

Positive Boolean lineage is not enough for arbitrary negation. It represents positive event formulas; complements and world-wise negation need a richer lineage/evaluation contract. A current anti-join on stored row presence must not silently become negation across possible worlds. The recommendation avoids this change in v1 while still allowing ordinary joins over probability-valued columns.

### 2. Erasing the shared model and outcome identity before an aggregate

The demonstrated robust-preference feature only works if both actions are compared against the same probability vector with aligned outcomes. Preserve `assessment` and `outcome` through joins and grouping, and subtract payoff functions before taking extrema. Independent intervals for the final scores are an insufficient intermediate representation.

Distinguish **proposition identity** from **assessment identity**. Two evaluations of the same proposition can disagree while referring to the same truth event. Conversely, equal interval bytes can refer to entirely different events. A model/run ID is not an independence certificate. Event selections also need their roster context; the same bit position in two closed rosters does not identify the same outcome.

Set semantics has a concrete aggregate trap: projecting rows to `(rank,prob)` before folding can collapse two different outcomes with identical values. Keep their identities until their contributions have been accounted for. An arbitrary join can also multiply payoff rows or omit outcomes. A `LowerExpectation` operation should consume a validated distribution view with one payoff per outcome, or independently check that shape; a bare aggregate over whatever rows reach the sink is too weak a contract.

### 3. Assuming mathematical associativity survives representation

The product counterexample in section 4 can invalidate planner regrouping, seminaive proofs, and cross-plan byte equality even though each individual rounded operation encloses the truth. Exact accumulation and outward rounding need distinct contracts for probabilities, arbitrary signed payoffs, and imported floats. Tie decisions should be made before lossy rendering when possible.

Choose the public semantics before optimizer integration: an exact-grid scalar operation, an outward enclosure of a full expression, or a restricted approximate propagation mode. Do not interchange them under one opcode. This is also why generic recursive product tags are out of scope.

### 4. Treating `WordPair` layout as complete support for a new value

The flat [ValueType declaration](/Users/bjorn/Documents/bumbledb/crates/bumbledb-theory/src/schema.rs:65) is the right place for a scalar `Prob` variant. A 16-byte layout is plausible and compatible with existing column storage machinery, but the current [decoder](/Users/bjorn/Documents/bumbledb/crates/bumbledb/src/image/decode.rs:144) has a catch-all `(ColumnWidth::WordPair, _) => Decode::Interval` branch. Merely assigning the width would decode the new value as an interval. UUID handling also illustrates that byte width alone is not a semantic contract.

The new tag must be handled explicitly throughout construction, validation, row encoding, ordering, image decoding, literals, parameter binding, schema fingerprints, logging/replay, native bridges, and SDK codecs. TypeScript must preserve the 63-bit scale through exact integer representations, not ordinary `number` endpoint fields. Keep the ordinary exact pair ordering for storage/indexes; the partial semantic refinement order is a separate predicate.

Do not reuse temporal interval constructors: they exclude the required points. Do not make probability keys inherit temporal coverage semantics. Exact pair equality is a valid key component; “a refining probability is the same key” is a different schema theory and should not arise accidentally.

### 5. Hiding new probability laws inside existing containment

The current [statement descriptors](/Users/bjorn/Documents/bumbledb/crates/bumbledb-theory/src/schema.rs:292) define functionality, containment, and capacity. Containment projections must meet the engine's target-key and selection rules; they do not currently mean semantic inclusion of probability models. A field's compatibility with a closed roster also does not by itself prove mass conservation, completeness per assessment, or canonical reachable bounds.

Reuse the grouped judge and discriminator machinery where it applies, but represent the probability obligation explicitly in structural schema data. Include all declared semantics in fingerprints and wire descriptions. The judge should reject an invalid proposed final state with a deterministic witness; it should not normalize input, choose calibration widths, infer missing outcomes, or call a floating LP solver with an epsilon.

The recommended box model makes exact admission cheap. A future general linear-constraint law needs exact rational feasibility or a checked certificate, deterministic error/witness behavior, and a deliberate resource policy. Floating optimization is suitable for exploration, but cannot silently substitute for the existing exact judge contract.

### 6. Unspecified conflict, empty groups, and expectation output semantics

For scalar intervals, intersection is partial. For coherent rosters, even individually valid coordinate intersections can make total mass infeasible. Return a distinct conflict/refusal result with enough information to diagnose it. Do not turn that into a missing row, `[0,1]`, or `[0,0]`. Missing data, ignorance, impossibility of an event, and inconsistency of evidence have different meanings.

IEEE decorations are a useful source of design vocabulary, but an undifferentiated NaI-like value is not a substitute for those distinctions. Keeping conflicts out of the valid `Prob` constructor is compatible with a separate typed operation failure or admission diagnostic.

Specify empty-group behavior as well. A distribution requires at least one outcome and complete membership. Hull over no inputs has no valid nonempty interval result; intersection of no constraints can naturally be vacuous, but that must not make an absent required assessment look complete. General expectation outputs need ordinary numeric bounds, not a half-open interval that excludes points or a `Prob` that excludes signed utilities.

### 7. Reusing temporal comparisons without proving their new algebra

`CmpOp::Allen` on nondegenerate temporal intervals has a sound existing interpretation. Extending it to points under the same mask/composition assumptions would import the counterexample in section 1e into the query engine. Add the few direct probability predicates needed for refinement and separation; keep their meanings independent of index byte order and temporal containment. This is a smaller surface than a second thirteen-relation system with altered definitions.

### 8. An oracle that repeats the implementation instead of checking the model

The differential and naive machinery in `crates/bumbledb-bench` should gain an independently specified reference semantics before any executor extension. Meaningful coverage includes:

* **Scalar representation:** all endpoint extremes, points, complement involution, overflow at `M+M`, noncanonical/invalid decoding, exact bytes and schema/log replay across Rust and TypeScript, binary64 subnormals and boundary quantization.
* **Roster models:** enumerate exact rational vertices for small `K`; compare feasibility, canonical tightening, every event, signed expectation, deterministic regrouping, and the conditioning information-loss example. Check idempotent tightening and preservation of the feasible set.
* **Admission:** missing and duplicate outcomes, parent without children, extra outcomes, inconsistent intersections, deletion/replacement in one proposed change, and independence of admission from insertion order or fork/replay order.
* **Relational calculations:** equal payloads under different outcome IDs, join-induced duplication, incomplete payoff tables, shared-assessment versus separate-assessment queries, and robust dominance with overlapping expectation intervals. Check extrema witnesses against the original constraints and objective.
* **Any later annotation lane:** direct small-world enumeration, repeated leaves, multiple proofs, categorical exclusivity, `A∧¬A`, recursion cycles, annotation improvements without new tuples, and comparison against the appropriate source probability model. Use the product regrouping counterexample to forbid unsupported rewrite assumptions.

The included rational checker is review evidence for the mathematical core, not a substitute for these future engine tests. No engine implementation or test-suite changes are part of this review.

## 7. Recommended v1 scope in ten lines or fewer

1. Add checked, independently tagged 16-byte `Prob` values with `2^63` fixed-point endpoints.
2. Add complement, explicit refinement/compatibility/separation, hull, and partial intersection.
3. Provide clearly named Fréchet scalar enclosures; spell out any independent product assumption and rounding.
4. Declare complete coherent distributions over closed rosters, with explicit assessment/outcome keys and canonical tightening.
5. Preserve event selections within a roster; compute exact event bounds and deterministic category regrouping.
6. Add exact-accumulation lower/upper expectation over explicit numeric payoffs, with appropriately typed outward-rounded results.
7. Add shared-distribution robust preference and optional extremizing-distribution witnesses.
8. Import jev forecasts honestly; retain confidence and calibration/evidence provenance as ordinary data.
9. Defer generic probabilistic tuple tags, probabilistic negation, recursive inference, and exact general conditioning; retain a clear path to constrained credal relations.
