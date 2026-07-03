# 20 — Query IR

## Decision: no text query language; the IR is pure data

Queries are **plain Rust data structures** — serializable in principle (plain owned
data, no borrows, no behavior), inspectable. The
IR is the engine's public contract.

**Alternative 1 — a Logica text frontend** (the long-standing plan): investigated deeply
(2026-07-02; findings summarized here since the working repo at `~/Documents/logica` is
outside this repo). Logica's syntax and rule model are excellent; its semantics are
SQL-shaped at the core — explicit bag semantics ("conjunction multiplies
multiplicities"), null in every type, negation defined as an is-null test over an
aggregate, 269 untyped SQL-passthrough functions plus a raw-SQL escape hatch, recursion
as observable depth-8 unrolling, `Count=` compiling to *approximate* count on one
engine. A "strict subset" could keep the syntax but not the semantics. **Why it lost:**
one user, calling from Rust; the parser layer served nobody. What we keep as IR design:
named-argument atoms, group-key-from-output aggregation, and rules-as-views reborn as
**host-language composition** — a "derived predicate" is a Rust function returning an IR
fragment; the engine has no view/rule/module concept.

**Alternative 2 — typed builder/generics as the contract.** **Why it lost:** owner
ruling — builders bake a Rust calling convention into the contract and translate poorly
to other-language bindings; data structures translate to anything. Builders/macros may
appear later as sugar *producing* the IR. (Host newtypes still give compile-time nominal
safety at the app layer — see `10-data-model.md`.)

## Semantics

- The logical solution of a query is the **set of distinct bindings of all query
  variables**; projection returns the **set** of projected facts.
- **Existential variables never multiply projection output.** (Scoped to projection —
  see aggregation below.)
- Distinctness is the default and only behavior; there is no DISTINCT concept.
- No ordering or limit in the IR: results are sets; the host sorts (`60-api.md`).

### Aggregation (normative)

- **The fold domain of every aggregate is the group's set of distinct full bindings
  over all query variables.** Group key = the values of the non-aggregated find
  variables. Two postings of amount 100 to one account are two distinct bindings (their
  serial ids differ): `Sum(amount) by account` = 200.
- **The footgun, stated loudly:** joining a multiplicity-adding relation into an
  aggregate multiplies the binding set — `Posting ⋈ PostingTag` with 3 tags per posting
  triples the sum, exactly as in SQL. Don't write that query; aggregate first (in v0:
  aggregate in one query, join its result in the host) or bind nothing from the
  multiplying relation... which still adds its variables. This is inherent to
  joins+aggregation, not to set semantics.
- `Sum` accumulates in **i128** and range-checks the final value once: Sum(I64)→I64,
  Sum(U64)→U64 (accumulator u128), out-of-range = a runtime query error. Deterministic
  under any fold order — set folds have none.
- `Count` is **nullary**: |the group's binding set|, result type U64. There is no
  count-distinct-of-x in v0.
- `Min`/`Max` accept orderable types only (U64, I64); result type = input type;
  deterministic (a set has one minimum).
- **All-aggregate finds are legal** (empty group key, one global group). Over empty
  input the result is the **empty set** — not a 0 or NULL row. "The balance of an
  account with no postings is an absent row, not 0." This is a documented divergence
  from SQL's ungrouped-aggregate behavior; the oracle rule lives in `50-validation.md`.
- Aggregates over non-orderable/non-summable types, an aggregate whose variable is also
  a group key, and duplicate find terms are validation errors.

## IR shape (normative)

```rust
Query {
    finds:      Vec<FindTerm>,        // ≥1; duplicates rejected
    atoms:      Vec<Atom>,            // ≥1; conjunctive
    predicates: Vec<Comparison>,
}
Atom {
    relation:   RelationId,
    bindings:   Vec<(FieldId, Term)>, // named-field; absence of a field IS the wildcard
}
Term       = Var(VarId) | Param(ParamId) | Literal(Value)
Value      = Bool(bool) | U64(u64) | I64(i64)
           | Enum(u8)                 // declaration-order ordinal, range-checked
           | String(Box<[u8]>)        // raw UTF-8 bytes; interning is the engine's job
           | Bytes(Box<[u8]>)
FindTerm   = Var(VarId)
           | Aggregate { op: AggOp, over: Option<VarId> }  // over: None for Count
AggOp      = Sum | Min | Max | Count
Comparison { op: Eq|Ne|Lt|Le|Gt|Ge, lhs: Term, rhs: Term }
```

Representation notes (the branch-removal decisions): no wildcard variant — an unbound
field is absent from `bindings`, so "wildcard bound to something" is unwritable.
Variables carry dense ids only; names are a debugging sidecar. `Value` has exactly one
variant per data-model type — the v5 missing-Bytes hole (post-mortem §13) is closed by
construction, and there is no universal-integer variant: U64 and I64 literals are
exact-typed, out-of-range is unrepresentable rather than truncated (§35).

