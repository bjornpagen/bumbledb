# Research adjudication: decomposition, renaming, and reachability

This round adds an exact Rust block-diagram competitor and compares face-major,
bit-major and paired-bit orders. Three additional versioned primary papers are
retained in [the manifest](block-decomposition/sources.json). The earlier
[SDD/canonicity review](representation-search.md) remains applicable.

## AND/OR diagrams expose a real alternative, with a real boundary

Robert Mateescu, Rina Dechter and Radu Marinescu, **AND/OR Multi-Valued Decision
Diagrams (AOMDDs) for Graphical Models**, JAIR 33 (2008), pp. 465–519;
[arXiv:1401.3448v1](https://arxiv.org/abs/1401.3448v1).
Reading scope: introduction and decomposition definitions; Theorem 4; §§6.2–6.3
and Algorithm 3; §7's canonicity statement; §8's weighted normalization and finite
precision qualification. This is a targeted reading, not a claim to have checked
every empirical result in the 55-page paper.

AOMDDs add AND nodes for function decomposition to multi-valued decisions.
Their attractive size bound depends on the graphical model's induced width
under the pseudo-tree rather than simply treating that tree as one linear path.
Theorem 4 states the relevant `O(n k^w)` construction/size bound with its domain
size and induced-width parameters. It does not say every function has low width.

The exact restriction matters for this proposal. Theorem 6 and Corollary 1 give
canonicality for equivalent constraint networks **accepting the same pseudo-tree**.
The introduction to §7 explicitly distinguishes this from canonicality across
all equivalent graphical models. The APPLY operator in §6.2 combines the model's
functions under `⊗`, with the stated compatible/strictly-compatible pseudo-tree
conditions. Proposition 3's product-size bound is under those conditions.
It is not a theorem that our entire sixteen-operator and relational interface
retains the same decomposition at no cost.

A minimal counterexample explains why this is substantive. Let two independent
coordinate blocks be Boolean `X` and `Y`, and let `E = {X=1} × {Y=1}`. This is a
product event. Its complement has three points. Both coordinate projections are
full, whose Cartesian product has four points, so the complement is not a product
on those same blocks. One can embed this below an unused root of a forked
pseudo-tree: each root cofactor still has the same failure. A representation may
change its decomposition or introduce a richer sum/partition structure; it cannot
pretend the original independent fork still describes that complement.

“Independent blocks” here describes factorization of the characteristic function,
not a probability assumption. The designated joint law can still couple the named
coordinates. Event decomposition never establishes probabilistic independence.

**Decision:** keep AOMDDs as a genuine broader competitor, especially for structured
source-law contraction. Charge closure under complement, abstraction and changes
of pseudo-tree before claiming them as the universal resident Event manager.
A small product formula with deferred recompilation is not yet the same contract
as constant-time scoped equality of published results.

## Weighted canonicity needs exact normalization

Robert Mateescu and Rina Dechter, **AND/OR Multi-Valued Decision Diagrams (AOMDDs)
for Weighted Graphical Models**,
[arXiv:1206.5266v1](https://arxiv.org/abs/1206.5266v1).
Read the weighted construction, the normalization example and canonicity statement;
cross-check against the fuller §8 and Theorem 7 of the JAIR paper above.

Weights can be redistributed between levels without changing a global function,
so structural graph interning alone does not give weighted-function identity.
Bottom-up normalization and promotion of constants supplies the additional
invariant in the paper, again under a common accepted pseudo-tree.

The JAIR finite-precision discussion describes an epsilon-tolerance approach used
in its experiments. That is not an acceptable equality rule for exact Event
identity, and it is not automatically an exact canonical law representation.
Our law work needs rational/symbolic normalization, including zero-denominator
domains when normalizers depend on parameters. A numeric approximation can have
an explicit observation contract; it cannot silently redefine admissibility.

**Decision:** retain separate region and joint-law identities. A weighted AOMDD
experiment must preserve exact normalization and declared dependence. Its weights
belong to source-law storage, never to ordinary Event rows.

## Reachability does not always require constructing R*

Sebastiaan Brand, Thomas Bäck and Alfons Laarman, **A Decision Diagram Operation
for Reachability**, [arXiv:2212.03684v1](https://arxiv.org/abs/2212.03684v1).
Read §§2.1–2.6, Algorithms 1 and 3, §3.4's correctness statement, and the
experimental conclusions. The algorithm's skipped-variable details and parallel
implementation have not been transplanted into our Rust code.

The operation computes `S · R*` directly for a given initial set. §3.1 explicitly
distinguishes this from constructing the whole reflexive-transitive relation
`R*`, which can cost much more. Its recursive fixed points work over smaller
state blocks; image operations carry newly reached states between those blocks.
Theorem 1 states correctness for ReachMdd. The BDD presentation assumes the
specified quasi-reduced/interleaved structure; our reduced diagrams and arbitrary
maps would require adaptations, not a mechanical copy of the pseudocode.

The MDD discussion adds an important cost warning: a local domain of size `m`
gives `O(m)` recursive Reach calls but `O(m²)` Image calls per loop iteration.
Larger decisions reduce depth while potentially increasing local cross-products.
This is directly relevant to testing 64-way blocks rather than assuming that
wider branching always wins.

The experiments show workload dependence: Reach tends to do well with lower
transition locality; saturation has advantages elsewhere. The paper's 29% result
against ITS-tools is not a universal superiority claim or a benchmark of our
implementation.

**Decision:** add a dedicated matched experiment for `May(R*, Goal)` implemented
as a backward reachable-state computation, alongside materialized `R*` followed
by May. Return the same state event and retain exact equality. The current full
relation lane also returns `R*`, its converse and a residual, so removing those
outputs to make a “faster closure” would weaken the benchmark. General monotone
fixed points, universal attractors and game strategies need their own justified
algorithms; Reach is not a drop-in solver for all of them.

### Independent check of the backward block equations

A further reading of §3.4 checks the argument needed for a backward goal query.
Replace forward images with preimages under the corresponding converse blocks.
For a state split into low/high parts, the update schedule becomes:

```text
G0 := ReachBackward(R00, G0)
G1 := G1 union May(R10, G0)
G1 := ReachBackward(R11, G1)
G0 := G0 union May(R01, G1)
repeat until neither changes
```

The direction on cross-block edges matters: `R10` contains high-to-low edges,
so its preimage propagates a low target into high predecessors. Taking the
converse of the full relation reduces this schedule to the paper's forward
equations. Every update is monotone on a finite state set; diagonal recursive
calls compute reachability inside their block before the next cross-block step.

[The independent checker](block-decomposition/reach_checks.py) passes
[393,304 finite cases](block-decomposition/reach-checks.json): all one/two-state
relations and goals, every four-state relation with empty/full/singleton goals,
and larger dead-ended chains. It compares the block recurrence with ordinary
iterated preimage, using explicit relations. This is evidence for the mathematical
adaptation, not a Rust graph kernel, a benchmark or a claim to reproduce Sylvan.

The [new diagonal experiment](../experiments/event-repr-lab/DIAGONAL.md) separately
removes world enumeration from relation-program preparation and adds a native
long-chain fixture returning full closure. Its full-result lane remains the
control that a state-only rewrite must not silently replace. A next matched
state-only lane can compare full closure followed by May, cumulative iteration,
frontier iteration and the checked block recurrence. Semantic cofactors must
account for skipped coordinates; direct child access is not valid merely
because the algorithm is written recursively.

### Test the adverse schedule too

A further reading of §3.2 and Appendix A supplies a required competing fixture.
The paper's counter is the ideal family for recursive Reach. Add a leading
state bit that toggles on every counter transition. Both diagonal blocks at
that split become empty, so recursive calls cannot accelerate progress there;
the cross-block preimages carry the entire computation. This is the paper's
bad-case construction applied to our backward goal schedule.

[A separate exact schedule check](block-decomposition/reach_schedules.py)
retains [sixteen cases](block-decomposition/reach-schedules.json), with ordinary
iterated preimage as its independent reference. For an eight-bit counter,
recursive backward Reach uses 46 preimage calls versus ordinary iteration's
256. The alternating-bit version uses 258 versus 256. Both return the complete
correct reachable-state set. These are semantic operation counts on explicit
finite relations, not graph visits or Rust timing; local preimage costs differ.

Therefore a new native state-only Reach lane must contain both families, plus
the existing bounded-component fixture. Retain full closure followed by May as
one same-answer control and preserve the separate five-output full relation
lane. Canonical recursive subproblem sharing helps the counter; it cannot
establish a generally superior reachability schedule from that one family.

## Why a bounded partition block is the current experiment

The implemented `block64` uses ordered coordinate groups of at most six bits.
An outgoing edge carries a disjoint selector set in one `u64`; equal continuation
functions share one selector. The last block is a truth table. This is a concrete
reduced ordered MDD with compressed selector partitions, not an AOMDD or general
SDD implementation and not a claim of historical novelty.

The bounded-vtree result in the earlier SDD paper provides the relevant design
intuition: when a left block has bounded size, exact prime-set operations and
canonical compression stay bounded locally. Our stronger, simpler ordered-block
construction has its own inductive uniqueness argument in
[BLOCKS.md](../experiments/event-repr-lab/BLOCKS.md).

A useful derived fact concerns coordinate maps. If a permutation maps each block's
coordinate set to itself, only its selector tables and descendant functions need
permutation. Decision levels do not change. This is a property the constructor
can check; it does not depend on guessing which individual events are “simple.”

For three state faces, interleaving corresponding bits puts two bits from each
face in a six-coordinate block. Face permutations then stay inside each block.
A face-major order puts different faces in different blocks and requires general
canonical reordering. Both represent exactly the same worlds. The experiment
charges the actual map implementation in each case.

These are hypotheses about a concrete representation and operation mix.
Nonuniform/shared-parameter law contraction, persistence, live updates and
reclamation still require their own matched evidence.
