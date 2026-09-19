# A binary relation is an Event with a checked face dependency

The [scoped kernel](SCOPED-PRODUCT.md) can contract Events over arbitrary legal
worlds. Promoting that operation to ordinary relational composition needs two
separate facts: the legal worlds supply the required witnesses, and each input
really is a relation on its declared faces. Full binary support proves only
the first fact. This distinction is a design constraint, not a new public scalar.

## A legal domain does not have to fill its encoding

Let `D(e)` be a legal state domain, allowed to depend on a retained environment
`e`. The homogeneous relation workspace has support:

```text
S(e,x,y,z) = x ∈ D(e) & y ∈ D(e) & z ∈ D(e)
```

Each state can have unused bit codes. The environment can constrain which
states exist. Composition preserves that environment and quantifies only the
shared legal state:

```text
(R ; Q)(e,x,z) = x ∈ D(e) & z ∈ D(e)
                 & exists y ∈ D(e): R(e,x,y) & Q(e,y,z)
```

Associativity, legal-state identity and the residual adjunction hold pointwise
in `e`. There is no premise about probabilities. This admits more than a fixed
Cartesian product of encoded bits, while retaining exactly the ordinary
relation laws on each environment's legal states. It is a sufficient constructor,
not a proof that every possible relation workspace must have this shape.

The new [Lean source](lean/LegalRelations.lean) expresses those laws over arbitrary
types and legal-domain predicates, including empty domains. It also connects
the staged clipped three-face lowering to the lifted binary composition. The
lab's anchored owner still requires nonempty overall support: a theorem covering
empty domains does not change that runtime admission rule.

## Why a scope certificate is insufficient

On the full three-bit cube, take the perfectly valid Event `F(x,y,z) = z`.
Pretend it is an XY relation and run the existing lowering with the XY identity:

```text
(F ; Id)(x,y,z) = exists m: F(x,m,y) & (m=y) = y
```

At `(false,true,false)` the result is true and F is false. Right identity fails.
No representation trick fixes that misuse; the Event was never a binary XY
relation. The current lab's relation fixtures construct genuine XY inputs and
its tested operations preserve that role. The low-level `Algebra` accepts raw Ids; the new `Checked` wrapper admits
inputs by the role equation and binds the resulting handles to its owner.
The raw Id import remains a trusted laboratory boundary, not a production
Event-handle validator.

## Certify the role with the algebra itself

Being a relation on XY means that two legal worlds agreeing on E,X,Y agree on
Event membership. This is the ordinary functional-dependency condition already
used in the proposal. Over the declared legal-domain product it is equivalent
to the scoped existential fixed point:

```text
E = exists_Z(E)
```

Here existential elimination returns the projected predicate lifted into the
same legal support, exactly as the lab's existing scoped operation does. The
equality is supported Event equality. A raw essential mask can still contain Z
because the physical encoding carries `Z ∈ D(e)`; demanding an absent raw Z bit
would incorrectly reject valid relations on non-full legal domains.

The general form needs no product premise at all. For a checked readout `q`,
take all legal worlds sharing q with some member of E. That saturation equals
E exactly when q functionally determines Event membership. Product support is
needed for the stronger composition interpretation, not for this role test.
The Lean source proves both the general dependency equivalence and its
three-face form. Thus the certificate can reuse the observation/dependency
algebra on coupled spaces too.

The laboratory's `Relation` retains the Event root and a private owner brand.
`Goal` additionally certifies dependence on E,Y. Admission checks the fixed
point once; lawful operators preserve it. Cloning the execution owner gives
it a new brand, and foreign handles are rejected before constant shortcuts.
No second denotation or weight field is needed. These are internal capabilities,
not implemented macro syntax. Cross-owner transport must re-establish the role
or carry a checked map proof.

## Exact support admission without enumerating worlds

[Domains](src/legal.rs) declares a legal code set for each retained environment.
The native constructor supports bounds and explicit noncontiguous code lists.
It admits an existing carrier only when both facts hold:

```text
S is contained in each of the three declared face-domain predicates
Count(S) = sum_e |D(e)|³
```

The first condition proves containment in the declared product. The independent
exact count proves that no declared combination is missing. Full's scoped
fixed-point equation and equal face marginals would not supply this evidence.
All counts here are exact integers, with a bounded finite encoding; no measured
probability or approximate estimator participates in admission.

[FiniteAdmission.lean](lean/FiniteAdmission.lean) proves the general finite
containment-plus-cardinality criterion in both directions and supplies a
counterexample to cardinality alone. Its finite enumeration is a proof witness,
not the runtime algorithm. The theorem does not verify Rust's counting kernel,
domain formula compiler, or arithmetic for the declared product population.
Those retain separate differential tests.

Essential and packed carriers construct domain support symbolically in a full
ambient owner, then seal it into the constrained owner. No intermediate public
Id escapes. The fallback finite carriers construct support explicitly up to the
lab's twenty-coordinate cap. Overall empty support is rejected by the anchored
owner; an empty domain at one environment is supported.

## The certificate authorizes a different contraction

