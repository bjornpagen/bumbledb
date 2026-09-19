# Machine-checked laws for the representation

The Event laboratory now includes Lean proofs using the already installed
Lean **4.32.0** and its standard library. There is no Mathlib dependency,
`sorry`, custom axiom, or `native_decide` escape hatch in these proofs.
The local `lean-toolchain` pins the installed version without changing the
user's global default.

## Signatures: exactly what they retain and what they lose

[EventSignatures.lean](lean/EventSignatures.lean) defines an Event denotation as
a Boolean predicate over an arbitrary world type. Occupancy explicitly requires
the support predicate, so unsupported assignments cannot become witnesses.
The proofs establish:

- Possibility of every binary Boolean expression is determined by occupied cells.
- Fullness is equivalent to the operation accepting every occupied cell.
- Nonempty support always occupies some cell.
- Complementing either operand and swapping operands transform occupancy exactly.
- Two pairs have the same signature **iff** every binary Boolean possibility
  test gives the same answer. This proves the signature's exact information
  boundary, rather than merely demonstrating a useful list of predicates.
- On the same three-world support, a singleton and a doubleton have identical
  self-pair signatures, but only the doubleton admits the requested nonempty
  proper intermediate Event. Strong composition cannot depend only on those
  fifteen signatures. A further theorem states this directly: no function of
  the signature, even a noncomputable one, decides that composition exactly.

The general theorems quantify over arbitrary regions and worlds. The final
counterexample is concrete and finite. These are stronger claims than checking
a large number of sampled Rust operations, but they address different obligations.

## Anchored storage: why the complement bit is legitimate

[Anchored.lean](lean/Anchored.lean) proves the invariant used by the
[anchored carrier](ANCHOR.md) and the packed/essential adapters:

```text
polarity        = A(anchor)
representative  = S & (A XOR polarity)
decode(r,p)     = S & (r XOR p)
```

It proves that the representative excludes the anchor and everything outside
support, decoding recovers A on support, relative complement preserves the
representative and flips polarity, and equality of representative/polarity data
is equivalent to equality on support. The complement and equality theorems
explicitly require an admissible anchor.

For every binary truth function f, the adjusted raw operation is

```text
h(x,y) = f(x XOR p, y XOR q) XOR f(p,q)
```

Lean proves `h(false,false)=false` and that applying h to the two representatives
produces exactly the normalized result representative. This is the reason one
raw Boolean Apply can preserve the invariant without an extra support mask.
The proof covers every binary function, including implication and constants.

## Essential coordinates: the unique least dependency set

[EssentialCoordinates.lean](lean/EssentialCoordinates.lean) proves the mathematical
basis for local tables over exactly the coordinates a predicate uses. For a
finite binary product, it establishes:

- Changing a coordinate can affect the predicate iff its two cofactors differ.
- Agreement on every essential coordinate forces equal predicate values.
- Every sufficient coordinate set contains all essential coordinates. Together,
  these results make the essential set the unique least sufficient set.
- Fixing omitted coordinates to false gives exactly the same predicate value
  when the retained set contains all essential coordinates.
- Global Boolean complement preserves the essential set.
- Raw existential abstraction makes the quantified coordinate inessential.

Finiteness is explicit: the theorem takes a list covering the coordinate type.
It does not assert that arbitrary predicates on infinite products depend on
only finitely many coordinates. The raw abstraction theorem also does not say
that a scoped Event will omit that coordinate after intersecting its result with
coupled admissible support. That support can reintroduce dependence.

Two further reports prove that restriction can only remove essential axes,
and can remove axes other than the one fixed. For `if x then y else z`, fixing
x=true leaves exactly y. This justifies testing exact canonical cofactors against
conservative pending-pin masks in the [normalization experiment](REUSE.md).

These nine central reports prove properties of represented functions. They do
not prove that Rust's mask normalization computes that set, that the branch/table
cut is canonical, or that the packed records implement it correctly. The forced
collision, growth and cross-layout ID checks retain those separate implementation
obligations. No theorem removes an omitted random draw from its joint law.

