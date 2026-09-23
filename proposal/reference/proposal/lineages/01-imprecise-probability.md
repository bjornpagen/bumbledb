# 01 — Imprecise probability: intervals, coherence, capacities, Choquet

The mathematical core of candidate A. This lineage owns the *type* and the
*laws*; it does not own propagation through joins.

## Walley's coherent lower previsions (1991) and the two rationality axioms

- A lower prevision `P̲` on gambles is **coherent** iff it extends to a coherent
  lower prevision on a linear space; on indicators of events it is a coherent
  lower probability. Weaker: **avoiding sure loss** (ASL). ASL assessments can
  always be "corrected and extended ... in a least-committal manner" to the
  **natural extension**: "the (point-wise) smallest, and therefore most
  conservative, coherent lower prevision on L(X) that dominates P on K."
  (Miranda, IJAR 2008 survey; de Cooman et al., arXiv:0801.1265.)
- Duality: `P̄(A) = 1 − P̲(A^c)`. A credal set (closed convex set of
  distributions) and a coherent lower prevision are equivalent ("somewhat
  puzzling that coherent lower previsions end up being as expressive as credal
  sets" — Cuzzolin).
- Independence is not one notion: **strong independence** (independence at
  every extreme point of the joint credal set) is what credal networks and
  credal circuits use. Epistemic irrelevance is weaker. The Fréchet product of
  lower bounds `l₁·l₂` is the strong-independence lower bound for a conjunction.

## Weichselberger's interval probability (IJAR 2000)

Axioms added to Kolmogorov's. Two grades:
- **R-probability**: interval limits "which are not self-contradictory" = avoids
  sure loss.
- **F-probability**: limits "fit exactly to the set of classical probabilities
  in accordance to these interval-limits (structure)" = every bound attained =
  coherent/reachable.
- Checking R vs F on a finite space "is done by means of Linear Programming."
  F-probability "can be uniquely determined ... by specifying only part of the
  parameter set."

Same two grades appear in de Campos as non-emptiness vs reachability, and in
Walley as ASL vs coherence.

## de Campos, Huete, Moral — probability intervals (IJUFKS 1994)

The canonical paper for **candidate A**. Not on arXiv; summarized from
secondary sources and Destercke et al.

### Definition

`L = {[l(x), u(x)] | x ∈ X}`, credal set `P_L = {P | l(x) ≤ p(x) ≤ u(x)}`.

### The two laws

```
non-emptiness (ASL):    Σ_x l(x) ≤ 1 ≤ Σ_x u(x)
reachability:           ∀x:  u(x) + Σ_{y≠x} l(y) ≤ 1   and   l(x) + Σ_{y≠x} u(y) ≥ 1
```

"A probability interval L is called reachable if the credal set P_L is not
empty and if for each element x ∈ X, we can find at least one probability
measure P ∈ P_L such that p(x) = l(x) and one for which p(x) = u(x)."

Reachability **correction** (closed form, no LP):

```
u*(x) = min( u(x), 1 − Σ_{y≠x} l(y) )
l*(x) = max( l(x), 1 − Σ_{y≠x} u(y) )
```

### Event bounds (closed form, no LP)

```
P̲(A) = max( Σ_{x∈A} l(x),  1 − Σ_{x∈A^c} u(x) )
P̄(A) = min( Σ_{x∈A} u(x),  1 − Σ_{x∈A^c} l(x) )
```

"De Campos et al. have shown that these lower and upper probabilities are
Choquet capacities of order 2." (Destercke/Dubois/Chojnacki, eq. 2.) They are
**only** 2-monotone in general, not ∞-monotone: "upper and lower probabilities
induced by reachable probability intervals are order 2 capacities only."

### Parameter count

`2|X|` numbers. Compare: general lower probability `2^|X| − 2`; random set
`2^|X| − 2`; possibility distribution `|X| − 1`; generalized p-box `2|X| − 2`.
Probability intervals are the cheapest representation that is still 2-monotone
on events.

### Operations studied in the paper

Combination, marginalization, conditioning, integration. Aggregation of
probability intervals from several sources (Moral & Sagrado, "Aggregation of
imprecise probabilities," 1998) covers conjunctive (intersection) and
disjunctive (hull) pooling.

## Why 2-monotone matters: Choquet gives exact expectation

- A lower probability is 2-monotone iff `P̲(A) + P̲(B) ≤ P̲(A∪B) + P̲(A∩B)`.
  "The weakest property that readily admits simple closed-form manipulations."
- **Theorem (de Cooman, Troffaes, Miranda, arXiv:0801.1962):** for exact
  functionals, "2-monotonicity of a lower prevision is actually equivalent to
  comonotone additivity, and therefore to being representable as a Choquet
  functional." And: the natural extension "is the only extension to be
  n-monotone." So for a 2-monotone lower probability the natural extension of
  any gamble `f` **is the Choquet integral**:

  ```
  E̲(f) = inf f + ∫_{inf f}^{sup f} P̲({f ≥ t}) dt
  ```

  On a finite space: sort the values of `f` descending, `f(x₁) ≥ … ≥ f(xₙ)`, then
  `E̲(f) = Σ_i f(x_i) · [ P̲({x₁..x_i}) − P̲({x₁..x_{i−1}}) ]`. O(n log n), no LP.
- For probability intervals the level-set `P̲` is the closed form above, so a
  **lower expectation over a ranked roster is exact and cheap**. This is what
  makes TypeSafe's `score` an honest interval: `[E̲(rank), Ē(rank)]` under the
  interval vector, tighter than the naive `[Σ i·l_i, Σ i·u_i]`.
- Abellán & Moral 2003 give a quadratic-time algorithm for the maximum-entropy
  distribution in a probability-interval credal set; Vu et al. 2026
  (arXiv:2603.23558) revisit upper entropy for 2-monotone lower probabilities via
  the base polyhedron of submodular optimization.

## The expressiveness lattice (Destercke, Dubois, Chojnacki, arXiv:0808.2747)

Chain of generalization, most to least expressive:

```
credal sets ≡ coherent lower previsions
  ⊃ coherent lower/upper probabilities
    ⊃ 2-monotone capacities           ← probability intervals live here
      ⊃ random sets / belief functions (∞-monotone)
        ⊃ generalized p-boxes
          ⊃ { p-boxes , possibility distributions }
            ⊃ precise probability
```

Key relation: "there is no generalization relationship between probability
intervals and random sets." Intervals and belief functions are **incomparable**
siblings under 2-monotone capacities. A generalized p-box is "a pair of
comonotonic mappings" = bounds on a nested family `α_i ≤ P(A_i) ≤ β_i`; it is
"representable by a pair of possibility distributions" and "a special kind of
random set." Probability intervals are intersections of σ-p-boxes over
permutations (Prop. 4).

Cuzzolin (arXiv:2104.06839) draws the same hierarchy with belief functions a
special class of interval probabilities on events, and argues for random sets as
the practical compromise (see lineage 02).

## Credal sets proper, when intervals are not enough

- For a **binary** proposition, a credal set on `{x, x̄}` is exactly one interval
  `[l, u]`. Intervals are lossless.
- For `|X| ≥ 3`, a credal set is a polytope; an interval vector is its bounding
  box. Many polytopes share a box. The domain-theoretic paper (arXiv:2604.09272,
  2026) makes this concrete: the image of an event under a credal set "need not
  be an interval" ("will be {0,1}" for a Cantor IFS example), so intervals give
  "only an outer (envelope) approximation."
- Domain theory (2026): the unit-interval domain `𝕀[0,1]` of "non-empty closed
  intervals of the unit interval ordered by **reverse inclusion**" is the value
  domain; credal sets are the upper space `U(P(X))` of compact subsets under
  reverse inclusion, whose convex sub-domain "has a basis consisting of convex
  polytopes." Event intersection is `(O₁,O₂) ∩ (U₁,U₂) = (O₁∩U₁, O₂∪U₂)`; lower
  endpoints multiply under conditional independence, upper endpoints are only
  Fréchet-bounded (`≤ min{...}`). "All operations extend Scott-continuously to
  spaces of credal sets." Compositionality *is* Scott continuity; the ordering
  that makes recursion converge is **reverse inclusion** (information order).

## What this lineage decides

1. **The type for a Boolean proposition is `[l, u]`**, closed, degenerate
   allowed, and it is lossless.
2. **The type for a K-roster is a reachable interval vector**, `2K` numbers,
   with the two closed-form laws (ASL, reachability) judged exactly and a
   closed-form correction available.
3. **Event bounds and expectations are exact via the closed forms and Choquet**
   because the induced capacity is 2-monotone. No LP is needed for anything
   the schema can express about one roster.
4. **The information order is reverse inclusion** (`[l,u] ⊑ [l',u']` iff
   `[l',u'] ⊆ [l,u]`). Meet in that order is intersection (conflict if empty);
   join is hull. This is the order recursion must be monotone in.
5. What intervals **cannot** do: represent a non-box credal set over three or
   more options, or a joint credal set over several propositions. Those need
   lineage 05.
