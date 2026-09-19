# Candidate: represent an Event through a fixed retraction onto legal worlds

**Two decoder policies implemented and checked. Twenty-four decoder-related Lean
reports pass; the central suite has 109 reports. The prefix revision passes 80
native processes, after 85 retained processes for the earlier revisions.
The [count specialization](FACTOR-COUNTS.md) adds 110 passing native processes
and confirms the 512-cell cutoff on the tested larger queries.
The [information-readout workload](PREFIX-READOUTS.md) separately tests partial
observations, exposing suffix gains and expensive general normalization paths.
Complete-query gains remain layout-dependent; no universal replacement is selected.** The checked [legal-domain interface](LEGAL-RELATIONS.md) exposes a more
substantial representation choice than removing redundant gates. This note
records the candidate and the counterexamples its implementation must respect.

## One fixed decoder, one canonical Boolean function

Let B be the ambient bit cube, S its nonempty set of legal worlds, and choose a
fixed decoder `rho : B -> S` that fixes every legal world. Equivalently, viewing
its output in B, it is a retraction: its image is S and it is identity on S.
Represent the supported Event A by the ordinary Boolean function

```text
encode(A)(b) = A(rho(b))
```

This is a canonical completion outside support once rho is fixed. It is not a
search for the smallest don't-care completion. A canonical raw Boolean carrier
can intern this completed function. Exact equality on S implies equal completed
functions; the converse follows by evaluating at the legal worlds fixed by rho.
Boolean operations commute with the encoding, including complement. Applying
the completion twice is idempotent. No probability premise is involved.

Every physical assignment decodes to a legal world, and every legal world is
represented. Consequently raw satisfiability and all fifteen Venn signatures
reflect the supported answers. A returned raw witness must be decoded through
rho before being reported as an actual world.

These statements are direct consequences of the inverse-image framework in
[ReadoutMaps.lean](lean/ReadoutMaps.lean) and the [map review](../../research/space-maps.md).
The dedicated [seventeen Lean reports](lean/Retraction.lean) now prove these
denotational laws and three counterexamples. The independent
[finite set checker](results/retraction-reference.json) passes 54,080 Boolean
comparisons, 2,000 compositions, 858 partial projections and 780 face maps.
These are separate from correspondence with the Rust implementation.

## The current anchor is already one such completion

For anchor a in S, the present representation's selected raw function is

```text
representative xor polarity = if S(b) then A(b) else A(a)
```

This is the pullback of A under the decoder that sends every invalid world to
the single anchor. The new idea is a different fixed decoder, not the discovery
that relative complement needs an ambient completion.

On `S(e,x,y,z) = D(e,x) & D(e,y) & D(e,z)`, decode the environment first, then
repair each face separately into its declared domain. Every legal value is fixed;
an unused code can map to that domain's declared anchor. If an environment has
an empty domain, decode it to a declared nonempty environment first. The owner
still requires nonempty total support. The repair policy is immutable owner
metadata, never inferred from whichever Event is currently being normalized.

For a binary relation R, its completed function becomes

```text
R(decoded_environment, decoded_x, decoded_y)
```

It does not read z at all. Under the current global-anchor completion, an invalid
z can reset the entire world to the anchor and change R's truth, so raw z bits
can remain essential even when the supported relation ignores z. The new
completion may remove that representation cost by construction. This does not
restore a unique least logical dependency set on arbitrary coupled support.

For example, let each face use two bits but allow only codes 0,1,2, with 3
decoding to 0. The completed relation `x <= y` is `repair(x) <= repair(y)`;
scratch z contributes no nodes to that function. Distinct supported relations
remain distinguishable because every legal x,y pair is fixed by the decoder.

## Whole-face relation operations may become ordinary raw operations

Equal-domain face permutations retaining the environment commute with this
decoder. Whole-face existential elimination has every legal witness and adds
only aliases of legal witnesses. Thus composition, converse, residuals and
whole-face modalities are candidates for direct raw Boolean/relational kernels
over the completed functions, with no support gates or subsequent normalization.