## Dependencies on admitted worlds: a different invariant

[SupportedDependence.lean](lean/SupportedDependence.lean) supplies four reports.
A sufficient coordinate set is defined by the usual functional-dependency
condition: two admitted worlds agreeing there have the same function value.
If support is closed under taking one world's selected coordinates and another's
remaining coordinates, sufficient sets can be intersected. Products of legal
coordinate domains have this closure, without any probability premise.

For support `x=y`, the Event `x=true` is determined by either x alone or y alone,
but is not constant. Lean proves that no least sufficient coordinate set exists
in this example. This rules out promoting the raw essential-coordinate theorem
to arbitrary supported Events. The existing carrier stays sound: its least mask
belongs to its canonical raw Boolean function, with supported Event identity
handled above it. [The design boundary](SUPPORTED-DEPENDENCE.md) explains the
consequence for legal categorical domains and future map capabilities.

## Maps: exactly when the same readout survives

[ReadoutMaps.lean](lean/ReadoutMaps.lean) separates three promises a map can make.
For `f : X → Y`, with admitted supports `SX` and `SY`, Lean proves:

- Every possibility query survives inverse image **iff** `f` maps admitted X
  worlds into SY and covers every admitted Y world. In other words, its image
  of SX must be exactly SY.
- Those two conditions preserve every occupied Venn cell and reflect equality
  on support. An unrestricted classifier can therefore be reused under such a
  lift, with the correct operand correspondence.
- Support inclusion alone preserves relative complement. It need not reflect
  possibility or equality: a restriction may remove their distinguishing worlds.
- Fixing one binary coordinate supplies a concrete counterexample to unrestricted
  possibility reuse.

The necessity theorem quantifies over **all** Boolean predicates, including
singletons. It applies directly to the finite carriers; it does not provide a
decision procedure for an arbitrary restricted guard language. The theorem has
no finiteness premise, but constructing or checking its hypotheses is a separate
implementation obligation.

This is a derived support-image law, consistent with the inverse-image account
in the [primary-source map review](../../research/space-maps.md). It does not
authorize moving quantifiers across arbitrary maps: a particular commuting
square needs its own complete-fibre condition. Nor does it preserve a designated
probability law without a pushforward-law proof. These five reports make that
boundary precise; they do not verify Rust cofactor, broadcast or memo kernels.

## Quantifier rewrites: complete fibres, without requiring unique lifts

[BaseChange.lean](lean/BaseChange.lean) formalizes the next map obligation.
For maps `u:W→X`, `v:W→Y`, `f:X→Z`, `g:Y→Z`, assume the square commutes:
`f(u(w)) = g(v(w))`. The types here contain admitted worlds only. For every
predicate A and target context y, compare:

```text
exists w: A(u(w)) and v(w)=y
exists x: A(x)    and f(x)=g(y)
```

The first implies the second automatically. Lean proves equality for **all**
predicates iff every compatible `(x,y)` has a lift w with `u(w)=x`, `v(w)=y`.
The criterion requires existence, not uniqueness. A set-theoretic pullback is
a sufficient constructor, but a checked square with multiple lifts of a pair
can also support the rewrite. A concrete two-world example proves that this
weaker criterion does not force unique lifts.

The same hypotheses preserve universal elimination and its nonempty-fibre
guard, covering the nonvacuous condition used by Must. A copied-bit square
provides a machine-checked failure: both projections are surjective, yet an
off-diagonal compatible pair has no lift. Keeping each old world somewhere
is weaker than keeping it in every compatible retained context.

The certificate itself has an ordinary relation-algebra expression. For the
graphs U, V, F and G of the four maps, Lean proves:

```text
Converse(U);V = F;Converse(G)
    iff the square commutes and has complete fibres
```

The left side relates endpoint pairs that have a joint lift; the right relates
pairs compatible at the shared base. If commutation is already known, only the
reverse containment needs proof. Its obstruction is an Event-valued gap.
An optimizer may retain a certificate, but the certificate's meaning remains
expressible and inspectable in the same algebra as the query.

