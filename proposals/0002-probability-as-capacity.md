# Proposal 0002: probability laws as capacity laws over worlds

Status: **research first, nothing to implement.** Written 2026-10-08 at
494caf52. It reopens the unmerged event work (`codex/event-native-design` at
bc4814c0, `codex/event-algebra` at 79c4629) from a different angle. Nothing
here changes the engine, and no syntax below compiles.

## 1. Why revisit the event work

The event design made an Event a value, `(U, A)`: a finite caller-named
universe of worlds and a subset of it. It deliberately kept probability out:
"no probability law, normalization equation, strategy … is stored inside an
Event", a model score is "an ordinary supplied fact", and Events get "no
duration or point-count capacity" (`proposal/proposal.md`,
`proposal/decisions.md` on that branch).

The design was coherent, and the owner still never merged it. This proposal
tests a hypothesis about what was missing: **probability belongs in the law
language, not in values, and the law that already carries it is capacity.**
Events stay sets of worlds; weights on worlds are what the laws talk about.

## 2. Three observations

### 2.1 A distribution is already a capacity law

A conditional distribution says that for each conditioning group, the
outcome weights sum to exactly one. With integer weights out of N, that is an
exact capacity window:

```math
\forall x:\ \sum_{y} p(y \mid x) = 1
\qquad\Longleftrightarrow\qquad
X(id) \le_{[p]}\{N..N\}\ P(x)
```

A Markov kernel is the same statement, and a single distribution is the one
group case. Duration weights already measure time: "fraction of the span" is
the uniform probability measure on an interval, so recipe 32's calendar
capacity is a measure bounded by a total measure.

There is one gap. Marginal consistency needs each group's sum to equal a
field on the target:

```math
\sum_{y} p(x, y) = p(x)
```

That is a dependent exact window, which needs a dependent floor. Ruling C6
(2026-07-24) refuses dependent floors, so today bumbledb can state "at most
the marginal" but not "equal to the marginal".

### 2.2 Capacity bounds are entropy bounds

A unit window `{0..hi}` caps every group at hi source facts. For any
distribution supported on the relation, the conditional entropy is then
bounded:

```math
\deg_S(Y \mid X) \le hi \ \Longrightarrow\ H(Y \mid X) \le \log hi
```

An FD is the case hi = 1, where H = 0. This is the step the polymatroid
output-size bounds rest on (put the uniform distribution on the output and
turn each degree constraint into an entropy inequality). It is also the
machinery proposal 0001 wants for delta-size bounds. The two proposals share
one piece of mathematics.

### 2.3 Bounds on event probabilities form a polytope, and refusal is a dutch book

Suppose the weights are not stored at all. A forecaster states only bounds on
events, `lo ≤ P(A) ≤ hi`, for a handful of events. Every bound is linear in
the unknown world weights w, so the distributions consistent with the stated
laws form a polytope:

```math
\mathcal{P} = \Big\{\, w \ge 0 \;:\; \textstyle\sum_{\omega} w_\omega = 1,\;
lo_A \le \sum_{\omega \in A} w_\omega \le hi_A \ \ \forall A \,\Big\}
```

- **Coherence is feasibility.** The stated bounds are coherent exactly when
  this polytope is non-empty (de Finetti's fundamental theorem of
  probability, as linear programming).
