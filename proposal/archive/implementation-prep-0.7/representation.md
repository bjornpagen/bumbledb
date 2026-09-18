# Resident representation and canonical identity

**Revision 0.7: exact Event semantics, competing physical carriers.** The
[temporary Rust laboratory](experiments/event-repr-lab/REPORT.md) compares
canonical sets, runs, Roaring and decision diagrams through actual Free Join.
Production has no Event implementation yet. The current experiments strengthen
the completed-function candidate without selecting a universal backend.

## Start with the value and its owner

An Event denotes a subset of an explicit legal-world space. Its proposed
sixteen-byte resident key names its owner and canonical region; complement flips
one polarity bit. The owner retains support, coordinate meaning and any
designated [joint source law](source-law.md). A law measures Events; it does not
decide which worlds exist. Zero probability therefore does not mean impossibility.

The representation must preserve exact equality, complements, shared witnesses,
coordinate changes, relational composition, and observation. These obligations
apply to every candidate. A compact signature is a useful readout of two Events,
not a replacement for the Events needed to construct future results.

## The leading structural idea: choose legal meaning for every raw code

Choose an immutable decoder `rho` from physical codes onto legal worlds that
fixes legal worlds. Store the canonical Boolean function:

```text
completed_A(code) = A(rho(code))
```

[Lean proves](experiments/event-repr-lab/FIBRE-RETRACTION.md) exact identity,
complement and Boolean operation preservation. The normal form itself is the
FD `decoded world -> Event membership`. Raw possibility is legal possibility;
a raw witness is decoded before it is reported. Raw code counts are **not**
probabilities or legal cardinalities: observation still uses original support
and the designated law.

For a certified product of state copies, repair each face independently within
the resolved environment. A binary relation then physically omits its scratch
state exactly when its legal membership FD says that scratch state is irrelevant.
A whole state can contain all Coup card correlations. Product support between
copies asserts neither independent players nor probabilistic independence.
A prefix-preserving decoder further reflects a chosen hierarchy of partial
observations. Arbitrary readouts and maps retain exact general fallbacks.

## Concrete memory candidate

The completed function can use [essential-coordinate tables](experiments/event-repr-lab/ESSENTIAL.md):
constants, a compact table when the exact dependence mask fits the chosen cutoff,
otherwise an ordered split with canonical children. The cut depends on the
function, preserving uniqueness. The [slab store](experiments/event-repr-lab/SLABS.md)
uses sixteen-byte raw records, a single word slab and collision-checked interning.
Six- and nine-coordinate cutoffs compete against fixed-tail packed tables.
Complemented raw references, memo tables and owner metadata are all charged.

The exact raw mask is useful physical information. It is not a universally least
logical dependency set: on legal support `x=y`, either x or y determines the
same Event. [The supported-dependence proof](experiments/event-repr-lab/SUPPORTED-DEPENDENCE.md)
shows why arbitrary coupled support needs explicit certificates and cannot be
optimized using a fictitious least semantic mask.

## Dependencies choose which work exists

| Operation | Certificate | Consequence |
| --- | --- | --- |
| Whole-face elimination | Independent face decoders and complete legal fibres | Direct raw quantification preserves completion |
| Partial readout in the chosen hierarchy | Prefix-preserving repair | Certified suffix abstraction needs no general repair |
| Omit a projection gate | Membership stability plus joint witness coverage | An observed legal target supplies the unused witness |
| Preserve an untouched completed face | Independent indexed decoders, retained environment, and no remaining gate on that face | Repair only affected faces and the environment |
| Contract an unused face during counting | Membership FD and known original legal fibre population | Replace raw multiplicity with exact legal multiplicity |
| Operate directly through a coordinate map | Decoder commutation or a separately proved base-change contract | Avoid repair only for certified maps |

