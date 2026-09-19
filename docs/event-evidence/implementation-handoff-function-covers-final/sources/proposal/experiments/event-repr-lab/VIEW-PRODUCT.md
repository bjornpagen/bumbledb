# Borrow the operands; construct the relational result

This representation experiment follows [bounded scratch](SCRATCH.md).
The current relation program in [relational_core.rs](src/relational_core.rs)
constructs a canonical renamed operand, performs a fused conjunction/projection,
then constructs a canonical output renaming. The new Rust control consumes
checked operand views directly and incorporates the output map into that
traversal. It still publishes an ordinary canonical Event for the result.
The raw and shared correctness gates pass under both stores. The
[first native measurements](VIEW-MEASUREMENTS.md) retain 78 passing processes,
240 distinct query cases and 96 matched strategy pairs. They reject a blanket
replacement: the first kernel loses the larger relation cases, and face-major output fusion can
increase retained intermediates. [The next control](VIEW-ORDER.md) separates
local readout from the output-map/working-order decision.

## The mathematical object the kernel must retain

Let W be admitted joint worlds, X and Y the operand spaces, and T the retained
output space. Three total maps describe the operation:

```text
u : W -> X       read the left operand
v : W -> Y       read the right operand
h : W -> T       retain the output context

result(t) = exists w: h(w)=t and A(u(w)) and B(v(w))
```

This directly evaluates two borrowed views and projects their conjunction.
It makes no surjectivity or injectivity assumption. W must already contain the
correct admissibility constraints; raw unsupported assignments cannot witness
the existential. The exact equivalent relational contraction is

```text
J(t,x,y) = exists w: h(w)=t and u(w)=x and v(w)=y
result(t) = exists x,y: J(t,x,y) and A(x) and B(y)
```

J records which two operand worlds can occur together in each output context.
No physical J table is required: coordinate descriptors, support roots and
compiled graph structure can implement it. As in the [map review](../../research/space-maps.md),
the descriptors have ordinary relation-graph semantics. They are an executable
presentation of a relation already expressible in the algebra.

The [Lean source](lean/ViewProduct.lean) specifies the exact boundary: replacing
J by C preserves the result for **every** pair of Boolean predicates iff C and
J are equal pointwise. Singletons establish necessity. A fixed restricted
query language may need less; this theorem is not its decision procedure.
The rule concerns existential truth, not multiplicity or a probability law.

For composition, W = X × Y × Z, u reads XY, v reads YZ, and h retains XZ:

```text
(R ; Q)(x,z) = exists y: R(x,y) and Q(y,z)
```

The shared Y witness is the operation's meaning. If two views each contain both
bit values but always read the same bit, `(left=true, right=false)` is impossible.
Replacing their joint image with the product of their marginal images invents
that possibility. Complete coverage of both operands separately cannot justify it.

In Coup, compose a legal exchange transition with a later action transition.
The exchange's resulting hand and the later action's starting hand must be
the same middle state. Knowing that some hand can result from the exchange and
that some hand permits the action does not establish a legal combined path.
The shared middle coordinates keep that connection intact inside the kernel.

## The internal view is not another public Event type

An execution-local descriptor needs the following logical fields; this is not
yet a committed byte layout:

```text
BorrowedOperand {
    retained_owner,
    canonical_root_with_polarity,
    checked_coordinate_map,
    source_support,
}

ProductContext {
    admitted_joint_support,
    retained_output_map,
    destination_owner,
}
```

Several fields may be inherited once from the invocation or arena. The public
sixteen-byte scoped Event key remains the value crossing database boundaries.
No temporary pin mask, stack address or borrowed slice becomes its identity.

For the first Rust control, restrict the maps to the existing validated local
binary permutations, and the relation program to its already checked full
Cartesian support. Preserve the existing three physical coordinate orders.
This is a bounded implementation target, not a reason to weaken the general
map semantics. Cross-owner or restricted-support operations need their own
checked context before this kernel can accept them.

## Candidate traversal over essential-coordinate tables

The current readout already consumes `(root, pinned, values)` without publishing
the cofactored Event. Extend that idea to a checked map per operand:

1. Derive live target coordinates from the mapped source dependence masks,
   removing pins. A conservative superset after pins is sound, though it can
   cause unnecessary work. Include any dependence of the joint support.
2. Pick the next coordinate in the destination's canonical order. Reading a
   source node follows already pinned branch variables; new pins may remain
   pending when source and destination orders disagree.
3. At a small local cube, borrow matching table words. Otherwise cofactor,
   permute and broadcast into bounded workspace. Conjoin both operands with
   joint support, then eliminate the requested local coordinates.
4. For an eliminated branch coordinate, union the two results. For a retained
   coordinate, use the existing canonical constructor. Publish actual outputs
   through the destination owner and its support/anchor invariant.

This is not free renaming. An adverse order may force many pending pins or
repeated cofactors; output interning and normalization remain real costs.
Branch-local operations cannot pretend that the source root is automatically
the next destination variable. The materialized path stays as a same-answer
control and fallback candidate.

The existing classifier's constant-or-table stopping invariant does **not**
automatically extend to these views. Pending pins can leave a raw branch whose
remaining coordinate set fits the local cube. The prototype must either prove
that lazy reduction exposes a table, or fill a bounded residual table by exact
evaluation of the view. Treat the latter work as part of this kernel's cost;
never reinterpret a branch payload as table words.

The scratch workspace lifetime ends after its local result is consumed; it
cannot alias a borrowed arena payload across an arena mutation. A prototype
must separate the read phase from canonical output insertion, or otherwise
make the borrow discipline explicit.

## First Rust implementation (retained as a control)

