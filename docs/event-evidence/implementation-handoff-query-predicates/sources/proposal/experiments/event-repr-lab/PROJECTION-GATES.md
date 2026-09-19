# A membership dependency can remove an entire support factor

The completed round retains 74 passing native processes, 264 configurations and
816 matched comparisons. Five shared correctness suites and 32 native
ownership/symbolic acceptance processes also pass. It emerged while testing
[conditional completion](TERNARY-COMPLETION.md).
The census confirms that 64 difficult readout answers share only 343 records,
while the staged arena retains over 1.36 million. The current general fallback
clips the Event to **all** original support, then
quantifies and completes. That can introduce dependence on a whole face which
the Event itself ignores. The factored count kernel already avoids a related
cost during observation; projection has a different witness obligation.

## The exact dependency-language condition

Write the retained variables as X and an omitted face as Y. A membership FD
`X -> membership_A` lets us represent A as a predicate of X alone. Let S(X,Y)
be legal support and D(X) its projected domain. Observations `fx` and `fy` can
keep all, some or none of their respective coordinates. For a target `(x0,y0)`,
the actual possible readout is:

```text
exists x,y: S(x,y) & A(x) & fx(x)=fx(x0) & fy(y)=fy(y0)
```

Dropping Y's support gate gives:

```text
exists x: D(x) & A(x) & fx(x)=fx(x0)
```

These agree **for every A** exactly when every admitted retained candidate has
a compatible omitted witness:

```text
forall x: D(x) & fx(x)=fx(x0)
          => exists y: S(x,y) & fy(y)=fy(y0)
```

That is a coverage obligation on a projection of a relation. It is the same
kind of exact extension information that an IND or checked support-map
certificate can supply. The membership FD and coverage fact do different jobs;
neither implies the other.

For certified product support `S(x,y)=D(x)&J(y)` at a legal output, choose
`y=y0`. This works for arbitrary partial observations of Y and arbitrary A.
No probability law, cardinality weighting or independence inference is involved.
If a decoder lands in that legal support, equality at legal outputs makes the
two completed raw predicates equal everywhere.

## The stronger observed-coordinate dependency

A whole face can occur in A without needing its support gate during this
projection. The sufficient FD is `(other faces, observed part of this face)
-> membership_A`. Changing the hidden portion cannot change membership, so use
that face's current admitted target value for its witness. A fully visible face
satisfies this automatically. Within a partly hidden face, a predicate using only
its visible coordinates satisfies it too.

The sixth Lean report, `observable_gate_elimination`, proves this form while
retaining the same observation equalities and joint existential witness. The
Rust test is sufficient and representation-derived: intersect the Event's exact
physical essential-coordinate mask with the hidden-coordinate mask. Only faces
containing a surviving bit need gates. The proof needs membership stability on
the raw candidates admitted by the relaxed gate; the raw physical FD supplies
that fact. An arbitrary supported FD alone must not be assumed to extend to
illegal raw aliases.

## Implemented controls and boundaries

`EVENT_LAB_RETRACTION_PROJECT=joint|active|witness` chooses three policies:

- `joint` retains the original full support gate and remains the default.
- `active` gates only whole faces present in the predicate's physical mask.
- `witness` gates only faces intersecting both physical dependence and hidden
  coordinates; these must intersect at the same bit, not merely the same face.

Both specialized policies require constructor-certified legal products and a
retained environment. They build the partial gate, quantify, then complete as
before. The witness policy also returns an Event directly when its physical
mask misses every hidden coordinate; this fixed-point rule is valid on arbitrary
support because the current legal output supplies a witness. The normalizer, public
Event meaning and original support used by observation remain unchanged.

An arbitrary mask intersecting dependence inside a face still needs that domain gate.
Hiding the environment requires the general fallback unless a stronger joint
coverage certificate is available. Arbitrary coupled support also retains the
general fallback. For a relational product, the relevant predicate is the joint
operand; both operands' dependence must be accounted for before omitting a face.