The [gate round](experiments/event-repr-lab/PROJECTION-GATES.md) makes the third
row concrete. With ITE fixed, its repeated difficult query falls from 392.599 to
10.758 ms and from 708,405 to 35,380 retained records. Packed still leads at
1.467 ms. All policies share the same input storage and 343-record output graph.
The [local-completion round](experiments/event-repr-lab/LOCAL-COMPLETION.md)
now passes 110 comparative processes and reduces that query from 10.659 to
2.247 ms with fixed witness gates and ITE; packed takes 1.587 ms. Eight new Lean
reports include repaired environments. These are distinct from probability
independence, and each has an exact fallback.

## Lower certificates to operation data

The laboratory's four completion switches isolate hypotheses. They are not new
schema choices or production configuration requirements. A production readout
can prepare its hidden-coordinate set, selected decoder coordinates and exact
support gates from the owner's certificates. In the three-face prototype there
are only eight possible gate selections, indexed by the intersection of Event
dependence and hidden coordinates.

This suggests one internal projection program: constrain by the selected gate,
quantify the hidden coordinates, then perform the selected simultaneous
substitution. Certified direct projection is the same program with no gate and
no substitution. Local completion selects fewer decoder coordinates. Arbitrary
coupled support selects the full gate and decoder. The canonical Event value and
operator contract stay the same in every case. Preparation may amortize over
many Free Join bindings, but that batching is proposed work, not part of the
current timing claims.

## Canonical outputs permit different internal schedules

Stable identity does not require every intermediate to be a fully completed
Event. Borrowed mapped operands, partial substitutions and aligned word planes
can remain internal views. Their cache keys must retain coordinate maps, selected
decoders and the joint witness context. Published roots recover exact canonical
meaning. [The view experiments](experiments/event-repr-lab/VIEW-ORDER.md) show
that maximal fusion can lose even while retaining fewer nodes; schedules need
matched measurements rather than an architectural mandate.

The [direct-projection comparison](experiments/event-repr-lab/FUSED-PROJECTION.md)
now confirms that boundary for completion too: all 81 comparison processes pass,
but staged local completion wins the repeated difficult readout, 2.229 versus
2.711 ms in bit-major order. The two schedules publish identical canonical
answer graphs. Retain staging for that family and preserve fusion as a measured
alternative, not a requirement of the representation.

[Direct ITE](experiments/event-repr-lab/TERNARY-COMPLETION.md) resolves all four
former reimport caps and emits ARM64 vector bit-selects. It is a useful local
kernel, while the much larger gate gain comes from deleting unnecessary work.
Counts of reachable answer records and retained arena capacity are measured
separately; no garbage collector or reclaimable-byte result is implied.

The experiment already includes transport, ownership and novel results in a
retained arena. Production wire identity, transaction lifetimes, concurrency,
and cross-owner certificates remain separate obligations. Never serialize an
arena index as a stable fact identity.

Sections 1–6 below retain the earlier BDD/pair layout as one specified candidate.
A mandatory BDD resident carrier, bitmap-to-BDD conversion, adaptive backend and
production wire format are not settled decisions. The full measurement history
and rejected controls remain in the laboratory report.

## 1. Layout

```rust
#[repr(transparent)]
struct BddRef(u32); // index = bits >> 1; complement = bits & 1

#[repr(C, align(16))]
struct BddNode {    // 16 bytes, four per 64-byte cache line
    variable: u32,
    low: BddRef,
    high: BddRef,
    reserved: u32, // zero; not serialized as logical state
}

#[repr(C)]
struct RegionPair { side0: BddRef, side1: BddRef } // 8 bytes

#[repr(C)]
struct EventKey { space: u64, region: u64 }       // 16 bytes

struct Event {                                  // owned API value
    key: EventKey,
    owner: Arc<EventSpace>,
}
```

These definitions specify the proposed resident layout. `EventKey` has private
fields and cannot be constructed from unchecked words in the public API.
Stored event fields and query values both admit empty and full; the
[storage contract](event-storage.md) distinguishes empty Events from missing
facts and retains nonemptiness as a planner proof premise. A full-space face is synthesized from the aligned scope, not stored as a
second random outcome.

