# Representation First

The governing doctrine of this codebase, recorded by the owner. Every agent —
planner, executor, verifier, sweeper — reads this document in full before any
other work. Every work packet opens with a representation verdict: the
data/type/invariant change that makes the defect's bad state inexpressible —
or an explicit essential-vs-accidental justification plus the horizon
representation. When technical direction could go either way, the
representational option wins. Prose may explain a rule; it may never be the
sole enforcement of an invariant a type, table, fold, or validator could
carry.

## Purpose

To document and ground one principle: the biggest lever in programming is the
data representation, not the control flow. When a new case shows up, you can
patch the trace of the computation with another branch, flag, or guard — and
complexity piles up in the control flow. Or you can change the data, types,
and invariants so the case stops being special, or stops being expressible at
all. Brooks, Pike, Raymond, and Torvalds have said this in almost the same
words across fifty years, and type theory explains why it works.

In scope: the practitioner lineage and its explicit chain of citation
(Brooks → Pike → Raymond → Torvalds); why precise types remove branches
(illegal states, parsing vs. validation, null, parametricity); named
techniques that remove branches (polymorphic dispatch, choosing coordinates,
sentinel nodes, reifying control flow as data); the limit — where
representation costs more than it saves, and the essential-vs-accidental
line. Out of scope: AI framing (the sources run 1975–2019), paradigm
advocacy (the principle holds in C, OCaml, TypeScript, and Lisp alike),
refactoring how-tos, performance benchmarking (indirection cost is a limit
noted, not measured).

## The three spiky points of view

**1. The data representation determines a program's complexity. The algorithm
and the control flow are downstream of it.** Code review argues about control
flow, interviews test algorithms, and "clean code" is taught as the art of
writing better conditionals. The lineage says the leverage is upstream of all
of that, in the shape of the data — and it says so in nearly identical words
from Brooks to Torvalds. What makes the claim more than one school's taste is
the convergence: four respected practitioners reached it independently across
thirty-one years, and two of them cite Brooks by name. The practical
consequence is an ordering: when complexity grows, change the representation
before you add to the control flow, because the representation is where the
complexity actually lives.

**2. Most of the branches in typical code are not handling the problem. They
are guarding against states a more precise representation would have made
impossible.** Three independent booleans — `loading`, `error`, `data` — admit
eight states, of which only a few are valid; the rest get guarded against
everywhere the value travels. A four-case sum type admits exactly four, all
valid, and the guards have nothing left to guard. Minsky's name for the move
is "make illegal states unrepresentable." Alexis King sharpens it: validation
checks a condition and throws away what it learned, so every caller
downstream must check again, while parsing returns a type that carries the
proof, so the check happens once at the boundary. Null is the proof by
counterexample — it sits in every type at once, which is exactly why it
forces a check on every use. The day-to-day implication: when you reach for a
guard, the better question is usually not "is this branch right?" but "what
representation would make this state impossible?"

**3. Most special cases belong to the representation, not the problem. Change
the representation and they are gone, not handled.** Special cases are
usually treated as inherent and met with more code. They often are not
inherent. Dijkstra's half-open interval makes length equal `b − a`, the empty
range clean, and adjacent ranges gap-free — the off-by-one is not handled, it
is unrepresentable. Homogeneous coordinates turn translation from an affine
exception into the same matrix multiply as rotation, as a matter of
arithmetic. A sentinel node makes the first and last elements stop being
special by giving the boundary a real node. None of these changed the
algorithm; they changed the coordinate system, and the special case vanished.
The ceiling of the move is to turn tangled control flow into data — an AST
with a small evaluator — with Greenspun's rule as the warning that a complex
enough program grows a bad interpreter by accident if you don't build a good
one on purpose.

## The lineage (the chain of citation)

- **Brooks, *The Mythical Man-Month* ch. 9 (1975), p. 102**, under the heading
  "Representation Is the Essence of Programming": "Show me your flowcharts and
  conceal your tables, and I shall continue to be mystified. Show me your
  tables, and I won't usually need your flowcharts; they'll be obvious." And:
  "strategic breakthrough will come from redoing the representation of your
  data or table. This is where the heart of a program lies."
