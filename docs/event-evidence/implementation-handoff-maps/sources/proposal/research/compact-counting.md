# A compact Event can still have an expensive probability observation

Positive Davio exposes a real representation tradeoff. A sparse cubic Boolean
polynomial has a polynomial-size positive-Davio representation, but exactly
counting its satisfying assignments is #P-hard. This holds already for the
uniform independent law on an unrestricted Boolean cube. Arbitrary correlated
laws are not needed to obtain the obstruction.

This is a worst-case complexity statement about an unbounded representation
family, not a lower bound on every query or a timing prediction for Coup. It
means we cannot promise both this succinctness and a uniformly polynomial-time
exact count unless FP = #P. It does not prohibit efficient specialized counting,
good variable orders, or conversion when the converted graph is small.

## Inspected sources and the claims they support

Bremner, Montanaro and Shepherd, **Average-case complexity versus approximate
simulation of commuting quantum computations**,
[arXiv:1504.07999v2](https://arxiv.org/abs/1504.07999v2), defines
`gap(f) = #zeros(f) − #ones(f)`. The text explicitly separates known exact
worst-case hardness from Conjecture 3's average-case approximation claim.
Appendix B writes the character sum `sum_x (-1)^f(x)`; Appendix D Proposition 8
and its reduction setup address worst-case multiplicative approximation.
The Event argument below only needs exact counts.

Dalzell, Harrow, Koh and La Placa, **How many qubits are needed for quantum
computational supremacy?**,
[arXiv:1805.05224v3](https://arxiv.org/abs/1805.05224v3), §2.1 equations (1)–(3)
relate zero counts and gaps. Section 3.1 distinguishes counting degree-three
polynomial zeros from deciding nonbalance. Its fine-grained and average-case
conjectures are not unconditional runtime bounds. We use neither conjecture.
The [source manifest](difference-decomposition/counting-sources.json) records
the exact PDFs, extracted text, hashes and reading scope.

Those sources concern Boolean polynomials, not the Rust manager. The following
reduction and size argument make the connection to our representation explicit.

## A direct reduction using ordinary circuit gates

Use an acyclic circuit made from AND and NOT gates. For each gate introduce a
wire variable z and a quadratic constraint over GF(2):

```text
z = a AND b:  c = z XOR (a AND b) = 0
z = NOT a:    c = z XOR 1 XOR a = 0
output true:  c = 1 XOR output = 0
```

For any assignment of the input wires, the gate constraints force exactly one
assignment of the auxiliary wires. Including the output constraint retains
exactly the circuit's accepting inputs. This is the usual parsimonious gate
encoding, not a claim that auxiliary wires are independent beliefs.

Let x collect the N input and auxiliary wire variables, and let there be M
constraints. Introduce fresh Boolean multiplier variables `y_1 … y_M` and form

```text
P(x,y) = XOR_i (y_i AND c_i(x)).
```

P has degree at most three. With g gates it has at most `3g + 2` monomial
occurrences before cancelling duplicates. Repeated Boolean factors satisfy
`a*a=a`. The output constraint ensures `M >= 1`.

For a fixed x that satisfies every constraint, P is zero for every y. For a
fixed x that violates any constraint, toggling the corresponding multiplier
pairs every zero with one one. Therefore

```text
sum_(x,y) (-1)^P(x,y) = 2^M * #accepted_inputs
2 * #zeros(P) = 2^(N+M) + 2^M * #accepted_inputs.
```

An exact zero count thus recovers the original circuit count using integer
arithmetic. This proves #P-hardness under a polynomial-time counting reduction;
it needs no average-case assumption. The signed gap itself should not be
confused with a nonnegative #P function. Zero counting is the #P problem here.

## Why the polynomial has a small positive-Davio graph

Represent its ANF as a set of square-free monomials with coefficient one. At the
next variable v, partition the set into terms without v and terms with v; remove
v from the latter. Those two smaller sets are precisely the base and difference
of a positive-Davio node. No expansion into truth-table assignments is needed.

With t monomials and n coordinates this term trie has at most `n*t` internal
records: each nonempty recursive branch can be charged to one of the terms
passing through it at that level. Omitting zero differences, normalizing
complements and sharing identical subgraphs cannot increase that bound.
Partitioning and interning take polynomial work. The prototype's
`Arena::from_anf` implements this construction directly, including duplicate
cancellation. The mathematical size argument is for arbitrary finite n; the
current Rust probe still has its explicit 62-coordinate limit.

A canonical positive-Davio graph can therefore stay small on the reduction's
cubic functions. If every such graph admitted a polynomial-time exact count,
the circuit counting problem above would too. Merely changing the counter's
machine instructions cannot establish such a general guarantee.

Shannon's uniform count sums *disjoint cofactors*, with powers of two for skipped
coordinates. That is a linear graph fold in arithmetic operations on the
compiled graph. It pays for difficult cases in compilation/graph size. Davio's
two edges overlap algebraically, so its count needs information beyond two
child counts. This comparison grants neither representation cheap arbitrary
source-law contraction.

## What is formalized and implemented

[ConstraintCounting.lean](../experiments/event-repr-lab/lean/ConstraintCounting.lean)
checks character orthogonality for every finite number of multipliers, the
constraint gap identity, the doubled zero-count identity, and each local gate
equation. All seven reports pass the pinned Lean kernel without `sorryAx` or
project-specific axioms. Four use Lean's standard `propext` and `Quot.sound`,
and three are axiom-free.

The file does **not** formalize complexity classes, the whole circuit compiler's
unique-extension theorem, the ANF trie size bound, or Rust arena refinement.
Those are mathematical arguments above, with a finite end-to-end check in the
[counting experiment](../experiments/event-repr-lab/COUNTING.md).

The added exact raw counter recovers ordinary cofactors and memoizes counts.
It is useful for verification and exposes intermediate graph growth; it is
not asserted to be the best Davio counting algorithm. Its input measure gives
every raw code equal mass. It must never be used as the count of decoded legal
worlds when the decoder gives those worlds unequal numbers of aliases.

## Representation consequence

Keep event meaning independent of its observation schedule. A compact resident
function may justify a specialized carrier, but admitting that carrier requires
accounting for projection, composition, transport and designated-law observation
as well as Boolean Apply. The parity screen's smaller outputs are real; its
greater retained query work is real too. Neither establishes a native Free Join
winner.

The subsequent [Shannon chain experiment](../experiments/event-repr-lab/UNARY-CHAINS.md)
implements bounded inline canalizing/XOR labels while preserving ordinary
cofactor semantics. It charges labels and post-collection reconstruction.
Collected storage improves, but the one-bit query kernel rebuilds the same
retained intermediate counts. A block-consuming kernel is now the concrete
remaining test; no native performance victory is established.