This needs an explicit proof that these particular substitutions and quantifiers
preserve the normal form. Use the existing complete-fibre/base-change criterion;
do not extend the optimization merely because a decoder is surjective. A legal
product constructor can supply stronger capabilities than an arbitrary support.

The mathematical value stays Event. A relation role still means a membership FD,
and this encoding is still required to represent every Event on its admitted
space, including predicates depending jointly on all faces.

## Dependency-language consequence

The normal form itself is an FD:

```text
rho(world) -> Event-membership(world)
```

[Lean proves](lean/Retraction.lean) that a raw predicate is unchanged by completion
if and only if its membership is constant on each decoder fibre. Every fibre
contains exactly one legal world. Completed Events are therefore the subsets of
physical codes that respect this equivalence, with ordinary complement and
canonical raw equality. On relations, the decoder kernel is their identity.

The normal form is aligned with a further useful native promise. For each retained legal
context, the membership FD that says a complete face does not affect the Event
holds exactly when varying that face's raw decoded input cannot affect the
completed function. [Lean checks both directions](lean/Retraction.lean). Thus a
canonical raw essential-coordinate carrier can physically omit the scratch face
for exactly the admitted binary relations on this product workspace.

The existing checked role still asks whether scoped abstraction fixes the Event.
That semantic test remains valid across all carriers. In this completion, the
whole-face abstraction has a direct raw kernel and an unused face has no raw
coordinates to eliminate. This aligns dependency admission, representation and
composition without adding a schema weight or a probability premise.

This claim concerns complete faces under the certified legal product. It does
not turn arbitrary partial bit masks into semantic fields, and does not create
a least dependency set on coupled support.

## Counterexamples prevent tempting incorrect implementations

**Raw population is not legal population.** For legal codes `{0,1,2}` with
`3 -> 0`, Event `{0}` completes to `{0,3}`. It has raw density 1/2 but legal
uniform probability 1/3. Logical aliasing grants no probability independence or
measure-preservation theorem. Exact counting can evaluate the completed function
against original support; law contraction must retain the actual joint law.
Charge that work and its caches in the native comparison. Do not silently reuse
the present essential carrier's unweighted raw count formula.

**Partial-bit projection is not whole-face projection.** With the same decoder,
hide the low bit of `{0}` while retaining the high bit. At legal code 2 (`10`),
the supported existential answer is false: its only legal low-bit alternative
is 2. Raw existential elimination from `{00,11}` incorrectly finds witness `11`,
which decodes to 0 and changes the supposedly retained high bit. An exact general
fallback is needed, or a checked complete-fibre capability for that projection.

Similarly, arbitrary bit permutations can move a legal world outside S. Applying
the completed predicate there would reinterpret an invalid world as an alias.
Preserve the original clipped map semantics through support checks and completion;
only certified commuting/preserving maps get the direct path. Eliminating an
environment whose domains differ can also alter retained state values through
repair and cannot inherit the whole-state-face rule.

**Relation identity compares decoded values.** Physical codes 0 and 3 can name
one state. The completed diagonal is `rho(x) = rho(y)`, which is an equivalence
relation on codes. Raw code equality is not the relation unit. The ordinary
symbolic diagonal constructor is correct when `variable` reads decoded bits.

**Preservation is weaker than commutation.** On legal codes `{1,2}`, send invalid
codes to 1. Swapping the two bits preserves the legal domain, but decoding the
swapped code 0 yields 1, while swapping its decoded value yields 2. The direct
substitution path therefore checks the decoder equation itself, not only S.

### Some commuting fast paths are impossible for every decoder

This limitation cannot always be repaired by choosing a better anchor. On the
legal codes `{01,10}`, swapping bits fixes the illegal code `00` and fixes no
legal code. If a decoder commuted with this swap, the decoded value of `00`
would have to be a legal fixed point, a contradiction. Both the general
fixed-point obstruction and this concrete domain are
[proved in Lean](lean/Retraction.lean), without axioms.