Equal-domain face permutations preserve S even when the domain has unused codes
or depends on a retained environment. The input supplies a legal witness, and
the output map makes the intermediate clip redundant with final sealing.
`support_certificates_remove_gates` in [ScopedProduct.lean](lean/ScopedProduct.lean)
proves sufficient conditions without requiring full binary support.

The essential carrier now implements a separate `preserving_product` capability.
It validates the maps and computes exact raw support-map equality. The cache
keys contain the full map, scoped to immutable support in the owning carrier.
There is no unchecked caller-supplied preservation flag. Only the first input
map and output map need preservation certificates: the first operand supplies
the common witness support. All maps remain checked bijections.

When certified, the kernel contracts the operand views directly and seals the
result; the general gated kernel remains a matched control. Certificate cache
storage and any intermediate nodes are charged. A certificate permits deleting
work; it does not promise that the extra checking and cache lookup is faster. The
matched paths also differ in repeated support-map work: the general gated
path recomputes its support renamings, while the preserving path caches map
certificates. A cached-support-renaming control that still executes both gates
is needed to attribute speedups specifically to gate deletion. Until then,
measurements compare the complete two strategies, not that isolated cause.
Coupled supports keep the existing exact scoped operation even when this stronger
relation constructor rejects them.

## Native checks and timing boundary

The [shared checks](results/admission-complete-enum-check.json) and
[slab check](results/admission-complete-slab-check.json) pass all sixteen carrier
configurations. Per carrier, 765 admission cases compare every nonempty
three-bit support against three declared domains. Six legal-domain fixtures,
two layouts, and twelve matrices exercise 1,728 relation pairs, or 3,456 for
each essential carrier with both gate policies. They check role rejection,
foreign-owner rejection, all sixteen Boolean operators, identity, converse,
composition, closure, residual adjunction, May and nonvacuous Must. One fixture
has an empty environment fibre.

Essential64, essential512 and packed512 also pass a symbolic legal-workspace
check with 61 coordinates: two domains of 2²⁰−3 and 2²⁰−5 states, retained as
separate environments. Identity, successor closure, converse and exact output
counts are checked without enumerating the world cube. This checks constrained
symbolic computation; it does not yet check publication through the production
cross-owner Event registry.

[The native program](src/legal_bench.rs) runs actual Free Join over typed inputs,
groups relation unions and constraint intersections, and computes closure,
converse, residuals, May and Must: eighty complete outputs per query. Every
fresh and warm result is checked pointwise against independent matrices outside
timing. Fresh execution includes first role admission; warm execution reuses
role caches. Support construction, input import and workspace admission have
separate timings. Result checksums are timed, exact comparisons are not. The
join plan and COLT indexes are prepared once outside these phase timings.
Resident byte estimates cover the Event owner and its caches, not COLT storage
or allocator metadata; process RSS includes fixtures and oracle bitmaps too.

The fixture varies full, bounded, noncontiguous and environment-dependent
domains, with sixteen or sixty-four state codes. This is a word-column lab
adapter over the real executor, not production Event persistence or automatic
planner placement. It measures a lawful relation workload, unlike the separate
arbitrary-support staged-contraction fixture.

## Native measurement: certificates help, with an attribution limit

The smoke and wider screen pass 31 and 42 native processes. They cover both
essential cutoffs and stores, full/bounded/noncontiguous/environment-dependent
domains, and packed/dense controls. The wider screen fixes bit-major order and
the output-late schedule; the smoke compares all three schedules in both orders
on bounded and environment-dependent domains. This is a staged matrix, not a
complete factorial experiment.

The [nine-sample environment repeat](results/legal-focused-repeat.json) passes
ten more processes. At nineteen coordinates, bit-major order, outer memo on,
and legal domains of 61 and 42 states:

| Carrier / execution | Fresh median ms | Fresh min–max ms | Retained KB |
| --- | ---: | ---: | ---: |
| essential512 slab / materialized | 45.111 | 43.960–49.833 | 11,527.3 |
| essential512 slab / fused, gated | 35.749 | 34.846–39.046 | 8,816.5 |
| essential512 slab / fused, certified | 28.062 | 24.421–74.336 | 6,867.1 |
| essential512 slab / output-late, gated | 42.902 | 42.237–45.175 | 9,404.4 |
| essential512 slab / output-late, certified | 31.989 | 31.230–42.024 | 9,404.4 |
| packed512 / materialized | 8.463 | 7.974–8.836 | 13,322.2 |
| dense / materialized | 6.424 | 6.256–6.489 | 45,018.8 |

The complete certificate path reduces median time in both schedules and saves
retained intermediates in the fused schedule. The outlying 74.336 ms certified
sample remains visible. The finite controls still lead in fresh latency; the
symbolic representation buys substantially lower retained storage in this case.
Warm replay is not the selection criterion: the timed output checksum includes
cardinality readout, whose cost differs between carriers even when operator
memos have eliminated most computation.