- **Pike, "Notes on Programming in C" (1989), Rule 5**: "Data dominates. If
  you've chosen the right data structures and organized things well, the
  algorithms will almost always be self-evident. Data structures, not
  algorithms, are central to programming. (See Brooks p. 102.)" — the literal
  citation, the second documented link.
- **Raymond, *The Cathedral and the Bazaar* (1997), lesson 9**: "Smart data
  structures and dumb code works a lot better than the other way around,"
  followed by the attribution: "Brooks, Chapter 9… it's the same point." He
  reached it replacing fetchmail's monolithic protocol branching with a table
  of method pointers.
- **Torvalds, git mailing list (July 27, 2006)**: "Bad programmers worry about
  the code. Good programmers worry about data structures and their
  relationships." Body: "I'm a huge proponent of designing your code around
  the data, rather than the other way around… one of the reasons git has been
  fairly successful." Independent corroboration at the largest scale.

The convergence is the evidence: 1975 → 1989 (citing Brooks) → 1997 (calling
it "the same point") → 2006 (independent), across four subcultures, citations
written down. That is what separates a principle from a fashion.

## Why precise types remove branches

- **Illegal states are the hidden source of branching** (Minsky, "make
  illegal states unrepresentable," 2010/2011): put the invariants in the type
  and the compiler rejects the states the guards were defending against; the
  guards have nothing left to guard.
- **Validation discards proof; parsing keeps it** (King, "Parse, Don't
  Validate," 2019): a validator returns nothing and forces every downstream
  caller to re-check; a parser returns a refined type that carries the proof,
  so the check happens once at the boundary and never again. This is the most
  precise account of the mechanism: the information the branch tested for
  moves into the type.
- **Null is the mechanism inverted** (Hoare, "the billion-dollar mistake,"
  2009): null is effectively a member of every type, which is precisely why
  it forces a check on every dereference — the worst possible representation;
  the fix he skipped in 1965 was the disjoint union, i.e., the sum type.
- **A type signature is an enforced specification** (Wadler, "Theorems for
  Free!," 1989; Reynolds, the Abstraction Theorem, 1983): a polymorphic
  signature alone constrains behavior — `∀a. [a] → [a]` can only rearrange,
  duplicate, or drop — and well-typed clients provably cannot branch on a
  concrete representation.

## Techniques that remove branches

- **A switch on a type tag is a polymorphism not yet named** (Fowler,
  "Replace Conditional with Polymorphism"): the variation moves from
  tag-plus-branches into identity-plus-dispatch.
- **Absence and boundaries are representational choices** (Woolf's Null
  Object; CLRS's sentinel nodes): represent "nothing" or "the boundary" as a
  real object and the checks disappear wholesale — CLRS's keyed sentinel even
  removes a per-iteration loop test.
- **Off-by-one is usually a coordinate error** (Dijkstra, EWD831): the
  half-open interval `[a, b)` is the one convention where length is `b − a`,
  the empty range is clean, and adjacent ranges share a boundary — the error
  is not fixed, it is made unrepresentable.
- **Some special cases are pure coordinate artifacts** (homogeneous
  coordinates): translation is an affine exception in Cartesian coordinates
  and the same matrix multiply as rotation in homogeneous ones. The exception
  lived in the representation, provably, not the problem.
- **The ceiling is control flow as data** (SICP ch. 4: "the evaluator is just
  another program"; Greenspun's Tenth Rule as the warning): represent the
  logic as an AST or transition table and write a small evaluator —
  table-driven code, state machines, and DSLs are one family, branching
  pushed out of code into inspectable data.

## The limit (what keeps the doctrine honest)

- **Representation is globally cheap but locally expensive; control flow is
  the reverse.** A representation costs design, abstraction, indirection, and
  sometimes speed up front. A branch is free now and expensive later, through
  drift and combinatorial state. That cost structure — not virtue — is why
  adding a branch is the common reflex and investing in representation is the
  experienced one.
- **It removes accidental complexity, not essential complexity** (Brooks, "No
  Silver Bullet"). Representation collapses accidental special cases but
  cannot dissolve essential ones; force two genuinely different cases into
  one representation and the branching just hides inside config flags. The
  right representation is usually only visible after the imperative version
  exposes the pattern — part of the skill is knowing when the refactor is
  earned.

*Owner's source document verified against primary materials June 30, 2026;
recorded in-repo 2026-07-19.*