The consequence is a precise design boundary. A single fixed completion cannot
make every support-preserving coordinate substitution an ordinary raw renaming.
Checked commuting maps and exact scoped fallbacks are intrinsic to this design.
Equal-domain whole-face permutations retain their positive guarantee. Changing
presentation/decoder across owners can create a different commuting square, but
such transport needs its own evidence and equality/publication costs; it is not
an automatic same-owner shortcut.

## Implemented carrier and observation

[retraction.rs](src/retraction.rs) stores the same raw essential-coordinate arena
as the anchored carrier, an immutable support root, one raw function per decoded
coordinate, a normalization memo and a checked map-commutation cache. A published
Event is a completed raw root; its low bit is ordinary complement. The public
mathematical type and the checked relation/goal roles are unchanged.

The minimum-repair constructor decodes an empty environment to the first nonempty one,
then repairs each face independently to its domain minimum. Construction for an
arbitrary support uses the global-anchor decoder and remains fully supported.
Normalization performs simultaneous Shannon substitution through the decoder.
Inserted decoder expressions are never recursively substituted into themselves.

Whole-face elimination and commuting face maps use the existing raw kernels.
All other operations preserve the exact staged scoped semantics using support
clipping and completion. The existing `preserving_product` capability retains
its support-preservation contract; when that is weaker than the new decoder
requirements it executes the exact fallback rather than the direct kernel.

[contraction.rs](src/contraction.rs) borrows the support and Event together to
count or construct a shared-parameter spectrum. It does not intern their
conjunction. Missing coordinates are smoothed explicitly. The retained initial version
uses scalar enumeration inside bounded local cubes. A new controlled path uses
borrowed/aligned local word planes and population count; both paths stay in the
executable and include their costs in the query timings. The polynomial spectrum
path still uses scalar local enumeration. Possibility and Venn classification can omit the support root,
because every physical assignment decodes to a legal world. Counting cannot.
Structural packets carry original support alongside completed roots, so transfer
does not publish aliases as new worlds.

The first [shared check](results/retraction-first-check.json) passes, including
40,128 arbitrary scoped products per new carrier, legal-role admission and
61-coordinate constrained relation programs. Dedicated checks exhaust every
projection mask for selected arbitrary three-face Events and assert that XY
relations physically omit the scratch face. They also check exact count/spectrum
against enumeration without any resident arena growth. Extended [transfer and slab checks](results/retraction-complete-slab-check.json)
and [enum/output-late checks](results/retraction-complete-enum-check.json) pass.
There are 16,384 dedicated projection cases for each of three local cutoffs,
plus cross-carrier packets and nine 40-coordinate symbolic transfer pairs.

## Completed experimental obligations

1. Seventeen retraction reports establish identity, Boolean structure, FD
   characterizations, whole-face quantification, composition and the required
   counterexamples. The underlying raw arena and Rust correspondence are not
   formally verified by these denotational proofs.
2. Independent finite sets cover holes, varying domains and empty environment
   fibres. Rust checks every projection mask for selected arbitrary three-face
   Events and tests exact general scoped operations against enumeration.
3. Both 64-cell and 512-cell carriers retain all shared operators, original-law
   observation, decoded diagonals, structural transport and owner publication.
4. Two native executables retain the scalar baseline and a controlled word
   observer. The current comparison covers 136 configurations and 216 matched
   completion/control pairs. Its 58 processes pass; eight further native
   acceptance processes check 24 owned-result cases and 12 symbolic programs.
5. The current binary retains six ARM64 extracts, including the scalar/word
   joint-count kernels and their real vector reductions. Construction, retained
   state, query computation and exact output observation are measured separately.


## Controlled native outcome

The [current comparison](RETRACTION-MEASUREMENTS.md) and raw records retain all
samples, including the initial outliers. The table below uses **five-sample
medians**, width six and outer memo enabled. Symbolic carriers use 512-cell slab
tables and certified fused products; dense and packed retain their own exact
materialized algorithms. Fresh time includes all eighty output counts.

