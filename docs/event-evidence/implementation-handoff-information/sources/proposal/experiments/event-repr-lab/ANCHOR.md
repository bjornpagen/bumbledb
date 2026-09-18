# An anchored representative of a complementary pair

This is a locally derived candidate, not a claim of a new mathematical discovery.
It asks whether the proposal's logical two-way partition needs two stored BDD
roots. The answer is no for scoped equality and Boolean operations. Whether it
is faster for relational operations is a separate measured question.

The representation invariant, exact support-relative identity, one-bit complement
and adjusted Boolean Apply now have [Lean proofs](LEAN.md). They prove the
denotational construction; Rust node canonicality and compiled-code refinement
remain separate implementation obligations.

## Invariant

Choose one fixed admissible assignment `a` in nonempty support `S`. For an event
`E ⊆ S`, store:

```text
polarity = E(a)
representative = E                  if polarity = 0
                 S \ E             if polarity = 1
```

Every representative is a subset of `S` and excludes `a`. Under a fixed BDD
order it therefore has one canonical root. The arena fixes the anchor; it is
never selected separately for each event. The denotation is
`E(w) = representative(w) XOR polarity` for `w ∈ S`, and false outside `S`.

```rust
struct EventKey { scope: u64, region: u64 }
// region = (canonical_bdd_root as u64) << 1 | polarity
// The BDD root already has its own complemented-edge bit.
// The two polarity bits belong to different representation layers.
```

Empty and full use representative false and polarities zero and one.
Complement is `region ^ 1`. Equality of scoped handles is exact event equality:
equal events agree at the anchor and hence select the same representative;
equal representative/polarity pairs reconstruct equal events. No probability,
approximate equivalence, or syntax identity enters that argument.

The initial experiment chooses the lowest enumerated admissible assignment.
A production arena would derive its fixed choice from the named presentation
or retain an explicit anchor descriptor. Different orders/anchors/arenas need
checked translation; their raw region IDs cannot be mixed.

## All sixteen Boolean operations stay inside the invariant

For stored `(r,p)` and `(s,q)`, and binary truth function `f`:

```text
g(x,y) = f(x XOR p, y XOR q)
new_polarity = g(0,0)
h(x,y) = g(x,y) XOR new_polarity
new_representative = h(r,s)
```

`h(0,0)=0`. Since both inputs vanish outside support and at the anchor, the
result vanishes there too. One ordinary canonical BDD Apply suffices, without
support masking or interning a pair. The new polarity is exactly the result's
value at the anchor. This includes implication, XOR, difference and all other
binary truth functions, not just AND and OR.

The experiment encodes that substitution as a permutation and optional flip
of the four truth bits. This resembles complemented-edge normalization, but
the invariant here applies to entire support-relative events.

## Quantification is where the tradeoff can reappear

Existential abstraction need not preserve falsity at the anchor. The anchor
can acquire another assignment as a witness. The prototype therefore:

1. Obtains the event's actual support-masked true root. A negatively oriented
   representative can require `S & !root` here.
2. Uses the existing exact abstraction or fused conjunction/abstraction routine.
3. Evaluates the result at the anchor and seals the new representative.

Thus a cheap Boolean path does not prove a cheap relational path. The modal
Free Join lane deliberately charges for these steps. An implementation of
signed relational product could potentially avoid materializing a true root,
but that is not what the current timings measure.

## Comparison with the other candidates

- **Masked true root:** stores the selected event directly; relative complement
  can require a support intersection.
- **Two roots:** eagerly stores both canonical sides; exposes either true root
  immediately, but pays pair interning and sometimes extra graph construction.
- **Anchored root:** stores the unique side excluding a fixed admissible world;
  Boolean operations and complement avoid the pair table, while abstraction
  can need normalization and actual-root recovery.
- **Finite complemented sets:** the other laboratory candidates choose the
  smaller side by cardinality, breaking ties at a fixed admissible world. That
  is another valid canonical orientation; the BDD candidate avoids cardinality
  work by using only the fixed anchor.

The shared oracle exercises all 61,440 four-world/support Boolean cases for
this candidate, including equality, complement and abstraction, and checks
multiword supports and native query outputs. These checks supplement the
invariant argument; they do not validate persistence, guard feasibility,
cross-arena alignment or reclamation.
