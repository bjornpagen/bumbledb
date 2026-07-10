# PRD 05 — Rules: the query IR becomes a program

**Depends on:** nothing (composes with 03; lands independently).
**Modules:** `crates/bumbledb/src/ir.rs`, `ir/validate/`, `ir/normalize/`,
`crates/bumbledb/src/api/prepared/build.rs` (per-rule plumbing).
**Authority:** `20-query-ir.md`.
**Representation move:** reify control flow as data, at the top of the query.
Disjunction never becomes an execution node; **a query becomes a set of
conjunctive rules sharing one head**, which is precisely a non-recursive
Datalog program. The recursion OPEN item gains its landing pad and is not
entered: rules are one step short of the fixpoint, on purpose.

## Context (decided shape)

The IR's top shape changes from one conjunct to:

- `Query { head, rules }` — `head` is the find shape (arity + positional
  types + aggregate ops); `rules` is a non-empty list of `Rule`s.
- A `Rule` is exactly today's conjunct: positive atoms, negated atoms,
  predicates, and its **own variable scope** — variables do not cross rules;
  the head aligns rules positionally (rule *i*'s find terms must match the
  head's types position by position).
- The single-rule query is the degenerate case and embeds today's IR
  unchanged — every existing query is a one-rule program.
- **Denotation:** the query denotes the set union of its rules' denotations.
  Set semantics means there is exactly one union — no bag distinction exists
  or is representable.
- Aggregates read the head: the fold domain is the union of the rules' binding
  sets projected to the head (dedup semantics owned by PRD 07; the elision
  theorem by PRD 08).
- Params are query-global (one binding surface; any rule may reference any
  param).
- Validation roster additions, each a distinct typed error: empty rule set;
  head/rule positional arity or type mismatch; `MAX_RULES` (the roster cap —
  16, documented at the definition, counted independently of the per-rule
  20-occurrence cap); per-rule rosters unchanged (negation safety, membership
  typing, caps) — a rule validates exactly as a query did.
- Dependencies are untouched: statement sides remain single-atom selections —
  the theory's language does not grow because the query language did.

## Technical direction

1. `ir.rs`: the `Query`/`Rule` split as plain data; `head` owns what `finds`
   owned plus the positional type row (computed at validation today — now
   pinned in the validated artifact so rules check against it).
2. Validation: per-rule reuse of the existing context machinery (each rule gets
   its own bivalent-anchor typing fixpoint); head unification across rules.
3. Normalization: per-rule, unchanged; the normalized artifact becomes a list.
4. `build.rs`: stats + DP + `binary2fj` + `factor` + `fj::validate` run
   **per rule** with zero changes; the prepared query holds a list of
   `ValidatedPlan`s and one sink configuration.

## Passing criteria

- `[shape]` `ir::Query` has `rules: Vec<Rule>`; no `union`/`or` node type
  exists anywhere in the IR; the word "disjunct" appears nowhere (they are
  rules).
- `[test]` One-rule queries: byte-identical validated artifacts vs the prior
  representation's tests (the existing IR test suite ports mechanically).
- `[test]` Head mismatch (arity, type, aggregate position), empty rule set,
  and `MAX_RULES+1` each produce their typed error.
- `[test]` Variable scoping: the same `VarId` in two rules refers to two
  variables (a two-rule query where the "shared" name binds different types
  validates — scopes are per-rule).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`20-query-ir.md`: the query shape section rewritten around head + rules; the
denotation sentence; the Datalog observation and the recursion OPEN
cross-reference; the roster additions.
