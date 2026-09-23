# One Event, several exact observations

The experiment now separates the Event's identity from an exact law
interpretation. All fourteen candidates can contract an event into assignment
counts by named parameter group. Those counts can be observed under several
explicit laws without reconstructing the event or replacing it with a number.
These are explicitly selected interpretations of the structural region; they
do not silently replace the designated law of a production Event owner.

This implements one source fragment: finite binary draws, conditionally
independent given their group's parameter. Reusing a parameter group preserves
dependence after that parameter is integrated out. Allocating an independent
group is an explicit law-construction choice. FD/INDs do not infer it.

## The useful counterexample

Let two draws share `theta ~ Beta(2,3)`. Each has probability `2/5` of heads.

| Exact question | Shared parameter | Separate independent parameters |
| --- | --- | --- |
| First heads | `2/5` | `2/5` |
| Both heads | `1/5` | `4/25` |
| Second heads given first heads | `1/2` | `2/5` |

The Boolean events are ordinary regions in both cases. The law supplies the
coupling. Multiplying the two already-integrated marginals loses that coupling.
Copying the first Event is a third construction: it retains the same outcome,
so `P(copy & first) = P(first)` and disagreement is empty.

At forty coordinates the shared checker verifies, without enumerating `2^40`
worlds, that all forty heads has probability `1/41` under one shared uniform
Beta parameter and `1/2^40` under a fixed fair parameter. One head with 39 unused
draws still has probability `1/2`. These are exact rationals. A possible head at
the fixed-zero endpoint has zero mass and remains nonempty; conditioning on it
is undefined.

## What is in memory

`CountPlan` holds a coordinate-to-parameter-group map, per-group degrees,
mixed-radix coefficient strides, and prepared contraction masks for physical
word tables. A `Spectrum` is a `Vec<u64>` of accepted assignment counts. For
degrees `n_g` its interpretation is

```text
P_theta(E) = sum_h count[h] * product_g theta_g^h_g (1-theta_g)^(n_g-h_g)
```

These are coefficients in the unnormalized Bernstein basis, not probability
weights on database facts. The coefficient count is `product_g (n_g+1)`.
This observation representation is not injective on Events: first-heads and
second-heads have the same spectrum in one exchangeable group while denoting
different worlds. Coefficients therefore cannot replace canonical Event
identity or decide a pointwise key or containment.
The current cap is one million bins and fewer than 63 outcome coordinates;
the finite number of assignments bounds these counts below `2^63`. Arbitrary
polynomial expressions do not inherit that bound.

Decision nodes sum disjoint alternatives. Selector blocks multiply local
selector counts by continuation counts over disjoint coordinates. Skipped
coordinates multiply the count-generating polynomial by `(1+t_g)`; they must
not vanish merely because a Boolean node skips them. Padding is fixed false,
not a stochastic draw. Relative complement subtracts the region spectrum from
the support spectrum. Each observation currently uses a fresh traversal memo.

Packed tables use prepared disjoint masks: a contribution is
`popcount(word & mask)`. Dense sets borrow their words directly; sparse,
Roaring and run carriers currently materialize a dense observation view, and
that conversion is charged to contraction. Symbolic carriers prepare local
blocks instead of a whole-world table. This is another reason to compare full
query-plus-observation cost rather than one Boolean instruction.

`ExactLaw` keeps arbitrary-precision integer numerators with one common
denominator. It evaluates the coefficient array under rational point parameters
or explicitly independent Beta priors between groups. A shared group's
per-assignment mass is

```text
(alpha)_h * (beta)_(n-h) / (alpha+beta)_n
```

Joint coefficients are constructed before integration. Conditional observation
returns `None` when the evidence numerator is zero. No numerical threshold
decides emptiness or equality. The fixed coordinate roster gives a precise
polynomial basis; this does not supply canonical rational functions or an
arbitrary source-law solver.

## Why this fold is justified

[Kimmig, Van den Broeck and De Raedt, arXiv:1211.4475v1](https://arxiv.org/abs/1211.4475v1),
Algorithm 1 and Theorem 2, justify algebraic evaluation when sums have disjoint
models, products have disjoint variables, and skipped-variable contributions
are accounted for. Theorem 4 explains when neutral literal labels permit
omitting smoothing. Our count-generating labels are `t` and `1`, whose sum is
`1+t`, so that shortcut does not apply at the coefficient level. The paper
also explicitly qualifies its complexity results by the cost of semiring
operations: a growing coefficient payload is not a constant-time scalar.
[Retained text](../../review-evidence/kimmig-vandenbroeck-deraedt-2012-algebraic-model-counting.txt).

[Staton et al., arXiv:1802.09598v2](https://arxiv.org/abs/1802.09598v2),
the `ProcessFactory` example and the exchangeability, discardability and
conjugacy equations, distinguish allocating a process from reusing it. The
Beta-Bernoulli and Pólya urn presentations have the same observable behaviour
under their stated abstraction. The small-case checker uses sequential Pólya
urn enumeration as an independent probability oracle for our Beta formula.
[Retained text](../../review-evidence/staton-et-al-2018-beta-bernoulli-algebraic-effects.txt).

These results establish this source fragment. They do not imply that arbitrary
Event intersections have independent probabilities, nor that Boolean
canonicality decides real-parameter feasibility.

## Native experiment contract

The `laws` lane uses the existing full relation program through actual Free
Join: closure, converse, residual, eventual May and one-step Must. It obtains
80 output events, intersects each with explicit evidence, contracts them, and
observes each under a Beta law, its plug-in mean and an all-zero endpoint.
There are 240 exact observations; the endpoint gives 80 undefined conditionals
despite structurally nonempty evidence. Direct world enumeration checks every
coefficient and every final region outside timing.

Times separate algebra, coefficient contraction and rational evaluation, with
fresh and retained arenas and the same outer-memo choice for every candidate.
Prepared mask/law plans are reused; their construction and retained estimates
are separately recorded. Traversal-memo peak memory is not included in retained
arena estimates. Process RSS includes fixture and oracle memory as well.

The law fixture uses full Cartesian admissibility and explicit evidence. For a
parameter-dependent support S, normalizing within each parameter fibre before
integrating and conditioning the already-integrated joint law on S can produce
different answers. This experiment does not choose one by accident or claim
general guarded normalization is implemented.
The restricted-support semantic checks contract ambient-law mass on a subset;
they do not designate that unnormalized restriction as a production context's
law. A measured context must still provide a normalized law, with `P(full)=1`.
The benchmark's Beta source is a controlled law on fixture coordinates, not
a Coup strategy model or transition probabilities inferred from a relation.

Next comparisons can include a normalized polynomial circuit that removes
discarded draws before degree elevation, memoized repeated observation,
native compressed-container contraction, and general factored laws. The
coefficient table is an exact baseline, not a universal arithmetic carrier.

## Actual ARM64 contraction kernel

[The disassembly](results/observation-table-contract-dependency-arm64.s) and
[binary/source metadata](results/assembly-dependency.json) retain the optimized
native `TablePlan::contract` symbol. A nonzero masked word lowers to scalar
`ANDS`, register transfer, NEON `CNT.8B` and `ADDV.8B`, followed by an indexed
integer coefficient update. The two count sites serve the ordinary and
support-XOR paths. This is hardware byte-popcount reduction for a masked word;
it is not a claim to vectorize several independent Event observations at once.
Mask traversal, coefficient indexing, zero checks and bounds checks remain in
the loop. The full-query measurements include that bookkeeping.