These seven reports generalize the existing finite checker. Necessity again
quantifies over all Boolean predicates and uses singletons. They follow the
powerset inverse/direct-image account in the
[primary-source map review](../../research/space-maps.md); the complete-fibre
iff is our derived criterion, not a throughput claim from that paper. They
neither verify a Rust rewrite nor preserve a designated law. Multiple witnesses
are irrelevant to existential truth but can matter to mass and counting.

## Borrowed products: retain the joint image of the views

[ViewProduct.lean](lean/ViewProduct.lean) supplies eight reports for the
[mapped-product experiment](VIEW-PRODUCT.md). W contains admitted joint worlds;
u and v read the two operands and h retains the output context. Lean proves

```text
exists w: h(w)=t and A(u(w)) and B(v(w))
iff
exists x,y: J(t,x,y) and A(x) and B(y)

where J(t,x,y) = exists w: h(w)=t and u(w)=x and v(w)=y
```

This holds without injectivity or surjectivity premises. A replacement relation
C preserves the result for **every** Boolean operand pair iff C is exactly J.
The necessity proof uses singleton predicates. Thus a direct kernel can avoid
constructing both renamed operands, but it must retain their joint compatibility
and the output context. J is a semantic relation; it need not be a stored table.

The full-product specialization proves ordinary relation composition: both
views share the same middle witness. Two counterexamples show that separate
surjectivity cannot replace joint compatibility, and that the same input
predicates under different maps can give full versus empty. Roots alone are
therefore insufficient for a general product memo key. A proven common
substitution can justify a narrower key; arbitrary maps cannot be discarded.

The witness conversion and composition proofs use no axioms. The exactness iff
uses standard classical singleton construction; the counterexamples use
`propext`. These are elementary denotational results, consistent with the
retained powerset/map framework. They do not establish a Rust implementation,
normal-form canonicality, law preservation or a performance bound.

Three additional reports separate legal rewrites from physical schedules.
`output_bijection` allows output renaming after contraction, with explicit inverse
laws. `joint_reindex_with_section` allows witness reindexing whenever the new
presentation supplies a lift for every original witness; duplicate witnesses
are harmless for existential truth. `shannon_merge` establishes the Boolean
identity used at each meaningful bit of the new local word reader. It does not
prove the packing, padding, coordinate alignment or Rust borrow implementation.

## Scoped fusion: preserve the support gates

[ScopedProduct.lean](lean/ScopedProduct.lean) supplies five reports for the
[constrained-product extension](SCOPED-PRODUCT.md). The staged operations clip
both mapped operands, the projected result and the mapped output to admitted
support. Lean proves that the common witness gate can instead be absorbed into
one operand through its inverse map, while the post-projection gate is retained
in output coordinates. The theorem allows arbitrary admitted support. The full-
support specialization recovers the previous expression.

Two singleton-support counterexamples prove why either gate can be necessary.
Omitting the witness gate admits an outside-support witness; omitting the output
gate can resurrect a world after renaming. Both counterexamples use no axioms.
The general theorem uses standard `Quot.sound`, and its full-support reduction
also uses `propext`. The result proves equality with the staged scoped expression;
it does not make clipped maps preserve full/complement, admit coupled supports
to ordinary relation composition, or prove a probability-law identity.

The fifth report proves sufficient map conditions for eliminating both extra
gates: the first input read map reflects support, and the output read map
preserves it. This theorem uses no axioms and does not require the full binary
cube. The new checked legal-domain path uses an exact support-map certificate to
remove these gates; the [runtime comparison](LEGAL-RELATIONS.md) keeps the
original gated path as a control. The proof does not verify either Rust kernel.

## Legal relation roles: composition laws meet functional dependencies

[LegalRelations.lean](lean/LegalRelations.lean) supplies ten reports. On legal
state domains `D(e)` indexed by a retained environment, it proves composition
associativity, left/right identity for supported relations, residual adjunction,
and the staged three-face lowering. No probability premise is involved.

