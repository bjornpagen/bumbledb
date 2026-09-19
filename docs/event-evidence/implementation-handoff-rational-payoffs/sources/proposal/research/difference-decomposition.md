# Difference decomposition: change the combinator, preserve the Event

This investigation asks whether a function should store two ordinary cofactors
or a base cofactor and their difference. It changes a physical decomposition,
not the Event meaning or the dependency language.

## Primary-source reading

Joan Thibault and Khalil Ghorbal, **Ordered Functional Decision Diagrams:
A Functional Semantics For Binary Decision Diagrams**,
[arXiv:2003.09340v4](https://arxiv.org/abs/2003.09340v4).
The [manifest](difference-decomposition/sources.json) retains the downloaded PDF,
extracted text, hashes, version and precise reading scope.

Definitions 1–5 express reductions as operations on functions. Theorem 3 gives
canonicality of the paper's ordered Shannon-based λDD-O-NUCX model, including
useless, canalizing and XOR variables with normalized negation. Its negation
invariance supports constant-time complement. The paper reports a Coq
formalization; we have not checked that external proof project here.

Section 5.1's conjunction still performs recursive cofactoring and normalized
reconstruction. Thus a smaller graph need not reduce every operation's work.
The qualifications after Theorem 4 explicitly exclude label storage from the
node-size comparison and explain how labels can consume the apparent savings.

Section 7.1 supplies the alternative combinator directly:

```text
Shannon: F(x,z) = if x then F1(z) else F0(z)
Davio+:  F(x,z) = F0(z) XOR (x AND D(z))
Davio−:  F(x,z) = F1(z) XOR (!x AND D(z))
D(z) = F0(z) XOR F1(z)
```

The paper proposes making the combinator a parameter to compare these choices.
Table 2 transfers the elementary reduction rules between the bases. It does
not give a runtime superiority theorem for Davio, a canonical adaptive mixture
chosen separately for every node, or a benchmark of relational contraction.
Its reference [12] names Kebschull–Schubert–Rosenstiel's original 1992 FDD work;
we have not treated that bibliography entry as an inspected primary source.

The retained Darwiche–Marquis **Knowledge Compilation Map** supplies a separate
cross-check: its introduction and §§4–5 distinguish succinctness, supported
queries and supported transformations. Its NNF tables do not rank this Davio
prototype. In particular its **FBDD** means a free binary decision diagram,
not the functional decision diagram discussed here. Its counting/probability
observation also states an independence premise; it does not supply our
correlated or shared-parameter source-law contraction.

## Derived rules worth testing

Write a positive node as `(a,d)` with meaning `a XOR x*d`. The two cofactors
uniquely determine these coefficients, and conversely. Negating the function
negates a and preserves d. With a fixed coordinate order and fixed basis per
coordinate, reduction and collision-checked interning can therefore retain a
one-bit complement and exact canonical identity.

The algebra supplies useful local operations:

```text
XOR((a,d),(b,e)) = (a XOR b, d XOR e)
AND((a,d),(b,e)) = (a AND b, (a AND e) XOR (b AND d) XOR (d AND e))
exists x.(a XOR x*d) = a OR d
forall x.(a XOR x*d) = a AND !d
```

Every binary Boolean truth function has four algebraic-normal-form coefficients,
so XOR and AND suffice for a direct coefficient Apply. Negative Davio uses the
same equations with the selector `!x`. A fixed mixed basis remains unambiguous;
changing bases after publication would require rebuilding that owner's roots.

The attractive quantifier equations eliminate the node's own selector. They
**do not** permit elimination to be pushed through arbitrary XOR coefficients.
For `F(x,y)=y XOR x`, hiding y yields full. Hiding y independently in the base
and difference gives `true XOR x`, which is not full. The prototype therefore
recovers ordinary cofactors when eliminating deeper coordinates under an
observed Davio node. Shared-witness reasoning remains necessary.

Counting also has an overlap term. For each assignment of the remaining axes,
the population across both x values is
`2*a + d − 2*(a AND d)` using 0/1 indicators. After summation, the two separate
coefficient counts are insufficient without their intersection count. A small
coefficient graph does not inherit the usual one-root Shannon counting fold.
No source-law factorization follows from the choice of Boolean basis.

## Raw coefficients are internal objects

The completed Event is still `A(rho(code))`. Its difference coefficients need
not themselves be completed Events under that decoder: fixing a physical bit
may change which legal world the decoder supplies. Keep those coefficients
inside the representation; publish the completed function with its owner.
An application-level change relation uses checked legal before/after faces and
their shared witnesses. It must not treat a physical bit derivative as a causal
intervention or an independently sampled alternative.

## Experimental discipline

The [Rust prototype](../experiments/event-repr-lab/difference-prototype/src/lib.rs)
uses one matched sixteen-byte raw node layout for Shannon, positive Davio,
negative Davio and one fixed mixed schedule. It has exact Boolean operations,
arbitrary simultaneous substitution, abstraction, and an exhaustive completed
projection oracle on coupled support. It is not yet a native Event manager with
the common wire codec, designated-law observation and Free Join integration.

Two Apply kernels compete on identical resident normal forms: direct Boolean
coefficients, and ordinary cofactors for nonlinear operations with native
coefficient XOR retained. Their constructors are identical. The structural
screen returns the same complete outputs and counts every raw-world answer;
it cannot stand in for a native timing comparison.

The first coefficient-only screen preserves a useful negative result: all eight
small relation programs and both Shannon Coup cases complete, but all six
Davio/mixed Coup cases hit the common two-million-node cap. Those are resource
caps, not wrong answers and not fast query results. The separate kernel control
tests whether the result comes from this choice of Apply algorithm rather than
the representation alone. Phase markers distinguish construction from query
growth. The control completes all six previously capped normal forms using
cofactor Apply, but with much greater storage and work than the matched Shannon
cases. A twenty-bit bilinear strength test reverses the output-size preference:
positive Davio's grouped-coordinate result has 330 reachable records versus
Shannon's 10,036, while still retaining more query workspace and Apply work.
All sixteen strength-test processes complete and verify every output assignment.
Full counts and frozen evidence appear in the
[experiment report](../experiments/event-repr-lab/DIFFERENTIAL.md).

The [counting review](compact-counting.md) settles a further substantive boundary:
sparse cubic functions have compact positive-Davio graphs, yet exact zero
counting is #P-hard. Two further inspected arXiv sources distinguish established
worst-case statements from conjectures. Seven additional Lean reports verify
the finite character-sum identities and gate equations used in our explicit
reduction; they do not formalize complexity classes or the whole compiler.

The selection remains open at the physical-carrier level. These results support
a specialized role for difference bases, not promotion over the measured
Shannon/table carriers. The subsequent [Shannon unary-chain probe](../experiments/event-repr-lab/UNARY-CHAINS.md)
includes actual inline labels and post-collection operation checks. It reduces
collected storage but leaves the one-bit kernel's retained query counts unchanged.
Node-adaptive basis selection also remains unproved; the fixed
schedule's uniqueness result does not establish it.
