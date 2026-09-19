# Exact finite functions and conditional sources

`FiniteFunction` is an exact rational-valued function on an Event space. It
reuses ordinary Event regions to group worlds having the same value, including
signed values. It is a host arithmetic object, not another database field type.
`FiniteKernel` uses this algebra to construct normalized conditional joint laws.
This extends the [fixed finite source layer](event-sources.md); continuous shared
parameters, solver capabilities, source adapters and observation query
heads retain their separate implementation gates.

## Function algebra

`FunctionPiece { region, value }` inputs must be disjoint, including zero pieces.
Every region is checked/aligned to the captured context. Unspecified worlds have
value zero. Construction merges equal nonzero values and removes empty pieces.
`pieces()` exposes ordinary Event cells and exact values; `at()` inspects one
legal world. Neither function samples a source.

`constant`, `add` and `multiply` construct functions. Addition sums overlapping
contributions exactly; multiplication is pointwise. Negative constants provide
negation and subtraction through these operations. `equivalent` and `align_to`
check the complete designated context before comparing or translating values.
`density(space)` captures its current law. `designate()` accepts only a
nonnegative function with total mass exactly one, creating a new measured space.
It does not rescale an arbitrary function.

For a checked deterministic map `f: Source → Target`:

```text
pullback(v)(s)    = v(f(s))
pushforward(w)(t) = sum_{s: f(s)=t} w(s)
```

Weighted image adds all witnesses. It does not perform existential image,
conditional averaging, or an automatic change to the target's designated law.
It preserves addition and total mass, and satisfies the weighted pairing law:

```text
sum_t v(t) * pushforward(w)(t) = sum_s v(f(s)) * w(s).
```

These are finite outcome sums, not sums over deterministic parameter guards.
A function can have different values at structurally different worlds even when
the designated law gives those worlds zero mass.

## Representation and weighted-image algorithm

A function retains its `Space` and canonical nonzero equal-value cells. Raw
operations use disjoint scalar partitions in the shared Boolean arena; final
publication completes regions in the target's original support. Intermediate
cofactors must remain raw: completing them against the original support would
change a partial sum. Zero defaults and exact cancellations are retained.

Weighted image follows target readouts in the target's working order. Its input
function is first masked by original source support. It sums source coordinates
unused by any readout, splits on the next readout, then sums coordinates whose
last remaining readout use has just ended. Low/high results become target cells;
canonical scalar states are memoized. Source and target may each have 62 bits
without a combined 124-bit arena or explicit world enumeration.

**Each source coordinate is summed exactly once.** Boolean existential
abstraction is idempotent; scalar summation is not. Summing a skipped outcome
bit doubles a constant density, and summing it again would invent another
factor of two. A copied readout retains its source coordinate until every copy
has been processed, preserving their correlation. Support masking also excludes
completion aliases when a result is used in another weighted image.

Equal-density partitions can grow exponentially. This is an exact initial
arithmetic representation, not a claim that every joint function is compact.
`FunctionLimits` bounds input/intermediate cells (65,536), logical steps
(1,000,000), and memo entries (100,000). Graph limits and a shared arithmetic
budget apply separately. Conservative limits can refuse a small final answer.
There is no aggregate allocator/retained-byte quota; standard scalar clones and
arithmetic-library allocations are not individually fallible. No partial
function, approximate answer or partially closed source is published on refusal.

## Conditional laws

A kernel captures a checked map `parent: Extension → Parent` and a nonnegative
finite function `K` on Extension. Admission requires:

```text
for every legal parent world p:
    sum_{e: parent(e)=p} K(e) = 1.
```

This includes zero-prior parent worlds. Merely checking that closure under one
prior sums to one would let a malformed zero-prior row escape validation.
Normalization also proves nonempty fibres; the constructor separately retains a
checked surjectivity certificate. Unused categorical encodings belong outside
explicit structural support. Zero-probability valid options stay inside it.

`kernel.close(prior, ...)` forms the joint density:

```text
joint(e) = prior(parent(e)) * K(e).
```

The result owns a new measured space and the map back to its prior. Every old
Event can be pulled back through `SourceExtension::parent()`. The old marginal
is preserved; `parent_surjective()` separately exposes structural coverage,
including zero-mass worlds. Closure explicitly permits a replacement designated
prior law on exactly the same named structural parent. Foreign names, support or
coordinates refuse. There is no automatic inference of a joint channel from
separately supplied marginal model answers.

The caller authors the extension's outcome coordinates and source identity.
Reclosing the same named channel with the same prior describes the same joint
source. A fresh draw needs a distinct authored extension; copying an Event adds
no draw. Successive channels may depend on a shared latent case. A prior over
such a case is explicitly supplied; this does not implement or invent a prior
over a continuous unknown parameter.

`kernel.factors_through(readout)` checks that the entire conditional density is
constant on each supplied observation fibre. For an actor policy, the readout
includes actor-visible parent information **and the candidate new outcome**.
It must not include other players' hidden cards. The constructor checks the FD;
the adequacy of that observation is still an application modeling contract.
This host check does not itself make TypeSafe requests or build adapter receipts.

## Native Coup example and evidence

The exact two-proposition deck marginal after Alice's specified hand has masses
`[36, 19, 19, 4] / 78` for `(Bob has Duke, Cleo has Duke)`. A conditional Tax
channel uses `4/5` with Bob's Duke and `1/5` without. Native closure and observation
give `P(Bob Duke | Tax) = 92/147`, `P(Cleo Duke | Tax) = 5/21`, and evidence
probability `49/130`. Reusing the Tax Event is idempotent. An information-factor
check accepts this channel and detects one that secretly depends on Cleo's hand.

The database consumer saves the prior holdings and extended Tax event, reopens
the store, imports the measured parent map, and runs ordinary query heads:

```rust
use map parent = &import;
(player, taxed: Event(Pullback(holding, parent) & tax), given: Event(tax)) |
    Region(id: player, condition: holding), Observation(condition: tax);
```

Returned owned Events can be measured after the executor, database and original
sources are dropped. This uses existing Event query heads; native Probability
heads remain pending. The complete executable Coup fixtures remain an M8 gate.

[WeightedMaps](../crates/bumbledb-event/semantics/WeightedMaps.lean) adds 18 reports,
four axiom-free: weighted pairing, total mass, fibre factoring, channel prior
preservation and normalization, and the local last-use elimination law. It
retains counterexamples for repeated summation, existential counting and checking
only a closed prior. These proofs use nonnegative common-denominator numerators.
Signed arithmetic, raw graph contraction, coordinate-dependency extraction,
canonicalization and memoization remain tested Rust rather than Lean extraction.

Eight core tests cover all 256 four-world maps across 15 nonempty source
supports (3,840 cases), signed pointwise arithmetic, shared-arena restrictions, 62-bit symbolic
images, copied readouts, fresh/copy distinctions, conditional information,
three-category encodings, channel refusal and resources. A database consumer
checks the Coup query through resident and cursor paths. The
[qualification](event-evidence/native-conditional-source-qualification/check.json)
and [semantic record](event-evidence/native-conditional-source-semantics/check.json)
pin that implemented slice. [BESC v1](event-source-descriptors.md) now adds
checked function/channel/receipt transport. The [SDK](event-sdk.md) now constructs
exact scalars, finite functions and designated laws, performs weighted images,
and returns owned fixed-law observations. Kernel/revision SDK operations,
parameter families and adapters remain open.
[Fixed-law revisions and signed expectations](event-revisions.md) now build on
this layer with separate proof and regression evidence.
