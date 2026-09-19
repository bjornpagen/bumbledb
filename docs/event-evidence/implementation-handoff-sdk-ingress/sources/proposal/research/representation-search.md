# Research adjudication: canonical representations under algebraic work

This round reads five additional versioned arXiv papers to decide which
representations deserve implementation and what a fair comparison must charge.
The [source manifest](representation-search/sources.json) records PDF hashes,
retrieval URLs and times; adjacent text files are extracted reading copies.
The papers establish the qualified results below. The Rust timings belong to
[our own experiment](../experiments/event-repr-lab/REPORT.md), not to these papers.

## Canonicality has an operation cost

**Guy Van den Broeck and Adnan Darwiche, _On the Role of Canonicity in Bottom-up
Knowledge Compilation_, [1404.4089v1](https://arxiv.org/abs/1404.4089v1).**
Read the partition definitions, Algorithm 1, Theorems 1, 3 and 4, the empirical
comparison, and the relevant conditioning/forgetting arguments in the appendix.

An SDD decomposes a function into a partition of primes over one variable block
and corresponding subs over another, organized by a vtree. Compression merges
equal subs by disjoining their primes. This can share structure a linear variable
order fails to expose. It also gives canonicality for the specified reduced form
and fixed vtree; arbitrary circuit hash-consing alone does not give that property.

The critical distinction is **unreduced versus reduced output**. Algorithm 1 has
polynomial Apply without compression. Theorems 1 and 3 exhibit exponential costs
when canonical reduction is required, including bounded conjunction/disjunction,
conditioning on a literal, and forgetting one variable. This does not say SDDs
are always slow: the paper's experiments find reduced SDDs practically much
better than unreduced ones because sharing, duplicate elimination and cache
identity matter over sequences of operations.

Theorem 4 supplies a useful qualified escape: Apply, including compression, is
O(nm) for reduced SDDs respecting a **bounded vtree**, with fixed bound on the
number of variables in each left subtree. It does not prove all balanced vtrees
have cheap compression. Nor does it settle which bounded decomposition wins
our queries.

**Decision:** compare the entire canonical manager. If a candidate defers
normalization, include its eventual equality/publication/observation cost.
A general SDD and a bounded-block decomposition remain serious competitors;
neither gets a universal performance promise. Our packed-terminal prototype is
an ordered Shannon decomposition, not an implementation of general SDD Apply.

## Compressing chains can change the relevant unit of work

**Randal E. Bryant, _Chain Reduction for Binary and Zero-Suppressed Decision
Diagrams_, [1710.06500v1](https://arxiv.org/abs/1710.06500v1).**
Read the introductory size comparisons, chain definitions/reductions, §7's
implementation, benchmark encoding comparisons, and §9 on complement edges.

CBDDs and CZDDs compress repeated OR/zero and don't-care chains. A long run of
levels need not consume one node per level. In the modified CUDD implementation,
two 16-bit level fields fit in an existing 32-bit index field; there was no
node-size increase from that particular layout change. This is an implementation
observation, not a guarantee for our Rust layout or an unlimited coordinate space.

The main stated size-ratio comparisons exclude the complemented-edge advantage.
The implementation discussion explicitly warns that complemented edges invalidate
direct use of those ratios. §9 reports an example where complement edges change
operation count more than final node count. Counting nodes alone is therefore
insufficient to rank our root-pair, anchored, or packed designs.

**Decision:** chain reduction remains an unimplemented challenger. Test one-hot
categorical constraints, long sparse chains and repeated frame constraints;
include bytes, Apply visits, coordinate movement and abstraction, with a matched
complement convention. Do not claim the paper's ratios for our complemented BDD.

## Sharing renamed structure is different from merging outcomes

**Kengo Nakamura, Shuhei Denzumi and Masaaki Nishino, _Variable Shift SDD: A More
Succinct Sentential Decision Diagram_,
[2004.02502v1](https://arxiv.org/abs/2004.02502v1).**
Read the construction and canonicity discussion, Theorem 13, Apply in §7, and
Appendix A's query/transformation qualifications (including Proposition 15).

VS-SDDs share subgraphs under specific variable substitutions using offsets.
That is directly interesting for successive turns, repeated card slots and
repeated state-transition templates. A common relation shape can be stored once
while its coordinate binding changes.

The polytime Apply claims concern the appropriate uncompressed representation.
The appendix explicitly distinguishes compressed VS-SDDs: bounded conjunction
need not preserve a polynomial-size compressed result. It explains why some
queries can still use a temporary uncompressed conjunction and count it without
requiring canonical output. This is a valuable query optimization, but it is not
free publication of a canonical Event.

**Decision:** semantic coordinate identity must remain separate from variable
position and shared graph shape. Equal shapes on two different draws do not make
them the same event and do not establish independence or shared randomness.
Measure substitution/renaming, repeated transition templates, and the cost of
converting an intermediate result into the arena's canonical representation.
Our current local-permutation lane tests that cost for existing candidates;
it does not implement variable-shift SDDs.

A further reading of Theorem 6 and Proposition 14 makes the cache boundary
specific. Structure sharing requires isomorphic vtree subtrees and the paper's
identical-vtree rule. Its normalized Apply can omit a common offset because
the paired operands and result move together under the permitted substitution.
An arbitrary coordinate map, support restriction or independently renamed
operand does not meet that hypothesis. In a future fused Event product, retain
the checked relative maps in the subproblem identity until a proved symmetry
justifies removing them. The [Lean map laws](../experiments/event-repr-lab/LEAN.md)
settle structural readout and quantifier conditions, but do not establish a
canonical quotient of arbitrary renamed shapes or a polynomial Apply bound.

## Support tags help some shapes, and have real bytes

**Deyuan Zhong, Mingwei Zhang, Quanlong Guan, Liangda Fang, Zhaorong Lai and
Yong Lai, _Variants of Tagged
Sentential Decision Diagrams_,
[2312.00793v1](https://arxiv.org/abs/2312.00793v1).**
Read the representation variants and canonicality claims, node/edge storage
comparison, and §IV's Apply and OrthogonalJoin algorithms.

Tagged variants combine standard and zero-suppressed reductions and make the
relevant variable domains explicit. Node-based and edge-based tags have different
storage costs; the paper gives examples where the preferred choice changes with
sharing. A smaller abstract graph need not occupy fewer bytes.

§IV explicitly states O(n|α||β|) Apply **without compression** and worst-case
exponential complexity with compression. Its OrthogonalJoin is an operation on
combination sets over disjoint variable blocks. That name does not make it
bumbledb's Free Join algorithm or prove a native integration benefit.

**Decision:** support/decomposition metadata is part of the representation budget.
Keep tagged SDDs as a candidate for sparse structured worlds. Evaluate exact
support-relative complement and canonical equality, not just stored positive
combinations or the size of an unnormalized temporary graph.

## “Effect algebra” does not mean every desirable algebra at once

**Kenta Cho, Bart Jacobs, Bas Westerbaan and Abraham Westerbaan, _An Introduction
to Effectus Theory_, [1512.05813v1](https://arxiv.org/abs/1512.05813v1).**
Read Definition 15, Exercise 16, the following definition of tests, Proposition 17,
and Theorem 28's state/effect duality. This is a targeted reading of those results,
not a claim to have checked the whole 150-plus-page development.

An effect algebra is a partial commutative monoid with a unique orthosupplement.
Two concrete instances in Definition 15 settle an ambiguity in our terminology:

- Boolean events form an effect algebra when addition is **disjoint union**.
  `E ⊕ F` is defined when `E ∩ F = ∅`; complement is relative to full support.
- Numeric effects in `[0,1]` add when their sum is at most one; complement is
  `1-p`. These are not sets of admissible worlds.

A test is a finite orthogonal family whose defined sum is one. Our existing
pointwise FD disjointness plus IND coverage expresses precisely that partition
structure for sharp events. Under an admitted normalized law, measuring the
partition gives numbers adding to one. Event rows need no separate weights.

The general effect-algebra axioms do not automatically supply arbitrary Boolean
meet, nondeterministic relational composition, converse, residuals, or finite
reachability. Those capabilities come from the proposal's separately specified
relations on typed faces and its Kleene-algebra-with-domain lineage.
Nor does the word “effect” establish a quantum/noncommutative model.

**Decision:** preserve Event's sharp possibility semantics and the full relational
interface. Study probabilistic effects through observables, channels and law
contraction, with their own admitted numeric structure. Do not weaken the current
algebra merely to inherit a broader-sounding name. The
[earlier adjudication](algebra-resolution.md) governs KAD, information abstraction,
source laws and inference updates.

## What “best” can mean in this experiment

There are 2^(2^n) subsets of an n-bit world domain. A counting argument already
rules out a representation using polynomially many bits for every such subset,
including the event-specific shared arena data behind a small handle.
This does not select one implementation, or prove no algorithm can dominate a
particular finite benchmark. It establishes why a claim to a universally compact
exact carrier would need qualifications the workload cannot remove.

The defensible target is the fastest exact canonical representation **for a stated
family of operations, presentations and lifecycle costs**, with explicit failure
limits. Current evidence distinguishes at least:

1. finite materialized worlds with frequent Boolean/grouping work;
2. contiguous sets;
3. structured relational elimination;
4. coordinate movement and repeated relation composition;
5. symbolic worlds too large to enumerate;
6. construction, updates, law contraction and persistent publication.

Only measured lanes can select a winner within their contract. Compactness under
one ordering, a cheap Boolean Apply, and a fast native row join answer different
questions. The new packed candidates test a concrete middle ground; their fixed
leaf boundary remains a hypothesis to attack with coordinate movement.