| Legal domain / order | Carrier | Fresh ms | Compute ms | Observe ms | Retained KB |
| --- | --- | ---: | ---: | ---: | ---: |
| below / bit-major | retraction512 | 5.654 | 3.768 | 1.823 | 1718.4 |
| below / bit-major | essential512 | 6.616 | 6.615 | 0.000 | 1188.8 |
| below / bit-major | packed512 | 2.280 | 2.106 | 0.171 | 2478.1 |
| below / bit-major | dense-dispatched | 2.804 | 2.763 | 0.041 | 7431.7 |
| fibred / bit-major | retraction512 | 28.727 | 19.850 | 8.854 | 9957.0 |
| fibred / bit-major | essential512 | 25.666 | 25.666 | 0.001 | 6867.1 |
| fibred / bit-major | packed512 | 8.414 | 7.468 | 0.881 | 13322.2 |
| fibred / bit-major | dense-dispatched | 6.476 | 6.390 | 0.087 | 45018.8 |
| below / face-major | retraction512 | 9.508 | 9.409 | 0.103 | 3103.5 |
| below / face-major | essential512 | 21.941 | 21.941 | 0.001 | 4288.8 |
| below / face-major | packed512 | 6.917 | 6.872 | 0.046 | 6870.2 |
| below / face-major | dense-dispatched | 2.815 | 2.775 | 0.040 | 7431.7 |
| fibred / face-major | retraction512 | 22.549 | 22.276 | 0.274 | 9126.5 |
| fibred / face-major | essential512 | 63.080 | 63.078 | 0.001 | 12052.8 |
| fibred / face-major | packed512 | 26.224 | 26.106 | 0.118 | 25974.3 |
| fibred / face-major | dense-dispatched | 6.681 | 6.584 | 0.105 | 45018.8 |

Phase medians need not sum to the median of total time. On bit-major fibred
inputs, scalar observation has a 180.018 ms median versus 8.854 ms for words;
the complete query falls from 200.371 to 28.727 ms. The roots and retained
storage are unchanged. Word counting makes the representation competitive;
it does not erase the anchored carrier's constant-time stored-count advantage.

The completed-root representation changes which order works well. In the
face-major fibred case it takes 22.549 ms (range 21.796–23.302), versus anchored
63.080 ms (61.676–69.945), while retaining 9,126.5 versus 12,052.8 KB. It also
beats packed in that order: 26.224 ms and 25,974.3 KB. Across orders, however,
packed's bit-major 8.414 ms remains much faster; dense is also faster. This is
a substantive improvement to the symbolic design, not a universal winner.

The bit-major bounded case favors the decoder, 5.654 versus 6.616 ms, but retains
more storage. The face-major bounded decoder has a 12.170 ms sample; it is not
dropped. The smaller 64-cell cutoff was screened in face-major order and loses
to the 512-cell decoder in those single-sample cases. That is not a universal
cutoff theorem.

The [actual ARM64 extract](results/retraction512-count-words-retraction-words-arm64.s)
contains vector masks, `CNT.16B` and `UDOT` reductions. These implement local
joint population counts after alignment. The shared-parameter spectrum still
uses scalar local enumeration, so this experiment does **not** establish a
comparable speedup for arbitrary probabilistic inference.

## Where this applies to Coup

For a game/environment e, let `D(e)` be the legal complete game states. A
transition Event reads a before-state X and an after-state Y; composition uses a
third complete state Z as scratch. Cards within one state can obey arbitrary
deck constraints. They do not need to be independent. What is certified here is
the workspace of three copies of that legal state domain, retaining e.

An XY transition satisfying the membership FD `e, X, Y -> membership` can then
omit Z physically. Composition quantifies a whole middle state, identity compares
decoded states, and residuals find which continuations preserve a target relation.
This is the direct fit with the existing checked relation language.

It does not follow that a predicate about one player's card physically omits all
other card bits inside that same state. The deck couples those fields, and its
support need not admit an independent per-card decoder. That finer decomposition
requires its own capability, or uses the exact general carrier. The domain
benchmarks use bounds and holes, not a newly implemented complete Coup engine.
Their exact populations are correctness checksums, not a uniform-game prior.

## A remaining representation choice: which fixed decoder?