The space owns: its logical descriptor, coordinate order, support root,
designated measurement (`Unmeasured` or a normalized law descriptor), immutable
node/partition slabs, unique tables, and bounded operation
caches. Large rational coefficients, polynomial terms, and source metadata are
arena data, never copied into each relation row. [Layout probe](kernels/layout.rs)
checks the candidate sizes independently of engine types.

## 2. Canonical Boolean core

The BDD candidates use a reduced ordered BDD with complement edges. References 0 and 1 are false
and true. Nonterminal indices begin at one. Every child tests a later variable.
Construction goes through one private `mk(variable, low, high)`:

1. If children are equal, return the child.
2. If high is complemented, complement both children and remember one result flip.
3. Intern the complete `(variable, low, high)` tuple, with high regular.
4. Return its reference with the remembered polarity.

A fixed coordinate order, elimination of redundant tests, and unique nodes make
the representation canonical for Boolean functions. Complemented edges preserve
that result with the stated normalization. Hashes accelerate lookup; complete
key equality resolves collisions. No expression-DAG hash is treated as a proof.

Node indices fit 31 bits: at most `2^31 - 1` nonterminal indices per arena.
Exhaustion is an explicit resource failure before publication. This is a concrete
initial limit, not silent index wrapping or an excuse to change the denotation.
Nodes use indices rather than raw pointers, allowing stable slabs and later
relocation. Allocation order is not the wire order.

The [CUDD internals/source notes](research/representation-notes.md) confirm the
normalization pattern. We are selecting that representation principle, not
proposing CUDD as an unexamined engine dependency.

## 3. The earlier two-root implementation of support-relative identity

Let S be the space's nonempty support BDD. For a raw event formula e:

```text
T = apply(AND, S, e)
F = apply(AND, S, not(e))
pair = (min(T,F), max(T,F))
region = intern(pair) << 1 | index_of(T_in_pair)
```

The two roots cannot be equal: their disjoint union is nonempty S. `RegionPair`
is constructed privately and proves that invariant. Partition zero is `(0,S)`;
its orientations 0 and 1 are empty and full. Every other complement is also
`region ^ 1`, requiring no graph traversal and no new partition allocation.

Within one resident space, equality of region handles is exactly equality of
events on support. Outside-support disagreements canonicalize away. Across
spaces, equal numbers or equal root indices say nothing. A checked alignment
must establish the same named coordinates, law context, and semantics first.

For `apply_event(op,a,b)`, obtain their selected roots, apply the four-bit
Boolean operation, intersect the result with S, and intern its two-way partition.
The reference form performs both masks. An implementation can omit a redundant
mask only after the operation's truth table proves the result already lies in S.
For example, AND of two true roots stays inside S; raw NOR generally does not.

The pair cache is a deliberate cost: eight bytes per complementary pair plus
its two root paths. It buys exact scoped complement by one bit flip. The support-masked-root candidate needs `S & !root` on a complement cache miss.
The anchored alternative below gives exact scoped equality and one-bit complement
with a single stored representative, so this pair cost is not mandatory.

## 4. Coordinates, legal support, and growth

Coordinates identify finite outcomes or logical parameter guards. A coordinate
name contains source allocation identity and draw ordinal, rather than the
current variable index. Categorical outcomes use a stable binary encoding;
invalid codes are outside S. Selecting an outcome is a predicate on those bits,
not a separate independently sampled Bernoulli value.

Ordering is fixed for an arena. Start with source/trajectory order, keep bits of
one outcome together, and preserve declared dependency blocks. This is a default
to evaluate, not a theorem of compactness. Source metadata preserves semantic
coordinate identity separately from that chosen order.