[view_product.rs](src/view_product.rs) implements the traversal as a child of
the raw essential arena, using its existing canonical constructors. It retains
the source roots and a checked map for each operand. Composing each input map
with the output permutation, and moving the elimination mask through that
permutation, directly constructs the answer in its final coordinate order.
This rewrite is restricted to checked bijections of a full binary product.

At the local boundary, table words use `Cow`: an order-preserving local map can
borrow its payload even when the named coordinates change. Pinned cofactors,
actual local permutations and missing-axis broadcasts allocate temporary word
buffers in this first control. Polarity is applied when reading. A raw branch
with small residual dependence takes an explicit bounded evaluation fallback.
It is never cast to a table. The pinned-target exclusion is checked before
forming either plane.

The conjunction is projected in temporary words and then admitted through the
existing table constructor. Larger visits construct canonical output branches;
eliminated branches union their two results. Canonical partial output nodes and
operation caches can still accumulate. The claim is avoiding resident renamed
operands and the explicit local conjunction Event, not eliminating all temporary
work or all intermediate output nodes.

Immutable input borrows end before any insertion can grow arena slabs. The
scope adapter accepts this path only when its support root is full. A constrained
scope returns a missing-capability result; it is never silently replaced by a
Cartesian product. The relation-program constructor already proves full support
before enabling this control. The broader proposal still requires explicit
joint support and cross-space map capabilities.

The per-call memo holds ordered pairs of `(root,pinned,values)`. Both maps,
the output order and elimination set are fixed for that memo's entire lifetime.
The outer relation-program memo instead includes both root/map pairs, the mask
and the output map; only complete root/map pairs may be swapped. A complementary
root shortcut checks agreement of the maps on all remaining source coordinates.
There is a two-million-subproblem cap, reported as resource refusal.

The first revision selects `EVENT_LAB_PRODUCT=materialized|views`; the default
remains materialized. The next revision adds `views-inputs` and a separate
`EVENT_LAB_VIEW_KERNEL=assignments|words` control.
Both essential cutoffs and both stores use the same implementation. Other
carriers retain the materialized control. The internal capability is not a new
public scalar or a changed sixteen-byte Event key.

## First revision correctness gates

The optimized raw checks pass all eight test functions under
[enum](results/view-raw-enum-check.json) and
[slab](results/view-raw-slab-check.json) storage. They include 49,152 exhaustive
two-axis products over all operand predicates, input/output permutations,
elimination masks, two orders and K=1/2/3. Both the independent pointwise result
and the materialized operation must produce exactly the same canonical ID.

Further checks force the pending-branch fallback, distinguish identical roots
under different maps, reject invalid maps even for trivial operands, and test
64 multiword/scattered-map/complement cases across K=6/9 in a 62-coordinate
presentation. Those 64 cases require exact agreement with materialized IDs
and also check 80 independent target assignments each; they do not enumerate
all 62-coordinate assignments.

The full shared suite passes in
[enum](results/view-shared-enum-check.json) and
[slab](results/view-shared-slab-check.json) release checks. It adds 1,080 scoped
mapped-product cases per essential carrier, covering borrowed adapters, exact
imported result IDs and constrained-support rejection. The existing composition,
residual, finite closure, three-layout and sixty-coordinate symbolic relation
checks now also run with the view strategy enabled. The suite retains its
separate ownership/transport tests; native owned replay of mapped results is
an additional acceptance lane. These are
Rust implementation checks; the earlier Lean reports establish the separate
denotational contract.

```sh
python3 proposal/experiments/event-repr-lab/view_check.py --layout enum --output my-view-raw.json
python3 proposal/experiments/event-repr-lab/check.py --release --essential-kernel derived --essential-layout slab --product views --output my-view-shared.json
python3 proposal/experiments/event-repr-lab/run.py --build-only
python3 proposal/experiments/event-repr-lab/view_sweep.py --output my-views.json
python3 proposal/experiments/event-repr-lab/view_comparison.py my-views.json --output MY-VIEWS.md --record my-view-comparison.json
```

## Cache identity and legal reuse

The effective key includes both roots and their maps, pins and values, the
joint-support context, retained output mapping/elimination set, and owner/order
identities wherever these are not fixed by the enclosing invocation. Swapping
operands is legal only when it swaps their complete view descriptors together.

The same roots can produce full or empty under different maps; the Lean
counterexample makes root-only reuse untenable. A proven common substitution
may allow a more compact relative key. The variable-shift paper's Theorem 6 and
Proposition 14 require the permitted substitution and identical-vtree rule;
they do not justify dropping arbitrary maps from our keys. Canonical compression
also changes the paper's polynomial Apply guarantee. See the qualified
[representation review](../../research/representation-search.md).

Moving elimination across a different map needs the complete-fibre square
proved in [BaseChange.lean](lean/BaseChange.lean). Directly evaluating the views
above needs no such rewrite: it keeps the shared witnesses in place. This
distinction allows a broad operation with a precise, narrower optimizer rule.

## Acceptance experiment

Use the current constructive path and an independent pointwise relation oracle.
Compare the same complete relation program through actual Free Join: composition,
converse, residuals, closure, and requested result Events. Also isolate one
product to identify where work moved. Both paths must charge output normalization
and publication, including cold operation memos; a read-only signature cannot
substitute for an Event-valued result.

Retain enum/slab storage, both essential cutoffs, packed512 and dispatched-dense
controls, all three coordinate layouts, shared versus distinct mapped coordinates,
complements, and empty/full operands. Include the existing long symbolic equality
and counter fixtures with operation caps. Native performance comes only after
the shared relation suite and exact destination-ID comparisons pass.

Record fresh/warm time, input and resulting arena bytes, map/product-cache bytes,
and materialized intermediate nodes avoided or added. A view may save resident
nodes while losing time; retain that outcome. No claimed universal carrier or
automatic adaptive backend follows from this one kernel.
