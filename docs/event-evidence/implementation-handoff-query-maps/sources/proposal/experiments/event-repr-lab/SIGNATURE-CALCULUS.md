# A finite algebra of Event relationships, with an exact boundary

The [read-only classifier](CLASSIFY.md) is more than three specialized predicates.
Its four bits are the **smallest exact readout for emptiness of every binary
Boolean expression**. This is an algebraic interface available to every carrier.
It does not replace the represented regions or their richer relational operators.

[Lean now checks](LEAN.md) the signature's sufficiency/minimality for all binary
possibility tests, its operand symmetries, and the finite composition
counterexample. The composition-envelope table and saturated-count criterion
below remain separately derived and exhaustively checked, rather than formally
proved in Lean.

## Three small values with different meanings

Use separate checked types even though two values fit in a nibble:

| Value | Meaning | Example |
| --- | --- | --- |
| `BoolOp4` | Which input truth cells contribute to a constructed result | AND is `0x8` |
| `Signature4` | Which input truth cells contain a supported world | Equal proper Events have `0x9` |
| Relation mask | Which of the fifteen valid signatures a predicate accepts | Inclusion accepts signatures with bit `0x4` absent |

Cell index is `2*a+b` in both four-bit values. For a valid signature `sig`:

```text
Possible(op(A,B))  iff (sig & op) != 0
Empty(op(A,B))     iff (sig & op) == 0
Full(op(A,B))      iff (sig & (op ^ 0xf)) == 0
```

Proof: the result is the union of exactly the Venn cells selected by `op`.
A union is empty exactly when none of those cells is occupied. Conversely,
the four singleton operations each reveal one signature bit, so an abstraction
that merges different signatures cannot answer every such test exactly.

The relation mask occupies fifteen bits of a `u16`, with bit zero reserved.
There are 32,768 predicates over the valid classes. Conjunction and disjunction
of predicates become mask intersection and union; negation is relative to
`0xfffe`. Complementing either Event and swapping the operands permute the
signature, and therefore also permute predicate masks. One classified pair can
serve all these predicates without constructing their Boolean result Events.

This gives Event the useful classify-once, predicate-as-data structure of Allen.
Classification still has representation-dependent cost. A requested witness,
output Event, or law observation carries additional information and still needs
its corresponding operation. A four-bit signature cannot answer independence.

## Derive composition rather than inventing names for fifteen cases

Here composition means relationships **between Event values**. Given a
relationship of A to B and one of B to C, what can follow about A and C?
This is distinct from `R(X,Y);Q(Y,Z)`, which eliminates a state coordinate and
constructs a new Event relation in the main proposal.

Three Events partition support into eight cells with index `4*a+2*b+c`.
Enumerate their 255 nonzero occupancy patterns. Project each pattern to its
AB, BC and AC pair signatures. Then:

```text
compose[r][s] = union of (1 << t)
               over patterns with AB=r, BC=s, AC=t
```

This constructs a 15-by-15 table of masks (450 bytes without the invalid row
and column). The [checked generator](signature_calculus.py) finds 165 possible
signature triples. All have witnesses on at most five worlds. The table has
exact operand converse and an identity mask consisting of signatures 1, 8, 9
(both empty, both full, and equal proper Events). Its mask composition passes
all 3,375 associativity cases on base signatures; distributive extension gives
associativity for arbitrary masks.

For every Event scope the table is a **sound composition envelope**: each
concrete triple necessarily supplies one of the enumerated patterns. For a
full finite powerset with at least five worlds it is the smallest envelope
expressible with these signatures: a minimal witness can be enlarged by
duplicating a world. This does **not** make it exact for each fixed endpoint
pair. Smaller scopes may permit a stricter envelope.

## Why the table cannot replace the carrier

Use the same three-world support `{0,1,2}` in both examples:

```text
A = C = {0}       signature(A,C) = 9
A = C = {0,1}     signature(A,C) = 9
```