In the earlier BDD wire candidate, the descriptor's wire ordering policy is
deterministic from the named presentation and format version. An execution order
can differ, but serialization must then pay conversion to that wire order.
The laboratory does not require that round trip for resident publication: a
sealed manager's own canonical identity suffices. The wire design remains open
because conversion into a poor fixed order can explode. Whichever persistent
format is adopted must prevent runtime heuristics from assigning different
logical identities to the same named event. The
[transport experiment](experiments/event-repr-lab/TRANSPORT.md) now measures
checked rebasing and a common packed512 wire candidate. It checks common bytes
for a fixed named presentation and ordered batch; per-fact persistent identity
and source/law descriptor serialization remain unimplemented.

A new draw or a new guard creates an explicit extension descriptor and a checked
lifting map from old events. Old event E lifts to `S_new & E(old_coordinates)`.
An ordinary extension must preserve the old admissible-support projection. If it
instead rules out previously possible worlds, it is a restriction/revised source
context and must be recorded as such. Conditioning retains the old scope and
supplies an evidence event; it does not silently mutate support.

The [Lean map proofs](experiments/event-repr-lab/LEAN.md#maps-exactly-when-the-same-readout-survives)
make the readout contract exact: inverse image preserves every possibility query
iff the source support's image is exactly the target support. That same checked
map preserves all four pair-occupancy bits and reflects supported equality.
This permits reusing an exact relationship result under a faithful lift, with
validated operand correspondence. A coordinate restriction may lose witnesses;
its new pair cannot simply inherit the old exact signature. This capability is
separate from the canonical IDs of either owner and from the map's law meaning.

Support totality does not grant arbitrary projection rewrites. Adding Y as a
copy of X retains every old possibility, but hiding X can leave information in
Y. A map's support proof, its complete-fibre proof for a specified projection
square, and its pushforward-law proof are separate reusable capabilities.
[The map review](research/space-maps.md) derives the finite criterion from
primary sources and checks a concrete counterexample exhaustively. Full fibre
products supply the required squares for the relation operators.

Distinct named sources have no automatic independence. A measured extension
supplies a normalized conditional joint law or an admitted coupling family.
An unmeasured structural product supplies only admissibility. A zero mass does
not erase a structural world; a supported restriction is explicit. Copying an
Event allocates no coordinate.

No dynamic reordering of a published arena. An exploratory reorder constructs a
new execution view or arena and an explicit handle translation. Raw handle
comparisons must never span the translation boundary. Committed sources with
identical logical descriptors are rebased to the registry's canonical resident
arena on import/bind; representation-local IDs cannot leak into logical equality.

## 5. Ownership, transactions, and memory pressure

Use immutable append-only slabs and stable indices. A transaction allocates in
private scratch structures, then rebases referenced nodes/pairs into a serialized
canonical publication step. Only admitted rows and their complete owner set become
visible. Concurrent builds can speculate independently; the publish path resolves
interning races by full structural equality. No lock-free allocation claim is made.

Images, prepared executions, derived stages, scratch-stage readers, and owned
result batches retain an `EventOwners` collection. Retain an owner once per
referenced space, not by atomic refcounting every binding copied through Free Join.
A standalone `Event` can retain an `Arc`; borrowed events use a lifetime-bound
owner. All references crossing a spill boundary keep the corresponding owners.

`space` is a generation-checked registry token. Retired slots are not reused while
reachable by any owner, and stale generations reject decoding. Node indices are
never recycled within a live arena. Empty/full are actual interned values, not
sentinel addresses that bypass ownership.

The initial reclamation policy is coarse: release an arena when its final owner
is released. Bound transaction scratch, operation caches, and derived tile caches
through `WorkContext`; check cancellation in traversals. Long-lived arenas may
retain dead intermediate nodes. Track that retention explicitly and add offline
compaction/translation if workloads require it. Cache eviction drops memo entries,
not semantic results or live source laws. Exhaustion aborts the operation rather
than returning an approximate event or a partial answer.

The transport lab now implements the coarse ownership fragment: a sixteen-byte
copied key, a retained scope owner per batch, published-ID resolution and
nonreused process tokens. Restored Free Join rows carry the destination token;
the full relation program returns the same canonical destination IDs. The lab
publishing boundary still trusts carrier-produced IDs. The [owned lane](experiments/event-repr-lab/OWNED.md)
now computes novel results through a mutable borrow of that owner and checks
sole-result-batch lifetime after releasing inputs and executor. Registry generations,
concurrency, multi-owner spill and transaction publication remain production
acceptance work.

## 6. Resident columns versus canonical storage

The native [image layout](../crates/bumbledb/src/image.rs) already decomposes
multiword fields into columns. EventKey uses two **ordinary word columns**,
`space[]` and `region[]`, and two binding slots. Do not assign the interval
`WordPair` meaning just because the width matches. Encoding, ordering, typing,
filtering, spilled rows, and ownership all need actual event support.

Resident equality and local sorting can use the two words. This local ordering
is only an execution order: arena allocation is not deterministic logical row
ordering, and `<` on handles is not event inclusion. Existing canonical row
ordering and diagnostic tie-breaking must use logical event bytes.

The earlier BDD wire candidate stores a versioned logical space descriptor and
reachable canonical BDD structure. This remains an implementation hypothesis: a
finite or differently decomposed manager must pay conversion into this form,
which can be prohibitive under a bad canonical order. Its steps, under the
descriptor's fixed semantic coordinate order, are:

1. Serialize terminals and semantic variable identities.
2. Traverse the reachable support/true/false graph deterministically, low before
   high, assigning fresh postorder IDs and preserving complement polarity.
3. Encode one selected support-relative root and the descriptor dependency;
   the complement root is reconstructible. Serialize the pair only if a later
   wire version justifies the redundancy.
4. On load, verify descriptor, support, acyclicity, order, reduction, normalization,
   references, and resource bounds; intern into the resident registry.

No arena addresses, raw resident indices, cache contents, or input expression
syntax define canonical fact bytes. Equal events in the same normalized named
presentation have the same wire form. Arbitrary semantically equivalent source
reparameterizations are not silently assigned the same descriptor; they require
an explicit proved alignment. The retained process theory's graph-equality
judgment is different from this named-space identity contract.

Content digests locate shared source/graph objects; verified canonical content
settles collisions. Event references and their objects commit atomically. Native
standalone canonical rows need either a self-contained descriptor encoding or a
verified object-store resolver carried by the row/transaction API; a bare hash
with a missing referent is never a valid Event. The implementation plan makes
that integration a storage task, not a two-word type patch.

The lab's `Split`/`Table` packet is a lossless structural transport format, not
the canonical fact encoding specified above. It exposes graph sharing and local
tables across sixteen carriers without requiring the eleven symbolic exporters
to enumerate worlds. Same-carrier allocation-independent bytes, cross-carrier
restoration and common-destination bytes are checked separately. A packet's
ordered root list cannot by itself define a single persisted Event's identity.

## 7. What the representation experiments reopen

| Candidate | Measured or semantic strength | Remaining cost or limit |
| --- | --- | --- |
| Interned dense bitmaps | Exact resident identity and one-bit complement; strong finite Free Join results | Enumerates a materializable world presentation |
| Interned sparse / Roaring sets | Exact identity and compact sparse storage | Density, interning, projection and observation can change the winner |
| Interned interval runs | Compact and fast when the world enumeration gives long contiguous regions | Arbitrary predicates destroy that structure |
| One masked BDD root | Canonical support-relative equality without pair records | First relative complement can require constructing a support mask |
| Interned BDD partition pair | Both true roots immediately available; one-bit complement | Pair interning, extra roots and order-dependent growth |
| Anchored BDD root | Canonical equality and one-bit complement; zero-preserving Boolean Apply needs no support mask | Quantification can require actual-root recovery and normalization |
| Packed-terminal diagram | Exact anchored identity with word/NEON-sized leaf payloads; fewer recursive levels | Coordinate movement across the leaf boundary can require rebuilding; fixed order still matters |
| Selector-block diagram | Canonical partitions of up to six coordinates; masks share equal continuation functions | High continuation diversity and maps that split blocks can make the representation expensive |
| Essential-coordinate tables | Canonical tables over exactly the function's relevant axes, even on noncontiguous coordinates | First native sweep trails fixed-tail controls; normalization, alignment and metadata need matched algorithm controls |
| Fixed-decoder completion | Canonical `A(rho(code))`, with independently decoded state faces or a prefix-preserving decoder; exact FDs describe the normal form | Direct maps/projections require specific proofs; original support remains necessary for law/count observation; native comparison is ongoing |
| Four-way DD | Grouped bits can reduce recursive traversal | Group selection and coordinate order remain decisive |
| Fixed Shannon/Davio schedules | Exact raw identity and complement; positive Davio compresses the bilinear strength fixture | Standalone screen only; Coup construction and exact cofactor counting expose large temporary growth; not yet a native carrier |
| Shannon unary edge chains | A competing sixteen-byte record packs ten letters; collected graphs and uniform counts pass matched checks | Block projection now passes but sometimes retains more nodes; no native speed claim |
| Sparse shared-exit selectors | Sixteen-byte records support grouped Apply, cofactoring and projection; [64 structural processes pass](experiments/event-repr-lab/RANGES.md) | Full scoped/native admission passes; [56 native query configurations](experiments/event-repr-lab/RANGE-NATIVE-MEASUREMENTS.md) do not favor grouped fresh Coup execution over its step control |
| Formula DAG / AIG / SAT | Cheap construction and deferred reasoning | Not measured here; must pay exact equality, coverage, quantification and observation costs before a matched comparison |
| SDD / other decomposed circuits | Can exploit structure unavailable to one variable order | Not implemented in this lab; no performance claim |

The [packed layout and invariant proof](experiments/event-repr-lab/PACKED.md)
records the implemented terminal scheme. The [primary-source review](research/representation-search.md)
qualifies SDD compression, chain reductions, variable shifts and tagged support.
The [selector-block layout](experiments/event-repr-lab/BLOCKS.md) adds compressed
word selectors at every level. Its [research review](research/block-decomposition.md)
keeps this ordered construction distinct from general AND/OR decomposition.
The [layout-planning rule](experiments/event-repr-lab/LAYOUT-PLANS.md) derives
invariant coordinate groups from the permutations required by the algebra.
These shared map plans inform physical decomposition without changing the
mathematical coordinate names or dropping a required operator.

The [essential-coordinate normal form](experiments/event-repr-lab/ESSENTIAL.md)
lets the function itself determine where a table belongs. Branch reconstruction
and bijective renaming provide exact dependence certificates; structural counts
can be propagated at interning. These are algebraic implementation facts, not
probability weights or claims of a universal winner. The first complete native
comparison retains all scalar/word results and favors the existing controls.

See [the measured recommendation](experiments/event-repr-lab/REPORT.md) for workload
winners and uncertainty. A bitmap manager can canonicalize complete finite events;
it need not translate every result through a BDD to establish resident equality.
That does not settle representation-independent persistent bytes or cross-manager
translation. The existing BDD wire sketch is therefore still a candidate to test,
not a free normalization step whose cost can be excluded from a future benchmark.

## 8. Keep the representation usable across layers

The isolated [difference-basis competitor](experiments/event-repr-lab/DIFFERENTIAL.md)
stores a base and XOR difference in the same sixteen-byte two-edge layout.
A fixed per-coordinate basis preserves unique coefficient trees, exact raw
identity and one-bit complement. It greatly reduces grouped bilinear output
graphs while increasing this probe's retained query work; Coup favors Shannon.
Those structural results do not yet admit a native Event carrier.

Observation is a separate requirement. The
[counting review](research/compact-counting.md) constructs sparse cubic functions
whose positive-Davio graphs remain polynomial-sized although exact uniform
counting is #P-hard. Shannon's disjoint cofactors support a graph counting fold;
arbitrary difference coefficients need overlap information. Neither statement
provides cheap arbitrary joint-law contraction. A future carrier must account
for the complete operator and observer costs, including temporary growth and
conversion, while retaining the same owner-aware Event contract.

Expose safe borrowed views of named coordinates, support, roots, and shared
subgraphs, retaining their owner lifetimes. Keep raw constructors private.
The reusable internal operations are canonical Apply/ITE, union folds,
satisfiability/witness extraction, cofactoring, checked substitution/lifting,
and contraction against a supplied law. Admission, queries, diagnostics, and
compiled programs consume those same operations rather than reinterpreting
the field independently.

Revision 0.5 makes [world-relation views and plans](world-relations.md) part of
this contract. `WorldRelation` retains an Event on a product space and owned
input/output FaceRefs. Composition plans retain checked lifting/permutation
maps, the eliminated coordinate set, and a result map. They use fused relational
product rather than expanding the region into world rows. Domain, modalities,
residuals and finite closure all share this interface.

Descriptors carry concrete, inspectable data (names are proposed):

```rust
struct FaceLayout {
    state_space: SpaceRef,
    environment: EnvironmentRef,
    coordinates: OwnedSlice<CoordinateId>,
}
struct ProductLayout {
    space: SpaceRef,
    faces: OwnedSlice<FaceLayoutRef>,
    lifts: OwnedSlice<CoordinateMapRef>,
    full_fiber_product: CheckedProductRef,
}
```

These are shared arena descriptors, not extra words in each EventKey. Their
layout sizes are not yet fixed or probed. The checked product records that its
ambient support contains all endpoint combinations at the same environment;
the move's legality is stored in the relation region. A coordinate map describes
renaming/substitution and its support proof; it is not an arbitrary callback.
State-role aliases are separate from stochastic draw identities. Retained
descriptor owners transitively keep all component spaces/maps alive.

A cofactor view carries its fixed prefix and remaining coordinate roster. It
is a predicate fragment until explicitly lifted or used in a correctly scoped
contraction, not a silently reconditioned Event. Existential abstraction is a
logical operation; probability marginalization is a weighted sum. Keep that
distinction in the types of the internal programs.

A coordinate map also has a denotation in this same relation algebra. Its graph
`G:T→S` is total when `Domain(G)=Full(T)` and functional when
`Converse(G);G <= Id(S)`. A renaming table is a compact executable presentation
of such a graph. Its inverse image is `May(G,E)` and its direct image is
`Post(G,A)`. A checked graph view can therefore give reference semantics to
specialized map kernels, rather than becoming a separate opaque callback API.
The [finite derivation and checks](research/space-maps.md) also derive an
observation's information relation as `G;Converse(G)`. General graph-backed maps
remain proposed; the current Rust transport validates binary embeddings.

Projection-commutation certificates belong to a particular square of maps,
not to the map alone. A full fibre product supplies a sufficient construction;
other constrained squares may also satisfy the exact complete-fibre criterion.
Retain those proofs when available instead of rejecting every constrained map
or granting every rewrite from a generic extension flag. The
[Lean base-change theorem](experiments/event-repr-lab/LEAN.md#quantifier-rewrites-complete-fibres-without-requiring-unique-lifts)
now proves the criterion, including universal and nonvacuous elimination.
It requires a lift for every compatible endpoint pair; the lift need not be
unique. Bind a certificate to the four maps and their admitted supports, not
just one map's handle. Multiple structural witnesses cannot be treated as
interchangeable by a probability or counting readout without its own proof.

Exact witnesses can explain a failed containment or surviving query condition.
For guarded worlds, reconstruct a feasible source assignment from the retained
guard certificate/solver instead of reporting an arbitrary impossible bit vector.
An Event expression can be exported as a circuit, compiled to dense cofactors,
packed through a relation, and returned to the same canonical identity. These
interfaces make the type composable without exposing unowned pointers or turning
the law into an opaque probability callback.
