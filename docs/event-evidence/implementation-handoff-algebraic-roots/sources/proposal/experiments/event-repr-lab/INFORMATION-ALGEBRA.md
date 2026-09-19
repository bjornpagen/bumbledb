# Information is a dependency, and its answers are Events

An observation is a function `f` on admitted worlds. It can be a player's hand,
the public game history, or a tuple of fields. Worlds with the same observation
are indistinguishable under that readout. The definition does not require those
fields to occupy a contiguous range of bits.

The laboratory names Possible, Guaranteed and Ambiguous denote the proposal's
same-space May, Must and Unresolved operators. They do not propose competing
public names. Calling their result an actor's knowledge additionally requires
the scope to contain all alternatives that actor admits.

For an Event A, construct three Events in that same legal scope:

```text
Possible_f(A)(w)   = some legal v with f(v)=f(w) satisfies A
Guaranteed_f(A)(w) = every legal v with f(v)=f(w) satisfies A
Ambiguous_f(A)     = Possible_f(A) & !Guaranteed_f(A)
```

The current world is one such completion. Consequently, `Guaranteed_f(A) <= A
<= Possible_f(A)`, and the following are three disjoint Events covering the
whole scope:

```text
Guaranteed_f(A)       -- the observation forces A
!Possible_f(A)        -- the observation rules A out
Ambiguous_f(A)        -- the observation leaves both answers possible
```

These are constructive answers. A query can join them to other Events, find
counterexamples, classify their relationships, or later measure them under an
explicit source law. No probability threshold participates in the definition.

## The native dependency-language connection

Call B observable through f when agreeing on f forces agreement on B's
membership. That is exactly an FD:

```text
f(world) -> membership_B(world)
```

This is a mathematical dependency description, not a new implemented macro
production. The same condition has three interchangeable forms:

```text
B is observable through f
Possible_f(B) = B
Guaranteed_f(B) = B
```

Possible returns the least observable Event containing A. Guaranteed returns
the greatest observable Event contained in A. This uses the same containment
order that the proposal uses for values, obstructions and dependencies; it
introduces no numerical confidence lattice.

For Coup, consider the Event “Cleo holds a Duke,” projected through a Bob-role-hand
readout in the [exact deal reference](results/coup-fibre-reference.json).
That fixture fixes Alice's two physical cards first. Within that support,
fourteen Bob role hands permit Cleo Duke, none force it, and holding both
remaining Dukes rules it out. The roles are the readout, not the private
physical-copy identifiers used to enumerate the deck. This is not automatically
Bob's actual knowledge: if Alice's fixed cards are private to Alice, Bob's
epistemic scope must include the alternatives he still admits.

## An FD is the exact condition for moving a filter

For a fixed filter B, this rewrite holds for **every** operand A exactly when
f determines B's membership:

```text
Possible_f(A & B) = Possible_f(A) & B
```

The dual law has the same exact condition:

```text
Guaranteed_f(A | B) = Guaranteed_f(A) | B
```

Sufficiency comes from transporting B along a common observation. Necessity is
also useful: substitute full for A in the first rule, or empty for A in the
second, and the required fixed point becomes the membership FD. A particular
operand can satisfy the rewrite accidentally; the iff describes the uniform
optimizer rule over all operands.

For example, hiding X while retaining Y and the environment lets a target
predicate on Y pass through that readout. In the native fixture the eligibility
predicate also depends on X, so it must stay with the transition predicate
unless a stronger dependency is actually established. On arbitrary coupled
support, coordinate names alone are insufficient evidence.

Free Join supplies complete bindings. Their Event predicates conjoin using the
same world; grouping unions the resulting Events. Possibility can distribute
over that grouped union:

```text
Possible_f(union_i Term_i) = union_i Possible_f(Term_i)
```

It cannot split a term into unrelated existential witnesses. A and !A are each
possible under an observation hiding their distinction, but A & !A is empty.
Likewise, `Guaranteed_f(A | !A)` is full while neither alternative need be
guaranteed separately. Guaranteed distributes over intersections, not arbitrary
grouped unions. Returning all four native outputs `[A, Possible, Guaranteed,
Ambiguous]` still requires producing all of them; optimizing a Boolean-only
question would be a different query.

The [native workload](PREFIX-READOUTS.md) currently evaluates readouts after
grouping. These proofs justify future plan alternatives; they do not claim
that such pushdown is already implemented in the native planner.

## Information order is another FD

Let f be a finer observation than g: `f(world) -> g(world)`. Then, for every A:

```text
Possible_f(A) <= Possible_g(A)
Guaranteed_g(A) <= Guaranteed_f(A)
```

The converse for the uniform Possible inequality holds too: if every Event's
possibility only expands, the observation FD must hold. A singleton Event
supplies a witness whenever it fails. Thus the order on these readout operators
is exactly an ordinary functional dependency between observations.

Nested readouts absorb to the coarser observation, in either order, separately
for Possible and Guaranteed. This licenses eliminating redundant successive
readouts when the FD is established. It does not grant arbitrary cross-scope
quantifier movement, which still needs the [complete-fibre condition](../../research/space-maps.md).

## Proof and implementation boundaries

Fourteen new [Lean reports](lean/InformationReadout.lean) prove these claims over
arbitrary admitted-world types and arbitrary observation functions. A restricted
scope can instantiate that type with its legal-world subtype. There is no
finiteness, decoder, coordinate-order or probability premise. The entire
[central suite](results/lean-check.json) now has 121 checked reports, with no
unproved placeholders or custom axioms. The initial check rejected an unavailable
standard-library lemma; its [log](results/readout-lean-initial/lean-check.log)
is retained. The corrected proof spells out the two directions directly.

An [independent finite oracle](results/readout-laws-reference.json) exhausts all
24 partitions of zero through four worlds: 291 Event/readout cases, 4,197 binary
law cases, 291 exact filter conditions, 1,709 observable-bound cases, 256
observation-order conditions and 1,071 nested-readout cases. It also verifies
the disjoint forced/ruled-out/ambiguous partition and both grouping failures.

The primary-source basis remains direct image left-adjoint to inverse image in
[Abbadini–Guffanti, Example 2.12](https://arxiv.org/abs/2607.06386v1). Definition
2.10 and Remark 2.11 distinguish these ordinary image operations from stronger
base-change promises. The [reading record](results/readout-reading.json) identifies
the exact retained source. The FD iff and query rules above are elementary
powerset derivations checked here; the paper does not certify these Rust
representations or their performance.

The prefix decoder provides an additional physical implementation law for a
chosen hierarchy of observations. The general information algebra does not
depend on that choice. A slow fallback is an implementation problem to expose,
not a reason to remove an algebraically valid observation from Event.