The role certificate has an exact algebraic test: forgetting the unused face
must leave the Event unchanged. The more general theorem needs no product
support: saturation under a readout fixes an Event exactly when that readout
functionally determines Event membership. This identifies the certificate with
the existing dependency semantics, rather than a physical raw-mask restriction.

Two boundaries are explicit. Full support does not make `F(x,y,z)=z` into an XY
relation: composing it with identity gives y. Conversely, scoped Full passes
every readout fixed-point test regardless of support shape, so that test cannot
certify a product workspace. Domain construction and input roles need separate
evidence. The nine general reports use no axioms; the concrete Boolean role
counterexample uses `propext` and `Quot.sound`.

[The design review](LEGAL-RELATIONS.md) and independent finite checker retain
the runtime implications. Native role admission and environment-indexed legal
products now have separate Rust differential and symbolic checks. These
denotational proofs do not certify that implementation.

## Finite support admission

[FiniteAdmission.lean](lean/FiniteAdmission.lean) adds two reports: finite
support equality is equivalent to containment plus equal cardinality, and
cardinality alone is insufficient. The common finite enumeration is a proof
witness; runtime support can be constructed, compared and counted symbolically.
The general proof uses `propext` and `Quot.sound`; the counterexample uses no
axioms. It does not verify the Rust count kernel or the domain population formula.
[The checked relation interface](LEGAL-RELATIONS.md) applies this criterion.

## Fixed-decoder completion

[Retraction.lean](lean/Retraction.lean) adds seventeen reports. A decoder lands
inside support and fixes every supported world. Pullback along it reflects
supported equality and possibility, commutes with every Boolean operation, and
is idempotent. The normal form is characterized exactly by the membership FD:
two codes with the same decoded world have the same Event membership. For a legal state face, raw existential and universal quantifiers
over decoded values range over precisely the legal values. Relation composition
is preserved, and its right identity compares decoded states rather than raw
codes. A commuting substitution preserves the completed function directly.
A further theorem identifies independence of a complete decoded face with the
FD saying legal Event membership ignores that face, for each retained context.

Two more reports prove an obstruction independent of repair policy: if a map
fixes some physical code and no legal world, no decoder can commute with it.
The legal codes `{01,10}` and bit swapping instantiate this theorem. It prevents
mistaking a support-preservation certificate for a universally attainable
normal-form certificate.

Three kernel-checked counterexamples separate this representation from tempting
incorrect implementations: aliases change raw assignment counts; projection of
only part of a state face can change a retained bit; and unequal raw codes may
name the same legal state. Fifteen reports use no axioms; idempotence and
commuting substitution use only `propext`. See the
[representation experiment](FIBRE-RETRACTION.md) for the Rust obligations.

## Prefix-preserving construction

[PrefixRetraction.lean](lean/PrefixRetraction.lean) adds seven reports for a
constructive recursive decoder. It keeps the incoming bit if that branch has a
legal completion, flips otherwise, and recurses in the chosen legal subtree.
The proofs establish the finite feasibility test, landing in support, fixing
legal worlds, prefix dependence, and exact coverage of every legal suffix
behind the decoded prefix. `prefix_exists` derives direct raw suffix abstraction
from those construction laws, rather than assuming a fibre certificate.
`prefix_dependency_reflected` then establishes that a supported membership FD
from the legal prefix is equivalent to raw prefix dependence of the completed
Event. It does not claim a least dependency set across incomparable coordinate
subsets.

The prefix-dependence theorem uses no axioms. The other reports use only
`propext` and, in some cases, `Quot.sound`. They do not use `Classical.choice`,
`native_decide`, admitted theorems or custom axioms. The independent exhaustive
[finite reference](results/prefix-retraction-reference.json) remains separate
from both this recursive proof and the Rust symbolic decoder. Non-prefix
projection still requires the exact supported fallback; the retained finite
counterexample rules out extending this certificate to arbitrary masks.

## Cardinality through omitted legal product factors

[FaceCounting.lean](lean/FaceCounting.lean) adds four reports. Counting a predicate
that ignores a product factor multiplies its retained count by that factor's
cardinality. The indexed theorem keeps each environment's domain size separate.
A third theorem validates dividing out a nonempty raw factor's cardinality and
inserting the legal factor's cardinality. A coupled-support counterexample
shows why membership independence alone is insufficient without product support.