The implemented product decoder sends an unused code to the domain minimum.
That is a reproducible policy, not an optimality theorem. A legal interval could
instead clamp unused codes to its maximum; a domain with holes could repair to
a nearby legal code. Any fixed per-face retraction preserves the same proofs,
provided all equal-domain faces use the same policy and maps check commutation.
It may change function size, substitution cost and order sensitivity considerably.

The prefix comparison below keeps the algebra and raw arena fixed while varying
this repair policy. Other choices remain possible: for an ordered state
relation, clamping may preserve structure that resetting to zero destroys. The original support and law must remain unchanged. Do not choose
a different arbitrary completion separately for each Event: the current equality
and operation contract depends on one immutable owner decoder.

## Initial native result: observation cannot be ignored

The [initial sweep](RETRACTION-INITIAL.md) passes all nineteen processes, with
64 configurations and 72 matched completion/control comparisons. It is a
one-sample screening run, not a stable performance ranking. Every one of the
eighty results per query matches the legal-domain matrix oracle.

On the environment-dependent width-six case with outer memo, materialized
retraction takes 202.385 ms fresh and 180.744 ms warm; the fused form takes
285.617 ms fresh and 181.959 ms warm. The anchored output-late control takes
32.417 ms fresh and 0.035 ms warm. These results reject promotion of the scalar
observation implementation. They do not isolate its cause: warm elapsed time
alone does not separately measure counting.

The controlled executable now measures query computation and exact output
observation separately and retains scalar/word counting over identical roots.
Its matched cases confirm identical resident memory and node counts. Counting
is always included; it is not a completed probability-law performance result.

### Prefix candidate: preserve extendible observations

A repair policy can target an algebraic property rather than only table size.
Choose an order within a state face. Walk its input bits in that order; retain
each bit if the repaired prefix still has a legal completion, and flip it
otherwise. Since the incoming repaired prefix is extendible, at least one of
the two bit choices remains extendible. Every legal input is fixed.

This constructs a triangular decoder: a decoded prefix depends only on the raw
prefix. More strongly, completing all raw suffixes behind a fixed raw prefix
produce exactly the legal suffixes behind its decoded prefix. That is
the complete-fibre condition needed for direct suffix projection. On the domain
`{01,11}`, a suitable prefix decoder preserves the high bit and forces the low
bit to one; resetting every invalid code to `01` unnecessarily makes the decoded
high bit depend on the raw low bit.

The independent [finite reference](results/prefix-retraction-reference.json)
exhausts 273 nonempty supports through three coordinates, 6,648 Events, 26,496
suffix projections and 8,352 fibre images. All pass. It retains a counterexample
for a non-suffix projection on `{01,10}`: this does not remove the obstruction to
making every readout cheap at once. The general complete-fibre theorem is already
in Lean. Seven additional [constructive Lean reports](lean/PrefixRetraction.lean)
now prove the prefix algorithm itself, including the fibre-coverage law and
exact suffix projection. Both [slab](results/prefix-first-slab-check.json) and
[enum](results/prefix-first-enum-check.json) differential checks pass. The native
comparison and its focused repeat are retained below.

The competing owner builds this decoder symbolically from suffix-existential
views of each legal domain. Its construction cost, normalized relation roots
and partial-projection capabilities can now be compared with minimum repair.
Original support, law, Event equality and fallback semantics stay the same.
The proved readout/dependence alignment is not a universally least dependency
set across all coordinate subsets.

### Symbolic construction and the physical experiment

The new internal candidates are `prefix64` and `prefix512`. They use exactly
`Retraction<K, true>` with the same raw arena, complement bit, original support,
count/spectrum contraction, and map-commutation check as the minimum decoder.
For a given legal state domain, process semantic bits high to low, regardless of
physical BDD order. At bit b, quantify the lower bits of the residual domain to
compute whether the incoming bit still admits a completion. The decoded bit is
XNOR(raw bit, feasibility). Substitute that chosen bit simultaneously into the
residual and continue. The final residual must be true. The legal constructor
also checks its symbolic support population against the independent domain
population formula for both decoder policies in this revision.