The first two-sample output-late gated median was 129.945 ms, from 45.461 and
214.429 ms. The nine-sample repeat does not reproduce that large apparent gain.
The wider one-sample screen also gave 42.414 ms for that case. Keep all original
samples in [the smoke](results/legal-smoke.json) and
[screen](results/legal-initial-sweep.json); do not promote their noisy medians
into an algorithmic claim. [Desktop activity](results/admission-environment.json)
is uncontrolled, and these are serial jobs rather than a machine-isolated study.

The [nine-sample face-major repeat](results/legal-face-repeat.json) adds ten
passing processes on the bounded 61-state domain. Materialization takes
30.608 ms (29.780–31.865); fused gated/certified take 22.226/21.824 ms; output-late
gated/certified take 21.005/20.193 ms. Retained storage is 6,611.9 KB for
materialization, about 4,288.8 KB fused, and 3,558.1 KB output-late. Dense and
packed controls take 2.729 and 6.821 ms. The gated/certified fused timing ranges
overlap here; the large environment-case gain is not universal.

The smoke's face-major materialized median of 134.375 ms (172.511, 96.240) and
certified fused median of 74.911 ms (35.415, 114.407) also fail to reproduce.
Keep them as evidence of variation, not discarded samples or proof of a layout
win. The comparison selects the latest supplied run per exact configuration.

[The combined comparison](LEGAL-MEASUREMENTS.md) retains 240 configurations and
632 matched pairs across 93 passing legal-program processes, including their
semantic verification jobs. Missing factorial baselines are listed explicitly;
no ratio is reported for an unrun pairing. The wider screen has one sample per
case and is a screening result, not a stable latency estimate.

The separate [native acceptance](results/admission-acceptance.json) adds thirteen
passing processes, 34 owned-result cases and fifteen full-space symbolic programs,
including million-state closure. Together this executable has **106 passing
native processes** in this cycle. These regressions do not turn the word-column
legal benchmark into production Event persistence.

The byte figures include estimated owner/cache storage, not temporary alignment
buffers or allocator overhead. They are from one executable and checked source
revision; the earlier scoped fixture returned sixteen different outputs and
must not be used as its performance baseline.

The [factorwise retraction candidate](FIBRE-RETRACTION.md) records a deeper
alternative exposed by these role checks: a different fixed completion outside
support may physically remove scratch-face dependence. It is unimplemented and
has explicit counting/projection counterexamples; it is not part of these results.

## Next capability experiment

The current path establishes preservation by exact raw support comparison and
caches it. There is another lawful constructor: once the equal-domain product
is verified, a face permutation retaining the environment is preserving by
construction. A private owner-bound map capability could retain that fact and
pass it to contraction without repeating vector validation or map lookup.
This would propagate a proved fact, rather than accept a caller's Boolean flag.
It is a design hypothesis, not a measured implementation.

The output-late schedule already supplies identity maps for the first operand
and contracted output. Those certificates are trivial on every support, and
the current implementation returns them without storing map-cache entries.
The general path still rebuilds identity-renamed support, so its next control
needs an identity fast path as well as cached nonidentity support renaming.

Keep three controls distinct in that experiment: the present general path,
cached support renaming with both gates still executed, and certified gate
elision with the same caching and validation policy. Include non-preserving
maps on coupled support so an optimization cannot silently weaken the general
operation. Production planning still needs arbitrary readouts, cross-owner
transport and role re-establishment; homogeneous face products are one admitted
capability, not the definition of every Event space.

## Grounding and verification boundary

Desharnais, Möller and Struth's [Kleene Algebra with Domain](https://arxiv.org/abs/cs/0310054),
Example 2.7, uses actual binary relations over a state set with diagonal identity.
It does not assign those laws to every ternary predicate in an ambient cube.
Abbadini–Guffanti [arXiv:2607.06386v1](https://arxiv.org/abs/2607.06386v1),
Definition 2.10, Remark 2.11 and Example 2.12, separates the available image
operations from the square conditions that justify base change. The checked
domain/role boundary here is our concrete application of those distinctions.

The [finite set checker](legal_relations_reference.py) independently compares
staged coordinate movement with matrix composition, associativity, residual
adjunction, and the role fixed point for all legal subsets of four presentation
codes. Relations and Events are exhaustive for domains of at most two values;
larger domains use fixed sampled inputs. Lean and this checker do not formally
verify the Rust lowering or establish a performance ranking.

The preceding [65-report Lean revision](results/legal-relations-lean-check.json)
includes ten reports in `LegalRelations.lean` and the support-gate theorem.
The [current 67-report revision](results/admission-lean-check.json) adds finite
support admission and its cardinality-only counterexample.
The relation laws, role fixed-point equivalences and gate-removal theorem use
no axioms; the concrete Boolean role counterexample reports `propext` and
`Quot.sound`. There is no `sorry`, custom axiom or `native_decide`.

The [finite record](results/legal-relations-reference.json) passes 2,193 relation
pairs, 25,249 associativity cases, 25,249 residual-adjunction cases and 2,825
product-domain role checks over sixteen legal-domain subsets. A further 19,683
checks cover every supported predicate on every three-bit support under three
readouts. It retains the explicit `F=z` counterexample and the copied support
that passes every Full fixed-point test despite failing the product condition.