Lists witness finite factors. Duplicate-free lists give set cardinality; the
same identities count multiplicity for arbitrary lists. Three reports use only
`propext` and `Quot.sound`; the counterexample uses no axioms. The proof does not
identify arbitrary source laws with counting measure and does not verify Rust
integer widths or the preparation of its partial-support roots.

## Information readouts and exact dependency conditions

[InformationReadout.lean](lean/InformationReadout.lean) adds fourteen reports
over arbitrary observation functions on admitted-world types. Possible is the
least observable superset; Guaranteed is the greatest observable subset. Both
return Events whose membership is determined by the observation. Their fixed
points are exactly those membership FDs.

The uniform filter rewrite `Possible_f(A & B) = Possible_f(A) & B` holds for
every A **iff** f determines B; the dual Guaranteed/union rule has the same
condition. The order on Possible operators is exactly the FD between their
observations, and nested readouts absorb to the coarser observation. Grouped
union distributes through Possible; grouped intersection distributes through
Guaranteed. Explicit counterexamples reject distributing across the wrong
connective. [The query explanation](INFORMATION-ALGEBRA.md) separates these
general laws from the prefix decoder's physical specialization.

## Direct conditionals and simultaneous substitution

[DirectConditional.lean](lean/DirectConditional.lean) adds six reports for the
current normalization kernel: semantic preservation of recursive simultaneous
substitution, composition and identity of substitutions, exact selector/output
polarity rules, common Shannon splitting, and failure of sequential replacement.
All six reports use no axioms. This proves the pure term semantics and local
Boolean identities, not Rust arena canonicality or memo correctness.

## Removing an unused support factor during projection

[ProjectionGate.lean](lean/ProjectionGate.lean) adds six axiom-free reports.
With an Event whose membership ignores an omitted face, eliminating that face's
support gate for every Event is equivalent to joint witness coverage. Product
support supplies the witness from the admitted target, including partial
readouts. Equality on admitted outputs becomes equality of completed codes.
The observed-coordinate extension permits omitting a face whose hidden
coordinates cannot affect membership, including a fully visible face.
Counterexamples show why coupled support and hidden environments require the
stronger coverage certificate. This is the contract of the current projection-gate prototype, separate from
direct ternary ITE. See [the design](PROJECTION-GATES.md).

## Reproduce and retain the trust boundary

```sh
python3 proposal/experiments/event-repr-lab/verify_lean.py
```

The verifier requires the pinned toolchain to be installed. It retains compiler
output, version, executable/source hashes and `#print axioms` for 174 central
theorems. It rejects failed checks, missing theorem reports, and any axiom other
than Lean's standard `propext`, `Quot.sound`, or `Classical.choice`. The general
signature possibility/fullness/nonempty/swap proofs use no axioms at all; the
map occupancy and equality proofs also use no axioms. Other reported proofs use
`propext`, `Quot.sound`, and, in some cases, `Classical.choice`.
Run proof checking outside performance sweeps.

The [retained check record](results/lean-check.json) and
[compiler output](results/lean-check.log) record this successful proof revision.
The previous [seventeen-report](results/lean-v1/lean-check.json) and
[twenty-four-report](results/lean-v2/lean-check.json),
[twenty-nine-report](results/lean-v3/lean-check.json), and
[thirty-six-report](results/lean-v4/lean-check.json), and
[forty-one-report](results/lean-v5/lean-check.json),
[forty-four-report](results/lean-v6/lean-check.json), and
[forty-six-report](results/lean-v7/lean-check.json), and
[fifty-report](results/lean-v8/lean-check.json), and
[fifty-four-report](results/lean-v9/lean-check.json), and
[sixty-five-report](results/lean-v10/lean-check.json), and
[sixty-seven-report](results/lean-v11/lean-check.json), and
[eighty-four-report](results/lean-v12/lean-check.json), and
[ninety-report](results/lean-v13/lean-check.json), and
[ninety-one-report](results/lean-v14/lean-check.json), and
[ninety-five-report](results/lean-v15/lean-check.json), and
[109-report](results/lean-v16/lean-check.json), and
[115-report](results/lean-v17/lean-check.json), and
[120-report](results/lean-v18/lean-check.json) revisions retain their
verifiers, compiler logs and exact proof sources.