A direct projection now accepts a low-bit suffix independently on each of the
three state faces, while retaining the environment. Whole-face abstraction is
a special case. An environment projection, arbitrary mask, arbitrary support
owner or noncommuting map keeps the exact existing fallback. Empty environments
continue to decode to the first nonempty environment; every face then uses that
environment's fixed decoder. There is no probability or sampling operation here.

For example, on legal state codes `{01,11}`, completion of the high-bit predicate
is literally the raw high bit. Completion of the low-bit predicate is true. The
minimum decoder instead makes the first predicate depend on both raw bits. This
is a change in which dependencies the physical representation carries, before
any instruction tuning. It can simplify relation imports and transformations,
while also certifying a wider family of direct projections.

The current native relation workload hides whole state faces for both policies.
Any speed difference there tests the changed completed functions and their
construction, rather than a new partial-projection shortcut. Dedicated finite
checks cover the larger projection contract separately. The planned [information-readout workload](PREFIX-READOUTS.md)
will be needed to attribute a performance benefit to that new contract itself.
A fixed semantic prefix is a choice of readout hierarchy; it is not a claim that
all useful observations can be put first simultaneously.

The slab check compares 261,120 decoded worlds at each of two local cutoffs
against the independent finite algorithm, plus 16,384 projections and 16,384
relational products per cutoff at three cutoffs. The shared suite also exercises
61-coordinate legal relations, count/law observation, supported identity,
signatures, cross-owner transfer, admission and the dependency-summary bridge.
These are correctness checks, not timings. The [primary-source rereading](results/prefix-reading.json)
retains the distinction between Boolean completion, exact base change and
operation complexity. Neither paper supplies this decoder algorithm.

### Exact membership dependencies along the readout hierarchy

The seventh [prefix theorem](lean/PrefixRetraction.lean) sharpens the database
connection. Fix a retained prefix. On legal states, suppose its value determines
Event membership. Then the completed raw function ignores every suffix bit.
Conversely, if the raw function ignores the suffix, the legal membership FD
holds. This is an equivalence, not merely a sufficient shortcut:

```text
legal prefix -> Event membership
    iff
completed Event depends only on the raw prefix
```

Thus the chosen hierarchy has no spurious prefix dependence from off-support
completion. An exact raw carrier can use that physical fact for terminal cuts
and dependency tests. This does not prove the Rust interner canonical, and it
does not make every incomparable logical FD physically visible. On `{00,11}`,
either bit determines the other. High-to-low repair chooses to carry the high
bit; it cannot simultaneously represent a nonconstant Event with no bits.
That is the existing coupled-support obstruction in a concrete form.

The independent [FD checker](results/prefix-dependency-reference.json) tests
26,496 logical/physical equivalences across all 273 nonempty supports through
three coordinates. It also checks a useful characterization: the greedy
repair selects the legal state minimizing the unsigned value `raw XOR legal`.
Earlier bit agreement takes priority over every combination of later bits, so
each greedy choice minimizes that rank. This is not numeric clamping or minimum
Hamming distance. The XOR characterization is a finite cross-check plus this
inductive argument; it is not an additional reported Lean theorem.

### Prefix structure and symmetry need not improve together

On legal codes `{00,01,10}`, minimum repair decodes `11` to `00` and commutes
with swapping the two bits. High-to-low prefix repair decodes `11` to `10` and
does not commute: the swap fixes `11` but sends its decoded value to `01`.
Both policies still implement the same exact semantic permutation; the second
requires normalization instead of an ordinary raw rename.

This conflict is stronger than a poor tie-breaking choice. Any decoder that
fixes legal states and preserves high-prefix dependence must send `11` to `10`:
it shares its high prefix with the fixed legal state `10`, and that is the only
legal state with that high prefix. A commuting decoder must instead send the
swap's fixed raw code `11` to a swap-fixed legal state. The only such state here
is `00`. Thus those two physical promises cannot coexist on this domain.

