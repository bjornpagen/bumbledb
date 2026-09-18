# Native Event relations

The finite structural relation core is implemented on `codex/event-algebra`.
An owned `WorldRelation` is an Event plus two checked endpoint views. The Event
still supports the same Boolean operations, storage and equality as any other
Event. The relation view adds composition, quantification and residuals.

This is native host functionality. Query heads, complete descriptor persistence,
and parameterized sources remain separate implementation work.
[Diagram inspection](event-inspection.md) is now available on every Event region.
Nothing here creates a probability law, an independent draw, or a release.

## Products and roles

`FaceProduct` constructs a full product of a nonempty roster of finite faces.
Each face supplies a `SurjectiveMap` from its legal worlds to one shared finite
environment. That premise establishes a nonempty endpoint fibre at every
admitted environment value. Face widths, supports and readouts can differ.
An environment with no varying coordinates is an ordinary zero-bit `Space`.

Product support contains exactly the tuples whose endpoint worlds are legal
and whose environment readouts agree. Original supports gate all witnesses;
decoder aliases cannot create endpoint states. A coordinate tuple is a structural
assignment, not a joint probability distribution. Arbitrary Boolean environment
readouts work, including nonlinear maps.

`FibreProduct` specializes this contract to an ordered pair. It exposes owned
left/right projections and their environment maps. `pair(left, right)` combines
two readouts on one source and checks compatibility against full product support.
`certify_square` additionally requires their **joint** map to reach every compatible
pair. Its `CompleteFibreSquare` certificate is distinct from either projection's
individual surjectivity. Multiple witnesses for a pair are permitted.

The environment equality in these constructors means equality of an entire
finite value. It cannot stand in for equality of real parameters merely because
their coarse guard codes match. The source/solver integration must preserve the
actual shared parameter binding before these kernels can implement that case.

A region already on the exact pair product has only declared endpoint coordinates.
To adopt a region from a larger workspace, `WorldRelation::from_lifted` requires
an onto pair view and uses `SurjectiveMap::descend` to check the separate role law:

```text
same endpoint readout ⇒ same Event membership
pullback(image(E)) = E
```

The descended Event is unique because the pair view is onto. This is a membership
FD, not a test that a physical dependency mask excludes scratch bits. A scratch
coordinate determined by the shared environment may legitimately occur in the
encoding. Conversely, full product support alone does not justify treating an
Event that reads a third state as a binary relation.

## Algebra and prepared products

| API | Result |
| --- | --- |
| `WorldRelation::new(product, region, control)` | Checked owned relation view |
| `region()`, `product()`, `input()`, `output()` | Safe inspection and reuse of its existing Event/face descriptors |
| `complement()`, `apply(...)` | Boolean relation construction within the full legal endpoint product |
| `in_product(...)` | Checked reindexing to another presentation of the same endpoint/environment meanings |
| `included_in(...)`, `equivalent(...)` | Supported relational inclusion/equality after checking roles |
| `converse()` | Exchange endpoint views without rebuilding the Event or changing its resident key |
| `domain(...)`, `range(...)` | States with a successor/predecessor |
| `may(E, ...)`, `post(E, ...)` | Existential predecessor/successor conditions |
| `all(E, ...)`, `must(E, ...)` | Universal condition, with Must additionally requiring a successor |
| `identity(...)`, `test(...)` | Symbolic diagonal and its restriction by an Event |
| `graph(...)`, `readout(...)` | Convert a checked map to its graph and recover a map from a total functional relation |

Graph construction checks environment preservation **before** clipping to the
product. Otherwise a bad total map could silently become a partial relation.
Readout recovery requires full domain and disjoint true/false preimages for
every output bit. It rejects partial or nondeterministic relations; it never
chooses an arbitrary output witness.

`RelationalProduct::new(workspace_id, ST, TU, SU, control)` prepares one full
three-face workspace and three onto pair views. It checks the endpoint maps
agree on S, T and U, including each map into the common environment. It uses
one copy of each endpoint state, so the middle witness is physically shared.
The owned workspace, products and prepared projections are inspectable.

Its three operators are:

```text
compose(R,Q)          = Exists_T(R_ST & Q_TU)
left_residual(R,V)    = !Exists_S(R_ST & !V_SU)
right_residual(V,Q)   = !Exists_U(!V_SU & Q_TU)
```

