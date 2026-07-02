# 20 — Query IR

## Decision: no text query language; the IR is pure data

Queries are expressed as **plain Rust data structures** — serializable, inspectable,
no behavior. The IR is the engine's public contract.

**Alternative 1 — a Logica text frontend** (the long-standing plan): investigated
deeply (repo at `~/Documents/logica`, 2026-07-02). Logica's *syntax and rule model* are
excellent; its *semantics* are SQL-shaped at the core — explicit bag semantics
("conjunction multiplies multiplicities"), null as a member of every type, negation
literally defined as an is-null test over an aggregate, 269 untyped SQL-passthrough
functions, recursion as observable depth-8 unrolling. A "strict subset" could keep the
syntax but not the semantics, meaning we'd maintain a parser, spans, error reporting,
and type inference for a dialect that only reads like Logica. **Why it lost:** there is
one user and he calls from Rust. The parser layer served nobody.

**Alternative 2 — a typed builder / generics API as the contract.** Catches type errors
at compile time (illegal queries unrepresentable in rustc). **Why it lost:** the owner's
call — a builder bakes a Rust-specific calling convention into the contract, and
translating builders to other language APIs later is painful. Data structures translate
to anything. A typed builder (or `query!` macro) may appear later as *sugar producing
the IR*; it will never be the contract.

What we keep from Logica as IR design: **named-argument atoms** (fields bound by name),
**group key = the non-aggregated output fields** (aggregation has no separate GROUP BY
to get wrong), and **rules-as-views** reborn as host-language composition: a "derived
predicate" is a Rust function that returns an IR fragment. The host language is the
rule language. The engine has no view/rule/module concept at all.

## Semantics

- The logical solution of a query is a **set of variable bindings**; projection returns
  the **set** of projected facts. Existential variables never multiply output.
- Distinctness is the default and only behavior. There is no DISTINCT concept.
- Aggregation folds over **sets of bindings** (never bags of derivations): the group key
  is exactly the non-aggregated output fields; each group folds the set of satisfying
  bindings. Empty groups do not appear (no input rows → no group → no null, ever).
- No ordering or limit in the IR: results are sets. Presentation ordering is the
  caller's concern (`OPEN` whether a results-API convenience appears).

## IR shape (normative sketch — field names may evolve, concepts may not)

```rust
Query {
    finds:      Vec<FindTerm>,        // variables and aggregate terms
    atoms:      Vec<Atom>,            // conjunctive relation atoms
    predicates: Vec<Comparison>,      // typed comparisons over vars/params/literals
}
Atom {
    relation:   RelationId,
    bindings:   Vec<(FieldId, Term)>, // named-field bindings; unbound = existential
}
Term     = Var(VarId) | Param(ParamId) | Literal(Value)   // no wildcard variant:
                                                          // absence of a binding IS the wildcard
FindTerm = Var(VarId)
         | Aggregate { op: Sum | Min | Max | Count, over: VarId }
Comparison { op: Eq|Ne|Lt|Le|Gt|Ge, lhs: Term, rhs: Term }
```

Representation notes (the branch-removal decisions):

- **No wildcard term.** An unbound field is simply absent from `bindings` — the illegal
  state "wildcard bound to something" cannot be written.
- **Repeating a variable within one atom** is legal and means same-fact equality.
- Variables carry no names in the IR, only dense ids; names are a debugging sidecar.
- Aggregates are in the IR from day one (a ledger database that cannot compute a balance
  fails its own thesis). Sum uses checked overflow → error. Min/Max over the value types'
  total order. Count counts the group's binding set, exactly.

## Validation boundary

Malformed IR is rejected at the execution boundary with typed errors: unknown
relation/field ids, variable type conflicts (including cross-serial unification —
`AccountId` never unifies with `InstrumentId`), comparisons over non-orderable types,
aggregates over unbound variables, empty `finds`. Validation happens once; everything
downstream trusts a `ValidatedQuery` witness type (parse, don't validate — the proof
travels in the type, and inner layers re-checking is a design smell we explicitly ban;
v5 validated the same plan four times).

## Prepared queries

A validated, planned query is a reusable object; executing it repeatedly is the hot
path the allocation contract (`30-execution.md`) is written against. Plan caching keys
on the IR structure; statistics changes invalidate plans, not validation.

## `OPEN` extensions (designed-for, not built)

Negation = anti-join atoms (trivial under set semantics — NOT EXISTS, no null trick
needed). Recursion = an explicit fixpoint construct, semi-naive, if a real need appears.
Both must arrive as new IR node kinds; nothing in the current IR may assume acyclicity
of future extensions.