The new projection certificate is not a dominance theorem over every map.
Whole equal-domain face swaps remain certified for both policies. The arbitrary
map fallback is a necessary part of preserving the complete Event algebra.
The dedicated [Rust map check](prefix_maps_check.py) passes with both
[slab](results/prefix-maps-slab-check.json) and
[enum](results/prefix-maps-enum-check.json) storage. For each store, it tests
5,760 cases per carrier over all 720 six-coordinate permutations, at three
prefix cutoffs and one minimum-repair control. It checks a finite pointwise
oracle and canonical reimport equality; legal exports alone could hide an
unnormalized root. The decoder's actual evaluated functions also confirm the
bit-swap commutation difference between policies.

## Prefix native result: a better completion, with a remaining observation cost

The [generated tables](PREFIX-MEASUREMENTS.md) retain 160 configurations and 128
matched prefix/control comparisons from 72 serial processes. The independent
[native acceptance run](results/prefix-acceptance.json) adds eight processes,
24 owned-result cases and 12 symbolic programs. All pass. This revision uses
executable `84ffdedfc491a5b326dd10f3e14c69ff091e0ace5b966ba00183e74135625047`;
its comparison does not mix timings from either earlier retraction executable.

Five-sample fresh medians below include exact original-support counts of all
80 query outputs. These are width-six cases with outer memo, 512-cell slab
carriers and certified fused products; packed/dense retain their native
materialized controls. Construction is separate.

| Domain / order | Prefix ms | Minimum ms | Anchored Essential ms | Packed ms | Dense ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| below / bit-major | 3.966 | 5.710 | 7.095 | 2.248 | 2.911 |
| below / face-major | 8.172 | 9.008 | 22.002 | 6.725 | 2.789 |
| holes / bit-major | 7.162 | 20.759 | 17.332 | 6.738 | 3.285 |
| holes / face-major | 7.516 | 12.800 | 40.064 | 15.956 | 3.093 |
| fibred / bit-major | 11.850 | 29.087 | 24.936 | 9.321 | 6.601 |
| fibred / face-major | 16.150 | 22.184 | 61.384 | 25.384 | 6.479 |

On fibred bit-major, prefix's query computation is 5.044 ms and observation is
6.794 ms; minimum repair spends 20.365 and 8.757 ms respectively. Packed spends
8.404 and 0.894 ms. The changed completed functions improve construction of the
Event results enough to lead that computation phase, but original-support
observation leaves packed ahead on the complete query. Counts remain included.
The prefix fresh range is 11.747–11.891 ms; minimum's is 28.571–29.198 ms.

Prefix retains 6,476.1 KB there, versus minimum's 9,957.0 KB, packed's 13,322.2 KB
and dense's 45,018.8 KB. On fibred face-major it retains 7,087.0 KB versus
minimum's 9,126.5 KB. These estimates include construction residue and resident
caches; temporary observation memo and allocator overhead are separate.

Construction can reverse the apparent tradeoff for short-lived contexts.
Fibred bit-major build medians are 26.381 ms for prefix, 34.163 ms for minimum,
60.418 ms for packed and 475.637 ms for dense. These describe the current
constructor algorithms, not an inherent lower bound for any representation.
The one-sample full-support controls have identical prefix/minimum resident
counts and bytes: both decoders are identity there. No claimed projection
improvement was measured by those full-support controls.

All raw samples remain visible, including the anchored below/face-major repeat's
56.122 ms outlier. The collector chooses the latest supplied process for each
exact configuration, never the minimum across revisions or runs. Five samples
are not a representative workload distribution or a machine-wide confidence
interval. `prefix64`, alternative product schedules and the planned partial
information-readout lane still need matched native measurement before promotion.

The [ARM64 record](results/assembly-prefix.json) contains a shared normalization
routine, separate product entries for the policies and the shared word count
kernel. Its word contraction includes vector Boolean operations, `CNT.16B` and
`UDOT` reduction. The gain above changed completed functions and their graph
shape; it did not introduce a new SIMD kernel. The prefix construction has made
an algebraic choice pay off while leaving the remaining costs measurable.