Ask whether a nonempty proper subset B of A can also be a proper subset of C.
The requested AB and BC signatures are respectively 13 and 11. It is impossible
in the first case and possible in the second (`B={0}`). Their AC signatures
are identical. Exact composition therefore cuts *inside* a signature class.
No table of unions of these fifteen classes can express that distinction.

This resolves a boundary left open in the earlier proposal. The paper by
Dylla, Mossakowski, Schneider and Wolter,
[*Algebraic Properties of Qualitative Spatio-Temporal Calculi*](https://arxiv.org/abs/1305.7345),
arXiv **1305.7345v2**, Definitions 2–3 and §2.2, distinguishes a sound abstract
composition, the smallest such envelope (weak composition), and equality to
concrete relational composition (strong composition). Its §3.2 explicitly warns
against conflating associativity with strong composition. The counterexample
above is our derivation for Event; it is not a result claimed by that paper.

On an **atomless Boolean algebra**, the table does become strong: every nonzero
AC cell can be split into two nonzero pieces whenever a triple pattern requires
both B values. Assemble B from those independently chosen pieces. This proves
realizability for each endpoint pair. Finite Coup worlds do not satisfy that
assumption. Nor does merely having an infinite underlying space imply it:
the powerset of an infinite set still contains singleton atoms.

## What extra information resolves one existential intermediate Event?

In a full finite powerset, retain the sizes of the four AC cells, capped at two:
`0`, `1`, `at least 2`. A triple pattern is realizable for those fixed endpoints
exactly when, in each cell:

1. It uses neither B value iff the cell is empty.
2. It uses one B value only when the cell has at least one world.
3. It uses both B values only when the cell has at least two worlds.

Necessity follows from disjointness. For sufficiency, put every world on the
chosen side when only one B value is needed; otherwise partition the cell into
two nonempty subsets. Take the union of the B=true pieces. Thus four saturated
counts suffice for **one** existential Event variable constrained by two exact
pair signatures. The generator checks every endpoint pair on one through five
worlds, plus all 255 nonempty vectors of four cell sizes from zero through three
(50,624 concrete assignments of B).

This is a bounded query certificate, not another proposed resident type.
Multiple intermediate Events can demand finer partitions. For k unconstrained
intermediate Boolean memberships, up to `2^k` pieces of a cell may be needed;
the cap-at-two abstraction is not asserted to remain closed under composition.
World counts also do not generalize to arbitrary guarded/definable Event
languages without proving that the required splits are expressible.

There is a concrete candidate kernel for this refinement: replace each occupancy
bit with a saturated count in `{0,1,2}`, where two means at least two. Disjoint
Shannon branches combine by `min(2, left + right)`. Support-masked leaf words
use population counts capped at two, and operand symmetry permutes the four
lanes as before. This kernel is **not implemented or benchmarked yet**.
Unlike occupancy, counting must account for skipped ambient coordinates and
their complete fibres. A free binary coordinate doubles multiplicity even when
the predicate does not mention it. A memoized count needs the appropriate
remaining-coordinate context; copying the occupancy traversal unchanged would
be wrong. Counts refer to the declared finite world presentation, never model
probability or the number of joined fact rows.

## Design consequence

Keep exact region identity and constructive operations as the carrier contract.
Use signatures as a reusable view for pair predicates, and the derived mask
table for sound constraint pruning. Refining a network to nonempty masks does
not prove that all its Event constraints are jointly realizable. Any optimization
that needs an actual intermediate Event must return to exact support reasoning
or a sufficient, checked splitting certificate. No model probabilities or row
weights enter these laws.

A checked example makes the network limit concrete: four pairwise disjoint,
nonempty proper Events cannot fit in three worlds. Nevertheless, each triangle
of these constraints is realizable, and the fifteen-class network is already
algebraically closed. The full regions retain the common world assignments
that pairwise signatures forget. Native FD/IND admission must preserve that
shared structure rather than infer global validity from a table fixpoint.

[Retained results](results/signature-calculus-check.json) include the table,
source hashes, the paper and text hashes, and the counterexample. Reproduce with:

```sh
python3 proposal/experiments/event-repr-lab/signature_calculus.py
```