Lifts and projections refer to that admitted workspace. Complements use the
full appropriate pair support. Results are completed in the specified result
owner. In particular, a residual constructs the largest compatible relation,
which may still exclude every continuation at some state.

```rust
use bumbledb::event::{CoordinateMap, FibreProduct, RelationalProduct,
    Space, SpaceId, WorldRelation};

let environment = Space::new(SpaceId([1; 32]), 0, &())?;
let states = Space::new(SpaceId([2; 32]), 1, &())?;
let base = CoordinateMap::new(&states, &environment, &[], &())?
    .certify_surjective(&())?;
let pairs = FibreProduct::new(SpaceId([3; 32]), &base, &base, &())?;
let plan = RelationalProduct::new(SpaceId([4; 32]), &pairs, &pairs, &pairs, &())?;

let transition = WorldRelation::new(&pairs, &pairs.space().full(), &())?;
let desired = states.coordinate(0, &())?;
let allowed_pairs = pairs.right().map().pullback(&desired, &())?;
let bound = WorldRelation::new(&pairs, &allowed_pairs, &())?;
let continuation = plan.left_residual(&transition, &bound, &())?;
assert!(plan.compose(&transition, &continuation, &())?.included_in(&bound, &())?);
```

This computes allowed behavior from a containment constraint. The corresponding
compiled public example is checked as a Rust doctest. The choice of named product
and workspace identities is explicit in the host API; a query compiler will own
those descriptors when constructing plans.

## Representation and lifetime

Original semantic coordinates concatenate each endpoint's complete code.
The default physical order interleaves coordinates across faces. `with_order`
permits another working order and resource limits without changing tuple meaning.
Products and relation workspaces currently admit at most 62 combined coordinates.
For example, three 20-bit state faces occupy a 60-bit workspace, with one middle
state. A checked test constructs and composes the diagonal without listing its
million states. This is evidence of symbolic execution, not a timing ranking.

Product construction lifts original support and readouts through the existing
substitution kernel. The initial relation plan stages lift, Boolean construction,
and exact image. It does not yet fuse these into a borrowed relational-product
kernel or connect world-coordinate factoring to Free Join. Those optimizations
must preserve the same descriptors, witnesses and result identity.

Converse swaps role descriptors and retains the same underlying Event. A reversed
product assembles readouts in the original semantic coordinate order when mapped
again. Context validation still occurs before empty/full shortcuts, including
zero-bit endpoints. All results own their context after maps, plans and database
snapshots are dropped. Cancellation/capacity failure publishes no partial result.

The Event region already persists through BEVT v1 and joins by canonical Event
identity. The role view and product/map descriptors do **not** yet have a wire
format. Persisting a region alone cannot reconstruct those roles; callers must
retain/rebuild the checked descriptor before constructing a relation view.

## Evidence and remaining gates

[Relations.lean](../crates/bumbledb-event/semantics/Relations.lean) adds 22 reports:
inhabited products, onto projections, completion gates, exact joint-fibre
certification, base change, membership-FD descent and uniqueness, complete triple
projections, shared-witness product lowering, both residuals and adjunctions,
graph admission and bitwise functionality/readout recovery. Counterexamples retain
the distinction between individual and joint coverage and prohibit dropping a
middle witness. These prove denotations/reference lowering, not the Rust builder.

Thirteen core relation tests include every two-state relation, all 4,096 R/Q/V
triples for both residual adjunctions, all 1,024 R/Q/E triples for the guarded
Must law, asymmetric legal supports, nonlinear environments, independent owners,
role-invalid third faces, zero-bit endpoints, dead ends, failed descriptors,
cancellation/capacity, converse/identity/associativity and symbolic 60-bit execution.
A database test saves a computed residual, reopens it, joins it to an independently
constructed Event on resident and cursor paths, and recovers its relation view
after the database closes using a retained checked descriptor.

The [native semantic run](event-evidence/relation-semantics/check.json) records
85 reports across five files. The [qualification](event-evidence/native-relation-qualification-final/check.json)
pins exact source hashes, logs and checks. Remaining full-proposal obligations
include source/continuous-parameter integration, complete descriptor transport,
query operator programs/stages and integrated performance. Diagram inspection
has its own later qualification, linked above.