**Comparison rules, complete:** both sides must have the same structural type (no
cross-type comparison, ever — no U64-vs-I64, no silent coercion). `Eq`/`Ne` are legal
for all six types; `Lt/Le/Gt/Ge` only for U64/U64 and I64/I64. `Eq` between two
variables is unification and obeys identical type rules. Any comparison without a
variable side (literal-vs-literal, param-vs-literal, param-vs-param) is a validation
error, and so is a variable compared with itself — both are constant-valued: write the
query you mean.

**Params:** a param's type is inferred from its anchors — the fields it binds and the
typed terms it compares against. An *unanchored* param is unwritable by construction
(every param position is itself an anchor; param-only comparisons are already constant
comparisons) — the roster discharges that item by representation. Conflicting anchors
and non-dense param ids (a gap would be a positional slot whose supplied value is never
type-checked) are validation errors. At execution bind time the supplied values are
checked for count and structural type; String/Bytes params (and literals) resolve to
intern ids by read-only dictionary lookup **per execution** — never an insert, never an
error, and a value interned by a later write is picked up on the next execution (no
stale-resolution trap; the lookup is one LMDB get, allocation-free). **Miss semantics
are per operator:** a missed value resolves to the never-minted sentinel intern id
(u64::MAX — the mint path asserts it is never issued), so an `Eq` use matches nothing
(and may short-circuit the query to empty, the only case where that is sound) while an
`Ne` use matches every stored value — the complement, per the semantics section above.

## Normalization (owned here; runs inside validation)

The paper's formalism (§2) assumes atoms with all-distinct variables, no self-joins
(renamed apart), and selections pushed to base tables. The IR deliberately permits all
three; **normalization lowers IR form to paper form**:

1. Atom occurrences are numbered — self-joins are supported; everything downstream
   (plan validity, trie schemas) quantifies over **atom occurrences**, never relation
   names.
2. A repeated variable within one atom lowers to a per-atom same-fact equality filter;
   literal and param bindings lower to per-atom filters.
3. Same-atom var-vs-var comparisons lower to per-atom field-vs-field filters.
4. Output: distinct-variable atoms + a per-atom filter list + a **residual list** (the
   comparisons whose sides come from different atoms — exactly those; nothing
   single-atom survives to the residual list).

**Deviation (paper §2):** the paper assumes selections pre-pushed and per-atom variables
distinct; we accept the richer surface and own the lowering, because there is no
external optimizer to have done it. Execution placement of filters and residuals is
specified in `30-execution.md`. **Reverses if:** never — the paper's assumption is a
WLOG, not a design.

Degenerate shapes, ruled: a query with no atoms is invalid; an atom with zero bindings
is legal and means a nonemptiness gate on that relation (Cartesian with the rest,
well-defined under the plan formalism); every find variable must appear in some atom
(Datalog safety); variables appearing only in comparisons are invalid; binding the same
FieldId twice in one atom is invalid (use one variable twice across fields for equality,
or a filter).

## Validation boundary (the roster is exhaustive)

Malformed IR is rejected once, at the boundary, yielding a `ValidatedQuery` witness that
everything downstream trusts — no inner layer re-validates (v5 validated one plan four
times, post-mortem §38). Rejections: unknown relation/field ids; duplicate FieldId in
one atom's bindings; variable type conflicts (structural); literal-vs-field and
param-anchor type mismatches (non-UTF-8 String literals included); enum ordinal out of
range for the field's variant list (in bindings and in comparisons, each precisely
diagnosed); comparisons violating the type rules above; constant comparisons;
self-comparisons; non-dense param ids; unbound find variables; comparison-only
variables; empty finds; duplicate find terms; no atoms; aggregate input-type
violations; aggregate-over-group-key; and the planner caps (more atom occurrences than
the DP accepts, more than 128 distinct variables) — enforced here so downstream id
widths and bitset sizes are true invariants.

## Prepared queries

A `ValidatedQuery` is planned once into a `PreparedQuery` — the reusable object the
zero-allocation contract is written against (`30-execution.md`). **Plans pin the
statistics read at prepare time and are never invalidated by writes**; stale plans are
accepted at this scale, and re-preparation is explicit. (Literal values are part of the
query, hence of the prepared plan; queries meant for reuse across values use Params.)
**Decision** (replaces the earlier "statistics changes invalidate plans" sentence):
pin-at-prepare. **Alternative:** stats-driven invalidation. **Why it lost:** exact row
counts change on every commit, so any honest trigger re-plans constantly, allocating on
the hot path and gutting the plan cache; drift at ≤10⁷ facts doesn't change join order
enough to matter. **Reverses if:** the benchmark shows a stale-plan regression a
re-prepare wouldn't have.

## `OPEN` extensions (designed-for, not built)

Negation = anti-join atoms (trivial under set semantics — NOT EXISTS, no null trick).
Recursion = an explicit fixpoint construct, semi-naive, if a real need appears. Both
arrive as new IR node kinds; nothing above assumes they never come.