Existing per-environment count-support roots supply the gates. Up to eight
combined gates are memoized lazily in a fixed owner array, charged equally as
metadata in every policy; generated nodes/caches and their construction time
are also charged. A census includes these prepared gates as owner infrastructure.
Their reuse does not turn projection into counting. All policies share one
executable. Compare direct ITE with both gate policies so that benefits of
the dependency rewrite and the conditional kernel remain distinguishable.

Canonical reimport of a full support-clipped bitset still has its own issue:
its raw input explicitly mentions the discarded faces. This projection rewrite
alone does not establish a cheap generic importer.

## Formal boundary and counterexamples

[ProjectionGate.lean](lean/ProjectionGate.lean) proves the iff coverage condition,
product specialization and completion lifting, plus two counterexamples. All
six reports use no axioms.

- On copied support `x=y`, the Event `x=true` ignores Y, but when Y is observed
  false, no legal witness exists. Dropping that constraint invents one.
- In each environment e, let Y's legal domain be the singleton `{e}`. If the
  environment is hidden while Y is observed false, the Event `e=true` remains
  impossible. A per-environment product certificate does not let us drop Y
  across that hidden environment.

The proposed Rust predicate-mask check is sufficient for the membership FD,
not a complete solver for every possible logical dependency. The first kernel
would use construction-certified product support. General relational coverage
certificates remain a planner opportunity, not an unchecked optimizer assumption.

## Validation before timing

The independent partition oracle checks 49,152 product-domain operation results
and 2,304 coupled-support results per decoder/cutoff combination, across six
combinations. These totals include all three policies and both existential and
relational-product operations. Inputs include whole-face and partial-coordinate
FDs, constants, random predicates, empty environments and masks hiding the
environment. Each policy must agree pointwise and recover exactly the same
canonical roots. All previous readout, role, transport, count and law checks
remain in the shared suite.

The native comparison keeps normalization and gate policy as independent key
dimensions, preserves input node/byte/storage statistics across gate policies,
compares identical masks, and checks every output against original legal support.
Eighty-four policy pairs preserve all resident statistics on full support or
certified prefix suffix readouts. General canonical bitmap reimport remains
required and passes; the gate rewrite does not alter its implementation.

## Why this applies to the game algebra

A product workspace can contain source, destination and scratch **copies of a
whole legal game state**. The legal-state domain itself can contain every card
correlation and deck restriction. Product support between those state copies
does not assert independence between players, and it does not designate any
probability law. An Event that only needs source-state information can therefore
avoid unused destination/scratch support factors while keeping the coupled game
state intact. Internally coupled arbitrary support still uses the general path
unless a suitable coverage certificate is separately available.

## Completed native comparison

[The generated measurements](PROJECTION-GATE-MEASUREMENTS.md) include a
three-sample fibred high-X-bit repeat at width six with outer memo and direct ITE:

| Order | Joint gates ms | Active gates ms | Witness gates ms | Packed ms | Dense ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Bit-major | 392.599 | 12.402 | 10.758 | 1.467 | 1.446 |
| Face-major | 8.584 | 7.346 | 5.985 | 0.595 | 1.480 |

Every bit-major policy starts with 19,801 arena records. Final counts are
708,405, 37,611 and 35,380. Estimated query bytes fall from 83,850,636 to
4,327,844 for both specialized policies; capacity steps explain the equal byte
estimates despite different node counts. All policies return the same 343-record
answer graph, with at most 24 records per root. Joint infrastructure reaches
430 records and the combined live roots reach 1,163; specialized gates add one
infrastructure record, giving 431 and 1,164. These are reachability censuses,
not garbage collection or reclaimable-byte measurements.

The staged-normalizer control also benefits: a one-sample bit-major query falls
from 1722.688 to 18.772 and 15.502 ms. Dependency-directed gates supply the largest
observed reduction, independently of ITE. One sample supports the scale of this
change, not fine latency ranks. Packed still leads the repeated non-suffix
query. [Local completion](LOCAL-COMPLETION.md) is the next distinct hypothesis:
avoid repairing faces whose completion projection already preserves.