The subsequent [121-report](results/lean-v19/lean-check.json),
[129-report](results/lean-v20/lean-check.json), and
[137-report](results/lean-v21/lean-check.json) revisions are also preserved
with their original verifier and proof sources.

These are denotational proofs, **not a formal verification of Rust**. They do not
yet prove that the raw arena's normal form is canonical, that packed IDs implement
the proved operations, or that an ARM64 kernel refines its Rust specification.
The signature composition-envelope table, its associativity and the cap-at-two
refinement currently have the derivations and exhaustive checker in
[the relationship-calculus experiment](SIGNATURE-CALCULUS.md); they are not among
the Lean theorems. Actual execution, lifetime and scope checks retain their
separate Rust and native acceptance tests.

## Local completion survives untouched faces

[LocalCompletion.lean](lean/LocalCompletion.lean) contributes eight checked
reports, all axiom-free. A completed predicate is separately invariant under
each independent idempotent face decoder, even if it couples the faces.
Existential and universal quantification preserve that invariance when their
relation/gate introduces no constraint on the untouched face. Completing only
the affected face gives exactly the full completed predicate on every raw code.
Two indexed theorems handle repaired environment codes and decoder families
selected by the resolved environment. Retaining an untouched-domain gate or
using a coupled decoder has explicit counterexamples. This proof justifies the
[local-completion control](LOCAL-COMPLETION.md); it is not a proof of Rust arena
canonicality or a performance result. That addition brought the central suite to 129 reports.

## Direct completed projection keeps source and target roles separate

[FusedProjection.lean](lean/FusedProjection.lean) adds eight axiom-free reports.
Hidden source coordinates combine their transformed cofactors by existential
union; observed source coordinates select a cofactor with the target decoder.
Graph substitution and absent-variable elimination establish the base cases.
Contraction preserves union, while explicit counterexamples reject complement
memo folding, independent witnesses for conjuncts, and re-abstracting target
coordinates introduced by a decoder. These are denotational step laws; source
cofactor implementation, recursive arena canonicality and native performance
remain separate checks. That addition brought the central suite to 137 reports.

## Storage closure and the planner's nonempty witness

[EventStorage.lean](lean/EventStorage.lean) added ten axiom-free reports, bringing
that revision of the central suite to **147**. The model separates whole-fact identity
from a fact's world incidence. Adding an empty-valued fact preserves both
coverage and conflict, but the fact still exists in ordinary relational queries.
Complete region equality identifies one fact under a pointwise key only when
the region has a witness; two distinct empty-valued facts are a counterexample.

Inverse image and existential projection preserve empty. Universal preimage of
empty characterizes dead ends. An empty pointwise source needs no target
witness, so it cannot discharge an ordinary scalar IND. Dropping empty rows
before complement loses a full-valued answer. Finally, inhabited owner support
keeps full and empty distinct. These results justify the
[storage contract](../../event-storage.md), without claiming that native Event
persistence or the planner already implements it.

## Fixed difference bases and the counting boundary

[Differential.lean](lean/Differential.lean) adds eleven reports: decomposition,
unique coefficients, complement, XOR/product coefficients, root quantifiers,
the obstruction to coefficient-wise deeper projection, overlap-sensitive
population, and arbitrary-width full-tree roundtrip/injectivity under any fixed
mixed basis schedule. Nine are axiom-free; the two tree proofs use `propext`.
Shared-node canonicality, Rust interning, and node-adaptive basis selection are
outside these theorems.