- **The refusal is a dutch book.** When the system is infeasible, LP duality
  (Farkas' lemma) yields multipliers on the violated bounds that prove the
  contradiction. Read as bets, those multipliers are a book that loses in
  every world. The judge's "every violation, cited" would become "the dutch
  book, cited".
- **The name fits.** In measure theory a capacity (Choquet) is a monotone set
  function generalizing probability. It underlies lower and upper
  probabilities, belief functions and credal sets, which is this polytope.

## 3. Two modes, and only one is new

**Precise mode: weights are facts.** Worlds carry stored weights, membership
is a relation, and laws check sums. The existing capacity evaluator already
does this arithmetic. What is missing is small and known: dependent floors
(C6) for marginal consistency, and pinned-copy weights, because path weights
are refused and a membership fact has to carry its world's weight through a
pinned containment (the cookbook's recipe 31 idiom).

**Imprecise mode: weights are unknowns.** Laws constrain an unknown weight
vector, and the judge asks whether **any** distribution satisfies them all.
That is a new kind of judgment. Every law the judge checks today is universal
over stored facts. This one is existential over weights, decided by LP
feasibility, with a certificate as its refusal. It is the research core of
this proposal.

## 4. Research questions

1. **Expressiveness.** Which probability statements are linear in world
   weights and therefore capacity-shaped? These include normalization,
   conditioning (`P(A ∩ B) ≥ c · P(B)`), marginals and event bounds.
   Independence, `P(A ∩ B) = P(A) P(B)`, is not linear, so it is outside the
   polytope by construction. Map the boundary, and say what the law language
   gives up by staying linear.
2. **Lifting C6.** Write down why dependent floors were refused and what
   breaks if exact dependent windows are allowed. Marginal consistency is the
   first consumer.
3. **Existential judgment.** Final-state judgment plus LP feasibility:
   - exact rational arithmetic, because the engine canonicalizes floats and an
     LP verdict must not depend on rounding
   - rational versus fixed-denominator weights, since fixing N turns
     feasibility into integer programming
   - rendering the Farkas certificate as citations
   - incremental re-solving across commits
4. **Scale through structure.** An explicit U gives an LP polynomial in |U|.
   Implicit worlds, as assignments to propositional variables, make coherence
   probabilistic satisfiability, which is NP-complete in general
   (Georgakopoulos, Kavvadias, Papadimitriou 1988). Dependency theory is the
   way out. For an acyclic hypergraph of marginals, local consistency implies
   a consistent joint (Vorob'ev 1962). This is the same acyclicity as acyclic
   join dependencies (Beeri, Fagin, Maier, Yannakakis 1983). The candidate
   theorem is: coherence is cheap exactly when the forecast schema is
   acyclic.
5. **Entropy as a planning tool.** Use section 2.2 to bound join and delta
   sizes from capacity laws (shared with proposal 0001), and to measure
   near-dependencies, `H(Y | X)` and `I(Y; Z | X)`, on real stores.
6. **Scoring stays outside the law system.** Resolution is a fact naming the
   realized world. Brier and log scores are queries over resolved forecasts,
   and per-relation versions give the point-in-time filtration. Confirm that
   nothing about scoring needs a new law.
7. **What becomes of Event.** Either Event stays a dense-bitmap value and
   weights live beside it, or events become a membership relation over a
   world relation and the bitmap is a physical representation. Section 2.3
   prefers whichever makes "sum of weights over A" a capacity statement
   without a new evaluator.

## 5. Prior work to read first

- de Finetti, *Theory of Probability*: coherence and the fundamental theorem
- Hailperin, *Boole's Logic and Probability*: probability bounds as linear
  programs
- Nilsson (1986), probabilistic logic; Georgakopoulos, Kavvadias,
  Papadimitriou (1988), probabilistic satisfiability
- Walley (1991), *Statistical Reasoning with Imprecise Probabilities*: credal
  sets, lower previsions, coherence
- Choquet (1954), theory of capacities; Dempster and Shafer, belief functions
- Vorob'ev (1962), consistency of families of marginals; Beeri, Fagin, Maier,
  Yannakakis (1983), acyclic database schemes
- Lee (1987), entropy and multivalued dependencies
- Abo Khamis, Ngo, Suciu (2017), degree constraints and Shannon-type
  inequalities
- Lenzerini and Santucci (1983), cardinality constraints; Ross, Srivastava,
  Stuckey, Sudarshan (1998), aggregation constraints
- Suciu, Olteanu, Ré, Koch, *Probabilistic Databases* (2011)

## 6. Decision gates

1. **Step zero, on paper.** Rewrite the event branch's coup example
   (`proposal/coup.md`) as precise-mode capacity laws in today's language.
   Record every statement that refuses and why (C6, path weights, anything
   else). That list is the precise-mode gap, measured instead of guessed.
2. If the precise-mode gap is only C6 and pinned weights, decide on C6 by
   itself. It is small and useful without any of the rest.
3. Imprecise mode starts with questions 1, 3 and 4 written up as claims with
   proofs or counterexamples, and the prior-work read done. No code until an
   LP verdict and its certificate are specified exactly, including
   arithmetic.
4. The event branches stay unmerged until this proposal says what Event
   should be (question 7).
