# Logical dependencies and physical coordinates

An Event is a region of **admitted worlds**. A raw decision diagram is a
function on its physical coordinate product. Those two levels admit different
dependency claims. Keeping them explicit protects the fit with bumbledb's
dependency language while allowing more than one execution layout.

## A dependency the schema can explain

Suppose admitted worlds require `x = y`, and Event A means `x = true`.
On this support:

```text
x determines A
y determines A
the empty set does not determine A
```

The two admitted worlds are `00` and `11`. Either column tells us which world
we have; A is false in one and true in the other. Both `{x}` and `{y}` suffice,
but their intersection does not. There is no unique least sufficient coordinate
set for this Event relative to its support.

This is ordinary functional dependence: among legal rows with the same selected
coordinates, the truth value agrees. The example does not alter native pointwise
keys, infer undeclared target keys, or expose an ordinary Boolean truth column
as the public Event representation. It explains a property an internal map or
planner proof may express through relations.

## When intersection becomes valid

Let `DependsOn(S, A, D)` mean that legal worlds agreeing on D agree on A.
To combine dependencies D and E, take worlds a and b agreeing on `D ∩ E`, and
form c using a's coordinates on D and b's elsewhere. If c is legal:

```text
A(a) = A(c) because a and c agree on D
A(c) = A(b) because c and b agree on E
```

Support closure under that splice is a sufficient condition. A Cartesian
product of legal coordinate domains has it. Coupled support need not: splicing
`00` and `11` in the copied-bit example produces an illegal world.

This condition refers to the chosen **logical coordinates**. A five-role domain
can be one categorical coordinate whose legal values are all five roles. Its
three-bit encoding has only five legal codes; arbitrary splices of its individual
bits need not remain legal. A product of legal domains therefore need not be
the full cube of all encoding bits. Neither kind of structural product declares
probabilistic independence.

## Consequences for the Rust design

The essential-table carrier keeps its current invariant: its axis mask is the
least dependency set of the **canonical raw Boolean function on the full
physical cube**. The supported Event is represented above it using the anchored
construction. In the copied-bit example with anchor `00`, A's supported raw
representative is `x & y`; both raw axes are essential. This is compatible with
either one-coordinate logical view determining A on legal worlds.

Do not replace this exact mask with “the Event's unique minimal semantic
dependencies.” Such an object does not exist in general. A support-aware
alternative may choose one sufficient view using a fixed convention and retain
its reconstruction proof. That is an explicit alternative normal form, with
conversion costs and its own equality proof; it is not the current table mask.

A dependency such as x determining y can justify a smaller execution
presentation when a checked reconstruction map recovers the admitted worlds.
An Event merely depending on x is a weaker fact: it does not remove y from
other Events or from the joint law. Separately chosen sufficient views must
still retain their joint compatibility when combined. This is the role of
the existing map and joint-image contracts, rather than a second interpretation
of Event equality.

The next relation-program extension should accept a checked product of **legal
domains**, including their encoding/decoding maps, before considering arbitrary
joint support. It must carry:

- Legal input and output supports, with each mapped operand's denotation.
- The shared witness and retained output context, as in the existing
  [joint-image law](LEAN.md).
- The exact support restored after elimination and output renaming.
- The canonical anchor/polarity conversion at publication.

Symmetry under face permutations is insufficient; `x=y=z` is symmetric without
being a product. Separate surjectivity of operand maps is also insufficient.
The relation-program constructor retains its full binary-product guard.
The subsequent [scoped-product extension](SCOPED-PRODUCT.md) gives the lower-level
contraction arbitrary support by retaining its exact gates. That capability
alone does not grant the stronger relation-program interpretation.

## Evidence and proof boundary

[SupportedDependence.lean](lean/SupportedDependence.lean) states the general
splice proof for arbitrary coordinate values, product-domain closure and the
copied-bit counterexample, including nonexistence of a least dependency set.
All four reports pass the pinned Lean kernel check. The central suite also
retains the subsequent scoped-fusion proofs. The two counterexample reports use no axioms; the splice
proofs use only standard propositional extensionality. No Rust source has
changed for this extension.

These are elementary derived dependency laws. The retained
[map research](../../research/space-maps.md) supplies the inverse-image and
complete-fibre framework. The
[knowledge-compilation reading](results/reuse-reading.json) distinguishes
restriction of a raw Boolean function from forgetting a coordinate. Neither
paper is cited as proving this proposed physical representation or a performance
bound. Law transport remains a separate obligation.