[ConstraintCounting.lean](lean/ConstraintCounting.lean) adds seven reports:
character orthogonality, arbitrary constraint gap, zero count from gap, doubled
constraint zero count, and AND/NOT/output constraint equations. The three gate
reports are axiom-free; four general finite-sum reports use `propext` and
`Quot.sound`. The complete circuit compiler, complexity classes and polynomial
size bound are not formalized. The [research argument](../../research/compact-counting.md)
states those boundaries and the inspected sources explicitly.

That revision of the central suite had **165 reports, 106 axiom-free**. Previous
[147-report](results/lean-v22/lean-check.json) and
[158-report](results/lean-v23/lean-check.json) suites remain frozen with their
original proof sources and verifier. The raw Rust counter's finite tests and
the [structural counting screen](COUNTING.md) supply separate implementation
evidence. A small positive-Davio graph does not inherit Shannon's disjoint
cofactor counting fold.

## Inline unary-chain laws

[UnaryChains.lean](lean/UnaryChains.lean) adds nine reports: exact cofactor
reification, evaluation and associativity of composition of the four unary
Boolean maps, identity laws, exact compilation of arbitrary finite label words,
concatenation, complement population, and each label's uniform-count action.
Six reports are axiom-free; the remaining three use `propext` and/or `Quot.sound`.

That revision had **174 reports, 112 axiom-free**. The previous
[165-report suite](results/lean-v24/lean-check.json) is frozen with its sources
and verifier. These new laws support the [inline-chain experiment](UNARY-CHAINS.md);
they do not verify the packed-bit encoding, arena uniqueness, collection, or
general correlated-law contraction.

## Block projection and empty fibres

[UnaryProjection.lean](lean/UnaryProjection.lean) adds eleven reports for the
image action on the four possible Boolean value sets, its composition and
occupancy laws, nonempty-fibre bound steps and an empty-fibre counterexample.
Seven reports are axiom-free; four use only `propext`. That revision
had **185 reports, 119 axiom-free**. The
[174-report suite](results/lean-v25/lean-check.json) remains frozen.
These are semantic proofs; the [block experiment](UNARY-BLOCKS.md) separately
checks Rust encoding, collection and multi-coordinate projection.

## Sparse shared-exit selectors

[SharedExit.lean](lean/SharedExit.lean) adds ten reports: shared-selector Apply,
splitting, complement, normalization, sparse-list splitting, hidden and visible
projection, essential selector witnesses, arbitrary-width raw-cube count, and a
counterexample when selectors differ. Seven are axiom-free; three use `propext`
and/or `Quot.sound`. The central suite now has **195 reports, 126 axiom-free**.
The [185-report suite](results/lean-v26/lean-check.json) is frozen.
These laws support [the shared-exit experiment](RANGES.md), while Rust segment
selection, sparse arena canonicality and native admission remain distinct
verification obligations.

## Separate query-factorization proofs

The central suite remains at 195 reports. Eighteen further axiom-free reports
are checked separately with the same pinned Lean executable:

- [FactorizedPack.lean](join-factorization/FactorizedPack.lean): six reports for
  the exact rectangularity criterion, the clover rewrite, group presence and
  counterexamples. [Result](results/factorized-pack-lean.json).
- [ParticipatingValidation.lean](join-factorization/ParticipatingValidation.lean):
  two reports for validation of participating rows and exact unordered fault
  sets. [Result](results/participating-validation-lean.json).
- [RelationPack.lean](join-factorization/RelationPack.lean): four reports for
  ordered composition, both residual aggregation directions and the necessity
  of one shared intermediate witness. [Result](results/relation-pack-lean.json).
- [QuerySeparator.lean](join-factorization/QuerySeparator.lean): six reports
  construct compatible branch assignments from a scalar separator and prove
  residual aggregation and a shared-environment counterexample.
  [Result](results/query-separator-lean.json).

The initial QuerySeparator proof checked successfully with `propext` in four
reports. Its [frozen source and manifest](results/query-separator-initial/manifest.json)
remain available; explicit Boolean case analysis removes those dependencies in
the final proof. These denotational laws do not verify the Rust IR checker,
arena implementation or machine code. The native Boolean and relational Pack
experiments retain their independent differential tests and measured costs.
